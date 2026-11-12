// One batch table row: the file's current name against the one the ticked automations would give it, changed runs marked.
import { el } from "../../dom";
import { checkbox } from "../../atoms/checkbox";
import { changes, isTicked, setTicked } from "../../../hooks/automations/batch";
import { diffRuns } from "../../../services/text-diff";
import type { Run } from "../../../services/text-diff";
import type { PlannedFile } from "../../../services/ipc/types";

export interface BatchRow {
  file: PlannedFile;
  tr: HTMLTableRowElement;
  box: HTMLInputElement;
}

const runNodes = (runs: Run[], mark: "del" | "ins"): (Node | string)[] => runs.map((r) => (r.changed ? el(mark, {}, r.text) : r.text));

export function setRowTicked(row: BatchRow, on: boolean): void {
  row.box.checked = on;
  setTicked(row.file, on);
  row.tr.classList.toggle("skipped", !on);
}

// A grouped row lives inside its project folder, so its new name starts from there.
export function batchRow(file: PlannedFile, grouped = false): BatchRow {
  const changed = changes(file);
  const runs = changed ? diffRuns(file.file, file.name) : null;
  const box = checkbox({ "aria-label": file.file });
  box.disabled = !changed;
  const tr = el(
    "tr",
    { class: changed ? "changed" : "unchanged" },
    el("td", { class: "batch-check" }, box),
    el("th", { scope: "row", class: "batch-before" }, ...(runs ? runNodes(runs.before, "del") : [file.file])),
    el("td", { class: "batch-after" }, ...(grouped ? [el("span", { class: "batch-home" }, "~/")] : []), ...(runs ? runNodes(runs.after, "ins") : changed ? [file.name] : [])),
  );
  const row = { file, tr, box };
  tr.classList.toggle("skipped", changed && !isTicked(file));
  box.checked = isTicked(file);
  box.addEventListener("change", () => setRowTicked(row, box.checked));
  return row;
}
