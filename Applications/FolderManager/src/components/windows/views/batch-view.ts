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

function groupBody(project: string, kind: "wkb" | "folder" | "lnk", files: BatchRow[], collapsed: Set<string>, onToggle: () => void): HTMLTableSectionElement {
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
    el("th", { scope: "rowgroup", colspan: "2" }, el("span", { class: "batch-group-head" }, kindIcon(kind), el("span", { class: "batch-group-name" }, `${kind === "lnk" ? "→ /" : ""}${project}/ (${plural(files.length, "file")})`), toggle)),
  );
  header.addEventListener("dblclick", (ev) => {
    if (ev.target instanceof Element && !ev.target.closest("input, .batch-group-toggle")) flip();
  });
  for (const r of files) r.tr.hidden = shut;
  const gap = el("tr", { class: "batch-gap", "aria-hidden": "true" }, el("td", { colspan: "3" }));
  return el("tbody", { class: shut ? "batch-box shut" : "batch-box" }, header, ...files.map((r) => r.tr), gap);
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
  const table = el("table", { class: "batch-table batch-head" }, head);
  const scroll = el("div", { class: "batch-scroll" }, table);
  const element = el("section", { class: "batch", "aria-label": "Batch mode" }, scroll);
  element.hidden = true;
  // Group headers stick right under the column headers, whose height follows the font and is unknown while hidden.
  new ResizeObserver(() => scroll.style.setProperty("--batch-head-h", `${Math.floor(head.getBoundingClientRect().height) - 1}px`)).observe(head);
  // Which groups are folded survives the re-renders a new dry run brings.
  const collapsed = new Set<string>();

  function render(): void {
    const open = batch !== null;
    element.hidden = !open;
    host.hidden = open || state.virtual === "board";
    if (!batch) return;
    const loose: BatchRow[] = [];
    const groups = new Map<string, BatchRow[]>();
    const kinds = new Map(batch.rows.map((f) => [f.folder ?? f.project, f.target !== null ? "lnk" : f.folder !== null ? "folder" : "wkb"] as const));
    rows = [];
    for (const file of batch.rows) {
      const home = file.folder ?? file.project;
      const row = batchRow(file, home !== null);
      rows.push(row);
      row.box.addEventListener("change", syncAll);
      const parts = [...batch.folder, file.file];
      row.tr.classList.toggle("selected", state.selectedKey === keyOf(parts));
      // A click beside the checkbox shows the file's details, as selecting it in a view does.
      row.tr.addEventListener("click", (ev) => {
        if (!(ev.target instanceof HTMLInputElement)) select(parts);
      });
      if (home === null) loose.push(row);
      else groups.set(home, [...(groups.get(home) ?? []), row]);
    }
    // Inside a group the rows follow their new names, so files that end under the same topic sit together.
    const byName = (a: BatchRow, b: BatchRow): number => a.file.name.localeCompare(b.file.name, undefined, { numeric: true, sensitivity: "base" });
    const sections = [...[...groups].map(([project, files]) => groupBody(project, kinds.get(project) ?? "wkb", files.sort(byName), collapsed, render)), ...(loose.length > 0 ? [el("tbody", {}, ...loose.map((r) => r.tr))] : [])];
    for (const section of sections) section.querySelector(".batch-group input")?.addEventListener("change", syncAll);
    // One table per section, because a sticky group header only lets go at the end of its own table.
    scroll.replaceChildren(table, ...sections.map((section) => el("table", { class: "batch-table" }, section)));
    syncAll();
  }

  return { element, render };
}
