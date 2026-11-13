// Batch mode: the dry run of a review folder's automations takes the place of the views until it is run or left.
import * as api from "../../services/ipc/api";
import { refresh, setReviewAutomation } from "../ops";
import { plural } from "../../services/plural";
import { notify } from "../../services/store/state";
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
}

export let batch: Batch | null = null;

export const changes = (row: PlannedFile): boolean => row.name !== row.file || row.project !== null || row.folder !== null;

// A file the sort is unsure of starts unticked.
export const isTicked = (row: PlannedFile): boolean => changes(row) && !batch?.skipped.has(row.file);

export function setTicked(row: PlannedFile, on: boolean): void {
  if (!batch) return;
  if (on) batch.skipped.delete(row.file);
  else batch.skipped.add(row.file);
}

async function load(picks: Picks): Promise<void> {
  if (!batch) return;
  batch.picks = picks;
  const before = new Set(batch.rows.map((r) => r.file));
  batch.rows = await api.planAutomations(batch.folder, batch.only, picks);
  for (const row of batch.rows) if (row.project !== null && !row.sure && !before.has(row.file)) batch.skipped.add(row.file);
  notify();
}

export async function openBatch(scope: ReviewScope, picks: Picks): Promise<void> {
  const folder = scope.folders[0] ?? scope.files[0]?.relParts.slice(0, -1);
  if (!scope.review || !folder) return;
  batch = { review: scope.review, folder, only: scope.files.map((f) => f.name), rows: [], picks, skipped: new Set() };
  try {
    await load(picks);
  } catch (e) {
    batch = null;
    reportError("error", e);
    notify();
  }
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
  batch = null;
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
  const { folder, rows } = batch;
  try {
    const picked = rows.filter(isTicked);
    const changed = picked.length > 0 ? await api.applyAutomations(folder, picked) : 0;
    batch = null;
    await refresh();
    setStatus("info", changed > 0 ? `Automations changed ${plural(changed, "file")}.` : "No file changed.");
  } catch (e) {
    reportError("error", e);
    batch = null;
    await refresh();
  }
}
