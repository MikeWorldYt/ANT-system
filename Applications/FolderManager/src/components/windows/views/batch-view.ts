// Batch mode list: each file of the review folder as it reads now and as the ticked automations would leave it, like a diff.
import { el } from "../../dom";
import { checkbox } from "../../atoms/checkbox";
import { guiIcon } from "../../atoms/gui-icons";
import { kindIcon } from "../../atoms/icons";
import { nodeToggle } from "../../atoms/node-toggle";
import { batch } from "../../../hooks/automations/batch";
import { plural } from "../../../services/plural";
import { keyOf, select, state } from "../../../services/store/state";
import { batchRow, setRowTicked } from "./batch-row";
import type { BatchRow } from "./batch-row";

function groupBody(project: string, files: BatchRow[], collapsed: Set<string>, onToggle: () => void): HTMLTableSectionElement {
  const box = checkbox({ "aria-label": `All files moving to ${project}` });
  const shut = collapsed.has(project);
  const toggle = nodeToggle(`${shut ? "Show" : "Hide"} the files of ${project}`, !shut, guiIcon(shut ? "chevronRight" : "chevronDown"));
  toggle.classList.add("batch-group-toggle");
  // Unlike in the views, no row holds the keyboard here, so the toggle is its own tab stop.
  toggle.tabIndex = 0;
  const flip = (): void => {
    if (shut) collapsed.delete(project);
    else collapsed.add(project);
    onToggle();
  };
  toggle.addEventListener("click", flip);
  toggle.addEventListener("keydown", (ev) => {
    if (ev.key !== "Enter" && ev.key !== " ") return;
    ev.preventDefault();
    flip();
  });
  const sync = (): void => {
    const on = files.filter((r) => r.box.checked).length;
    box.checked = on === files.length;
    box.indeterminate = on > 0 && on < files.length;
  };
  box.addEventListener("change", () => {
    for (const r of files) setRowTicked(r, box.checked);
    sync();
  });
  for (const r of files) r.box.addEventListener("change", sync);
  sync();
  const header = el(
    "tr",
    { class: "batch-group" },
    el("td", { class: "batch-check" }, box),
    el("td", {}),
    el("th", { scope: "rowgroup" }, el("span", { class: "batch-group-head" }, kindIcon("wkb"), el("span", { class: "batch-group-name" }, `${project}/ (${plural(files.length, "file")})`), toggle)),
  );
  for (const r of files) r.tr.hidden = shut;
  return el("tbody", {}, header, ...files.map((r) => r.tr));
}

export function createBatchView(host: HTMLElement): { element: HTMLElement; render(): void } {
  const all = checkbox({ "aria-label": "Tick every file that changes" });
  let rows: BatchRow[] = [];
  const syncAll = (): void => {
    const live = rows.filter((r) => !r.box.disabled);
    const on = live.filter((r) => r.box.checked).length;
    all.checked = live.length > 0 && on === live.length;
    all.indeterminate = on > 0 && on < live.length;
  };
  all.addEventListener("change", () => {
    for (const r of rows) if (!r.box.disabled) setRowTicked(r, all.checked);
    render();
  });
  const head = el("thead", {}, el("tr", {}, el("th", { scope: "col", class: "batch-check" }, all), el("th", { scope: "col" }, "Current"), el("th", { scope: "col" }, "After")));
  const table = el("table", { class: "batch-table" }, head);
  const element = el("section", { class: "batch", "aria-label": "Batch mode" }, el("div", { class: "batch-scroll" }, table));
  element.hidden = true;
  // Which groups are folded survives the re-renders a new dry run brings.
  const collapsed = new Set<string>();

  function render(): void {
    const open = batch !== null;
    element.hidden = !open;
    host.hidden = open || state.virtual === "board";
    if (!batch) return;
    const loose: BatchRow[] = [];
    const groups = new Map<string, BatchRow[]>();
    rows = [];
    for (const file of batch.rows) {
      const row = batchRow(file, file.project !== null);
      rows.push(row);
      row.box.addEventListener("change", syncAll);
      const parts = [...batch.folder, file.file];
      row.tr.classList.toggle("selected", state.selectedKey === keyOf(parts));
      // A click beside the checkbox shows the file's details, as selecting it in a view does.
      row.tr.addEventListener("click", (ev) => {
        if (!(ev.target instanceof HTMLInputElement)) select(parts);
      });
      if (file.project === null) loose.push(row);
      else groups.set(file.project, [...(groups.get(file.project) ?? []), row]);
    }
    const sections = [...(loose.length > 0 ? [el("tbody", {}, ...loose.map((r) => r.tr))] : []), ...[...groups].map(([project, files]) => groupBody(project, files, collapsed, render))];
    for (const section of sections) section.querySelector(".batch-group input")?.addEventListener("change", syncAll);
    table.replaceChildren(head, ...sections);
    syncAll();
  }

  return { element, render };
}
