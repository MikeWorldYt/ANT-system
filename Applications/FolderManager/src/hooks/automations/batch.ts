// Batch mode: the dry run of a review folder's automations takes the place of the views until it is run or left.
import * as api from "../../services/ipc/api";
import { refresh, setReviewAutomation } from "../ops";
import { plural } from "../../services/plural";
import { findNode, notify, select, state } from "../../services/store/state";
import { openVirtual } from "../virtual-nav";
import { currentSplit } from "../split";
import { loadPath } from "../../services/store/virtual";
import { readPref, savePref } from "../../services/store/prefs";
import { inDownloads } from "../../services/ipc/types";
import { DOWNLOADS } from "./scope/review-scope";
import { snapshotView, startReplay } from "./batch-replay";
import { reportError, setStatus } from "../../components/organisms/bars/status-line";
import type { Picks, PlannedFile, TreeNode } from "../../services/ipc/types";
import type { ReviewScope } from "./scope/review-scope";

interface Batch {
  review: TreeNode;
  folder: string[];
  only: string[];
  rows: PlannedFile[];
  picks: Picks;
  // Files left unticked, kept by name so a new dry run does not tick them again.
  skipped: Set<string>;
  // Subfolders the batch folder already has, so the view tells a folder the run fills from one it creates.
  dirs: Set<string>;
  // The split pane the batch took, so the other pane stays free to browse and drag from.
  right: boolean;
  // The folder's files as listed, by name, for the dates and sizes the Details view shows.
  files: Map<string, TreeNode>;
  // The view the batch replaced when it opened, and whether it now waits in another tab rather than being closed.
  viewBefore?: string;
  parked?: boolean;
}

export let batch: Batch | null = null;

// Files keep landing while the dry run is open, from a download or another app, so the folder is read every few seconds and replanned when its files change.
const WATCH_MS = 2000;
let watch = 0;
let seen = "";

const fileList = async (folder: string[]): Promise<string> =>
  (await api.listChildren(folder)).filter((c) => c.file).map((c) => c.name).sort().join("\n");

function stopWatch(): void {
  window.clearInterval(watch);
  watch = 0;
}

function startWatch(folder: string[]): void {
  stopWatch();
  void fileList(folder).then((list) => (seen = list), (e: unknown) => console.warn("Batch watch could not read the folder", e));
  watch = window.setInterval(() => {
    if (!batch || state.busy) return;
    void fileList(batch.folder).then(
      async (list) => {
        if (list === seen || !batch) return;
        seen = list;
        await replanBatch(batch.picks);
      },
      (e: unknown) => console.warn("Batch watch could not read the folder", e),
    );
  }, WATCH_MS);
}

// An open dry run is kept across restarts: it changes nothing until it runs, so reopening it only saves the user its setup.
const SAVED = "fm.batch";

interface SavedBatch {
  root: string;
  review: string[];
  folder: string[];
  only: string[];
  picks: Picks;
  skipped: string[];
}

function remember(): void {
  const saved: SavedBatch | null = batch && state.tree ? { root: state.tree.root, review: batch.review.relParts, folder: batch.folder, only: batch.only, picks: batch.picks, skipped: [...batch.skipped] } : null;
  savePref(SAVED, saved ? JSON.stringify(saved) : "");
}

// The views the batch can be drawn in; the others fall back to List.
export const BATCH_VIEWS = ["list", "grid", "cards", "details", "columns"];

export const changes = (row: PlannedFile): boolean => row.name !== row.file || row.project !== null || row.folder !== null;

// A file the sort is unsure of starts unticked.
export const isTicked = (row: PlannedFile): boolean => changes(row) && !batch?.skipped.has(row.file);

export function setTicked(row: PlannedFile, on: boolean): void {
  if (!batch) return;
  if (on) batch.skipped.delete(row.file);
  else batch.skipped.add(row.file);
  remember();
}

async function load(picks: Picks): Promise<void> {
  if (!batch) return;
  batch.picks = picks;
  const before = new Set(batch.rows.map((r) => r.file));
  const open = batch;
  const [rows, children] = await Promise.all([api.planAutomations(open.folder, open.only, picks), api.listChildren(open.folder)]);
  // A run or a close may have ended this batch while its dry run was being read.
  if (batch !== open) return;
  batch.rows = rows;
  batch.dirs = new Set(children.filter((c) => !c.file).map((c) => c.name.toLowerCase()));
  batch.files = new Map(children.filter((c) => c.file).map((c) => [c.name, c]));
  for (const row of batch.rows) if (row.project !== null && !row.sure && !before.has(row.file)) batch.skipped.add(row.file);
  remember();
  notify();
}

// A restored run brings back the files it was narrowed to and the ones left unticked, and fails quietly when its folder is gone.
export async function openBatch(scope: ReviewScope, picks: Picks, restored?: { only: string[]; skipped: string[] }): Promise<void> {
  const folder = scope.folders[0] ?? scope.files[0]?.relParts.slice(0, -1);
  if (!scope.review || !folder) return;
  batch = { review: scope.review, folder, only: restored?.only ?? scope.files.map((f) => f.name), rows: [], picks, skipped: new Set(restored?.skipped), dirs: new Set(), right: currentSplit()?.right ?? false, files: new Map() };
  try {
    await load(picks);
    startWatch(folder);
  } catch (e) {
    batch = null;
    remember();
    if (restored) console.warn("The saved batch could not reopen", e);
    else reportError("error", e);
    notify();
  }
}

// A batch belongs to the tab it was opened in: leaving the tab parks it, coming back brings it back replanned.
export type ParkedBatch = Batch;

export function parkBatch(): ParkedBatch | null {
  const parked = batch;
  if (!parked) return null;
  stopWatch();
  parked.parked = true;
  batch = null;
  return parked;
}

export function unparkBatch(parked: ParkedBatch | null): void {
  if (!parked || batch) return;
  parked.parked = false;
  batch = parked;
  startWatch(parked.folder);
  void replanBatch(parked.picks);
}

export async function restoreBatch(): Promise<boolean> {
  const raw = readPref(SAVED);
  if (!raw || !state.tree) return false;
  let saved: SavedBatch;
  try {
    saved = JSON.parse(raw) as SavedBatch;
  } catch (e) {
    console.warn("The saved batch is unreadable", e);
    savePref(SAVED, "");
    return false;
  }
  if (saved.root !== state.tree.root) return false;
  if (!inDownloads(saved.review)) await loadPath(saved.review).catch((e: unknown) => console.warn("The saved batch folder could not load", e));
  const review = inDownloads(saved.review) ? DOWNLOADS : findNode(saved.review);
  if (!review) {
    savePref(SAVED, "");
    return false;
  }
  // The panel closes a batch whose folder is not the one selected, so the selection goes back there first.
  // Opening Downloads already selects nothing; a select(null) after it would leave the virtual list.
  if (inDownloads(saved.review)) await openVirtual("downloads");
  else select(saved.folder);
  await openBatch({ folders: [saved.folder], files: [], label: "", review }, saved.picks, { only: saved.only, skipped: saved.skipped });
  return batch !== null;
}

export async function replanBatch(picks: Picks): Promise<void> {
  try {
    await load(picks);
  } catch (e) {
    reportError("error", e);
  }
}

// An edited dictionary.md regroups the open dry run with the automations already ticked.
export function replanAfterDictionary(): void {
  if (batch) void replanBatch(batch.picks);
}

export function closeBatch(): void {
  stopWatch();
  batch = null;
  remember();
  notify();
}

// Panel order, which is also the order they apply in; each kind's flag makes it run on every file that lands in the folder.
export const AUTOMATIONS: { kind: keyof Picks; label: string; flag: string }[] = [
  { kind: "reduce", label: "Clean up names", flag: "auto-reduce" },
  { kind: "addDate", label: "Add date", flag: "auto-add-date" },
  { kind: "removeDate", label: "Remove date", flag: "auto-remove-date" },
  { kind: "sort", label: "Sort by projects", flag: "auto-sort" },
  { kind: "group", label: "Group by dictionary", flag: "auto-group" },
  { kind: "removeSymbols", label: "Remove symbols", flag: "auto-remove-symbols" },
];

export const autoKinds = (node: TreeNode): (keyof Picks)[] => AUTOMATIONS.filter((a) => node.flags.includes(a.flag)).map((a) => a.kind);

export const labelsOf = (kinds: (keyof Picks)[]): string => AUTOMATIONS.filter((a) => kinds.includes(a.kind)).map((a) => a.label).join(", ");

// Writes only the flags that differ, so switching one automation does not touch the others.
export async function setAuto(review: TreeNode, kinds: (keyof Picks)[]): Promise<void> {
  for (const { kind, flag } of AUTOMATIONS) {
    const on = kinds.includes(kind);
    if (review.flags.includes(flag) !== on) await setReviewAutomation(review, kind, on);
  }
}

export async function runBatch(): Promise<void> {
  if (!batch) return;
  const { folder, rows, dirs, right } = batch;
  const shot = snapshotView(right);
  stopWatch();
  savePref(SAVED, "");
  try {
    const picked = rows.filter(isTicked);
    const { changed, chained } = picked.length > 0 ? await api.applyAutomations(folder, picked) : { changed: 0, chained: [] };
    batch = null;
    await refresh();
    if (inDownloads(folder)) startReplay(shot, folder, picked, dirs, right, chained);
    setStatus("info", changed > 0 ? `Automations changed ${plural(changed, "file")}.` : "No file changed.");
  } catch (e) {
    reportError("error", e);
    batch = null;
    await refresh();
  }
}
