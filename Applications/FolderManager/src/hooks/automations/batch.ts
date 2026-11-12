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

// `auto` keeps the ticked automations running on every file that lands later.
export async function runBatch(picks: Picks, auto: boolean): Promise<void> {
  if (!batch) return;
  const { review, folder, rows } = batch;
  try {
    const picked = rows.filter(isTicked);
    const changed = picked.length > 0 ? await api.applyAutomations(folder, picked) : 0;
    const kinds = Object.keys(picks) as (keyof Picks)[];
    if (auto || kinds.some((k) => review.flags.includes(`auto-${k}`))) {
      for (const kind of kinds) await setReviewAutomation(review, kind, auto && picks[kind]);
    }
    batch = null;
    await refresh();
    setStatus("info", changed > 0 ? `Automations changed ${plural(changed, "file")}.` : "No file changed.");
  } catch (e) {
    reportError("error", e);
    batch = null;
    await refresh();
  }
}
