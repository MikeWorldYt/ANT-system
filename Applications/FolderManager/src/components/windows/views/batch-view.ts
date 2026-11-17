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
import { batchTiles, forgetTiles } from "./batch-tiles";
import { installMarquee } from "../../../hooks/marquee";
import { currentSplit, focusPane, paneLocation, paneOn } from "../../../hooks/split";
import { inDownloads } from "../../../services/ipc/types";
import type { BatchRow } from "./batch-row";

// In Grid the group's files are one row of tiles; in List one row each.
function groupBody(project: string, kind: "wkb" | "folder" | "lnk", files: BatchRow[], collapsed: Set<string>, onToggle: () => void, fresh: boolean, content: HTMLTableRowElement[]): HTMLTableSectionElement {
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
    el("th", { scope: "rowgroup", colspan: "2" }, el("span", { class: "batch-group-head" }, kindIcon(kind), ...(fresh ? [el("span", { class: "batch-new", title: "Does not exist yet; the run creates it" }, guiIcon("sparkle"))] : []), el("span", { class: "batch-group-name" }, `${kind === "lnk" ? "→ /" : ""}${project}/${fresh ? " (new)" : ""} (${plural(files.length, "file")})`), toggle)),
  );
  header.addEventListener("dblclick", (ev) => {
    if (ev.target instanceof Element && !ev.target.closest("input, .batch-group-toggle")) flip();
  });
  for (const tr of content) tr.hidden = shut;
  const gap = el("tr", { class: "batch-gap", "aria-hidden": "true" }, el("td", { colspan: "3" }));
  const entry = files.find((r) => r.file.entry !== null)?.file.entry;
  return el("tbody", { class: shut ? "batch-box shut" : "batch-box", ...(entry ? { "data-entry": entry, "data-home": project } : {}) }, header, ...content, gap);
}

// The other pane shows the batch too while it stands in the batch folder, so a split opens on both; browsing away frees it.
function inBatchFolder(onRight: boolean): boolean {
  if (!batch) return false;
  const { key, virtual } = paneLocation(onRight);
  const folder = keyOf(batch.folder);
  if (key === null) return virtual === "downloads" && inDownloads(batch.folder) && batch.folder.length === 1;
  return key === folder || key.startsWith(`${folder}/`);
}

// One view per pane: `right` says which pane it stands in for.
export function createBatchView(host: HTMLElement, right: boolean): { element: HTMLElement; render(): void } {
  const activeView = (): string => paneOn(right)?.views.current().id ?? "list";
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
  const currentHead = el("th", { scope: "col" }, "Current");
  const afterHead = el("th", { scope: "col" }, "After");
  const head = el("thead", {}, el("tr", {}, el("th", { scope: "col", class: "batch-check" }, all), currentHead, afterHead));
  const table = el("table", { class: "batch-table batch-head" }, head);
  const scroll = el("div", { class: "batch-scroll" }, table);
  // In Grid a press on empty space draws a band that selects the tiles it touches, as the views do.
  installMarquee(scroll);
  const element = el("section", { class: "batch", "aria-label": "Batch mode" }, scroll);
  element.hidden = true;
  // A press in the batch focuses its pane, so a selection made here never lands in the other pane.
  element.addEventListener("pointerdown", () => focusPane(right), { capture: true });
  // Group headers stick right under the column headers, whose height follows the font and is unknown while hidden.
  new ResizeObserver(() => scroll.style.setProperty("--batch-head-h", `${Math.floor(head.getBoundingClientRect().height) - 1}px`)).observe(head);
  // Which groups are folded survives the re-renders a new dry run brings.
  const collapsed = new Set<string>();

  function render(): void {
    host.hidden = batch === null && state.virtual === "board";
    // The batch stands in for its own pane, and for the other one while that one stays in the batch folder.
    const pane = paneOn(right);
    const own = batch !== null && (batch.right && currentSplit() !== null) === right;
    const open = pane !== null && batch !== null && (own || inBatchFolder(right));
    element.hidden = !open;
    pane?.host.classList.toggle("batch-covered", open);
    if (pane && open) {
      if (element.previousElementSibling !== pane.host) pane.host.after(element);
      element.style.flex = pane.host.style.flex;
    }
    if (!batch || !open) return;
    const loose: BatchRow[] = [];
    const groups = new Map<string, BatchRow[]>();
    const kinds = new Map(batch.rows.map((f) => [f.folder ?? f.project, f.target !== null ? "lnk" : f.folder !== null ? "folder" : "wkb"] as const));
    const fresh = new Set(batch.rows.filter((f) => f.create).map((f) => f.folder ?? f.project));
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
    // Grid shows only the names files will get, so the Current column goes and the After one needs no title.
    const grid = activeView() === "grid";
    forgetTiles();
    currentHead.hidden = afterHead.hidden = grid;
    const content = (files: BatchRow[]): HTMLTableRowElement[] => (grid ? [batchTiles(files, batch?.folder ?? [])] : files.map((r) => r.tr));
    // Folders already in the batch folder first, then the ones the run creates there, then moves to nodes and to nodes it creates;
    // within each, track order, so 300. comes before 400. and 400. before 400./01.
    const rank = (home: string): number => {
      const rows = batch?.rows.filter((f) => (f.folder ?? f.project) === home) ?? [];
      const out = rows.some((f) => f.target !== null);
      const made = out ? fresh.has(home) : !batch?.dirs.has(home.toLowerCase());
      return (out ? 2 : 0) + (made ? 1 : 0);
    };
    const ordered = [...groups].sort(([a], [b]) => rank(a) - rank(b) || a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }));
    const sections = [...ordered.map(([project, files]) => groupBody(project, kinds.get(project) ?? "wkb", files.sort(byName), collapsed, render, fresh.has(project), content(files))), ...(loose.length > 0 ? [el("tbody", { class: "batch-loose", "data-home": "" }, ...content(loose))] : [])];
    for (const section of sections) section.querySelector(".batch-group input")?.addEventListener("change", syncAll);
    // One table per section, because a sticky group header only lets go at the end of its own table.
    scroll.replaceChildren(table, ...sections.map((section) => el("table", { class: "batch-table" }, section)));
    syncAll();
  }

  return { element, render };
}
