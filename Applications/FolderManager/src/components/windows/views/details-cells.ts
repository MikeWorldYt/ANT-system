// The Details view's columns beside each name, under a header that sorts, resizes and picks them as Explorer's does.
import { el } from "../../dom";
import { notify, setColumnSort, sortOf } from "../../../services/store/state";
import { fileTypeName, ownerOf, statusLabel, typeName } from "../../../services/store/node-info";
import { COLUMNS, isShown, MIN_WIDTH, onColumnsChange, setWidths, shownColumns, toggleColumn, widthOf, type ColumnDef, type ColumnId } from "../../../services/store/details-columns";
import { closingItem, menuItem, separator } from "../../molecules/menu";
import { openMenuWith } from "../../organisms/menus/context-menu";
import type { TreeNode } from "../../../services/ipc/types";

// A folder shows its storage type, as the side panel does; one with no type is a plain folder.
export const fileKind = (n: TreeNode): string => (n.file ? fileTypeName(n.name) : n.nodeType ? typeName(n) : "Folder");

const stamp = (ms: number | null): string => (ms === null ? "" : new Date(ms).toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" }));

// Rounded up to whole KB, as Explorer shows it, so a tiny file never reads as 0.
const size = (bytes: number | null): string => (bytes === null ? "" : `${Math.max(1, Math.ceil(bytes / 1024)).toLocaleString()} KB`);

const VALUES: Record<Exclude<ColumnId, "name">, (n: TreeNode) => string> = {
  modified: (n) => stamp(n.stamps.modified),
  type: fileKind,
  size: (n) => size(n.stamps.size),
  created: (n) => stamp(n.stamps.created),
  status: (n) => (n.status ? statusLabel(n.status) : ""),
  owner: (n) => (n.file ? "" : (ownerOf(n) ?? "")),
};

const cellClass = (id: ColumnId) => `detail-cell detail-${id}`;

export const detailCells = (n: TreeNode): HTMLElement[] =>
  shownColumns().flatMap((c) => (c.id === "name" ? [] : [el("span", { class: cellClass(c.id) }, VALUES[c.id](n))]));

// A root entry is no folder on disk, so it only fills the Type column.
export const entryCells = (type: string): HTMLElement[] =>
  shownColumns().flatMap((c) => (c.id === "name" ? [] : [el("span", { class: cellClass(c.id) }, c.id === "type" ? type : "")]));

// One column template shared by the header and every row, which line up on it as the columns of a sheet.
export const detailTemplate = (folder: string, over?: { id: ColumnId; width: number }): string =>
  shownColumns().map((c) => `${over?.id === c.id ? over.width : widthOf(folder, c.id)}px`).join(" ");

// The batch lays its rows out in tables, so it sizes each cell from its own variable instead.
const BATCH = "::batch";
function applyWidths(over?: { id: ColumnId; width: number }): void {
  const root = document.documentElement.style;
  for (const c of COLUMNS) root.setProperty(`--col-${c.id}`, `${over?.id === c.id ? over.width : widthOf(BATCH, c.id)}px`);
}
applyWidths();
onColumnsChange(() => {
  applyWidths();
  notify();
});

// The widest content a column holds right now, as Explorer measures for "Size column to fit"; a range reads text past a cut box.
function contentWidth(from: Node, to: Node | null): number {
  const r = document.createRange();
  r.setStartBefore(from);
  if (to) r.setEndBefore(to);
  else if (from.parentNode) r.setEnd(from.parentNode, from.parentNode.childNodes.length);
  return r.getBoundingClientRect().width;
}

function fitWidth(host: Element, id: ColumnId): number {
  const rows = [...host.querySelectorAll<HTMLElement>(id === "name" ? ".grid-item .detail-lead" : `.grid-item .detail-${id}, .batch-tile .detail-${id}`)];
  const head = host.querySelector<HTMLElement>(`.details-head .detail-${id}`);
  // The name box stretches over its whole column, so a name is measured from the row's start to the end of its text.
  const nameEnd = (c: HTMLElement): number => {
    const text = c.querySelector(".name-text");
    if (!text) return 0;
    const r = document.createRange();
    r.selectNodeContents(text);
    return r.getBoundingClientRect().right - c.getBoundingClientRect().left;
  };
  const widths = rows.map((c) => (id === "name" ? nameEnd(c) : c.firstChild ? contentWidth(c.firstChild, null) : 0));
  if (head?.firstChild) widths.push(contentWidth(head.firstChild, head.querySelector(".detail-grip")));
  return Math.ceil(Math.max(MIN_WIDTH, ...widths)) + 6;
}

function columnMenu(ev: MouseEvent, host: Element, folder: string, at: ColumnDef | null): void {
  ev.preventDefault();
  openMenuWith(
    (close) => {
      const item = closingItem(close);
      const fit = item("Size column to fit", at !== null, () => at && setWidths(folder, { [at.id]: fitWidth(host, at.id) }));
      const fitAll = item("Size all columns to fit", true, () => setWidths(folder, Object.fromEntries(shownColumns().map((c) => [c.id, fitWidth(host, c.id)]))));
      const toggles = COLUMNS.map((c) => menuItem(c.label, "menuitemcheckbox", c.id !== "name", () => {
        close();
        toggleColumn(c.id);
      }, "ctx-item ctx-check", isShown(c.id)));
      const items = [fit, fitAll, ...toggles];
      return { items, subs: [], content: [el("div", { class: "ctx-group", role: "group" }, fit, fitAll, separator(), ...toggles)] };
    },
    "Details columns",
    ev.clientX,
    ev.clientY,
    () => undefined,
  );
}

// Dragging the right edge sizes the column; a double click there fits it to its contents.
function resizer(c: ColumnDef, cell: HTMLElement, folder: string): HTMLElement {
  const grip = el("span", { class: "detail-grip", "aria-hidden": "true" });
  grip.addEventListener("click", (ev) => ev.stopPropagation());
  grip.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    ev.preventDefault();
    ev.stopPropagation();
    grip.setPointerCapture(ev.pointerId);
    const left = cell.getBoundingClientRect().left;
    let next = cell.getBoundingClientRect().width;
    const move = (e: PointerEvent): void => {
      next = Math.max(MIN_WIDTH, e.clientX - left);
      const over = { id: c.id, width: next };
      if (folder === BATCH) applyWidths(over);
      else cell.closest<HTMLElement>(".grid")?.style.setProperty("--detail-cols", detailTemplate(folder, over));
    };
    grip.addEventListener("pointermove", move);
    grip.addEventListener("lostpointercapture", () => {
      grip.removeEventListener("pointermove", move);
      setWidths(folder, { [c.id]: next });
    }, { once: true });
  });
  grip.addEventListener("dblclick", (ev) => {
    ev.stopPropagation();
    const host = cell.closest(".grid, .batch-scroll");
    if (host) setWidths(folder, { [c.id]: fitWidth(host, c.id) });
  });
  return grip;
}

function header(folder: string, cells: (c: ColumnDef) => HTMLElement): HTMLElement {
  const head = el("div", { class: "details-head", role: "presentation" }, ...shownColumns().map((c) => {
    const cell = cells(c);
    cell.append(resizer(c, cell, folder));
    cell.addEventListener("contextmenu", (ev) => {
      ev.stopPropagation();
      columnMenu(ev, head.closest(".grid, .batch-scroll") ?? head, folder, c);
    });
    return cell;
  }));
  head.addEventListener("contextmenu", (ev) => {
    ev.stopPropagation();
    columnMenu(ev, head.closest(".grid, .batch-scroll") ?? head, folder, null);
  });
  return head;
}

// The batch lists a dry run, not a folder, so its header names the columns without sorting by them.
export const plainDetailsHeader = (): HTMLElement => header(BATCH, (c) => el("span", { class: cellClass(c.id) }, c.label));

export function detailsHeader(columnKey: string): HTMLElement {
  const { by, dir } = sortOf(columnKey);
  return header(columnKey, (c) => {
    if (!c.sort) return el("span", { class: cellClass(c.id) }, c.label);
    const key = c.sort;
    const on = by === key;
    const button = el("button", { type: "button", class: `${cellClass(c.id)} detail-sort`, "aria-pressed": String(on) }, c.label, on ? (dir === "asc" ? " ↑" : " ↓") : "");
    // A second click on the sorted column flips its direction, as in Explorer.
    button.addEventListener("click", () => setColumnSort(columnKey, on ? { dir: dir === "asc" ? "desc" : "asc" } : { by: key, dir: "asc" }));
    return button;
  });
}
