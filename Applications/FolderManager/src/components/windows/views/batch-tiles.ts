// Batch mode in Grid: each file as the grid shows it, with its real icon and one name that mixes what goes (red) and what comes (green).
import { el } from "../../dom";
import { nameIcon } from "../../atoms/icons";
import { changes } from "../../../hooks/automations/batch";
import { nativeImage } from "../../../services/file-images";
import { diffRuns } from "../../../services/text-diff";
import { keyOf, select, selectKeys, state } from "../../../services/store/state";
import type { TreeNode } from "../../../services/ipc/types";
import type { BatchRow } from "./batch-row";
import { tileMenu } from "./batch-tile-menu";
import { groupAt, moveOut, pinTo } from "../../../hooks/automations/batch-drop";
import { targetAt } from "../../../hooks/folder-move";

// Every tile on screen by its key, so the menu reaches the rows the selection holds.
const shown = new Map<string, { parts: string[]; row: BatchRow }>();

// Both panes may draw the batch, so a redraw only drops leftover ghosts; a later tile of the same file replaces the entry.
export function forgetTiles(): void {
  for (const ghost of document.querySelectorAll(".batch-ghost")) ghost.remove();
}

function tile(row: BatchRow, folder: string[]): HTMLElement {
  const file = row.file;
  const parts = [...folder, file.file];
  const key = keyOf(parts);
  shown.set(key, { parts, row });
  // The thumbnail is read from where the file is now; only its path and name matter to the image cache.
  const src = nativeImage({ relParts: parts, name: file.file } as unknown as TreeNode, true);
  const art = el("span", { class: "grid-art" }, src ? el("img", { class: "thumb", src, alt: "", draggable: "false" }) : nameIcon(file.file));
  const after = changes(file) ? diffRuns(file.file, file.name).mixed.map((r) => (r.mark === "same" ? r.text : el(r.mark, {}, r.text))) : [file.name];
  const item = el("div", { class: "grid-item batch-tile", title: file.name, "data-key": key }, row.box, art, el("span", { class: "batch-tile-name" }, ...after));
  item.classList.toggle("selected", state.selectedKeys.includes(key));
  const sync = (): void => void item.classList.toggle("skipped", !row.box.checked && !row.box.disabled);
  row.box.addEventListener("change", sync);
  sync();
  // The box ticks the file for the run; the rest of the tile selects it, so the panel shows its details and preview.
  dragToGroup(item, key, file.folder ?? file.project);
  item.addEventListener("click", (ev) => {
    if (ev.target === row.box || item.dataset.dragged) {
      delete item.dataset.dragged;
      return;
    }
    if (ev.ctrlKey) selectKeys(state.selectedKeys.includes(key) ? state.selectedKeys.filter((k) => k !== key) : [...state.selectedKeys, key]);
    else select(parts);
  });
  item.addEventListener("contextmenu", (ev) => {
    if (!state.selectedKeys.includes(key)) select(parts);
    const picked = state.selectedKeys.flatMap((k) => shown.get(k) ?? []);
    tileMenu(ev, picked, () => item.focus());
  });
  return item;
}

// The webview hands native drags to the system, so a tile is dragged with pointer events; letting go on another group pins the file there,
// and on a folder outside the batch moves the file itself there, as it is, leaving the batch. A tile in the selection carries the selection.
function dragToGroup(item: HTMLElement, key: string, home: string | null): void {
  let start: { x: number; y: number } | null = null;
  let ghost: HTMLElement | null = null;
  let over: HTMLElement | null = null;
  let folder: string[] | null = null;
  const hover = (next: HTMLElement | null): void => {
    if (next === over) return;
    over?.classList.remove("drop-target");
    next?.classList.add("drop-target");
    over = next;
  };
  const picked = (): { parts: string[]; row: BatchRow }[] => (state.selectedKeys.includes(key) ? state.selectedKeys : [key]).flatMap((k) => shown.get(k) ?? []);
  const files = (): string[] => picked().map((p) => p.row.file.file);
  // The browser's own drag (of the thumbnail or a text selection) would swallow the pointer events, so it never starts here.
  item.addEventListener("dragstart", (ev) => ev.preventDefault());
  item.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0 || ev.target instanceof HTMLInputElement) return;
    ev.preventDefault();
    start = { x: ev.clientX, y: ev.clientY };
  });
  item.addEventListener("pointermove", (ev) => {
    if (!start) return;
    if (!ghost) {
      if (Math.hypot(ev.clientX - start.x, ev.clientY - start.y) < 6) return;
      const count = files().length;
      ghost = el("div", { class: "batch-ghost", "aria-hidden": "true" }, count > 1 ? `${count} files` : (shown.get(key)?.row.file.file ?? ""));
      document.body.append(ghost);
      item.setPointerCapture(ev.pointerId);
      item.classList.add("dragging");
    }
    Object.assign(ghost.style, { left: `${ev.clientX + 12}px`, top: `${ev.clientY + 12}px` });
    const under = document.elementFromPoint(ev.clientX, ev.clientY);
    const group = groupAt(under);
    const place = !group && under && !under.closest(".batch") ? targetAt({ x: ev.clientX, y: ev.clientY }) : null;
    folder = place?.parts ?? null;
    hover(place?.el ?? (group?.dataset.home === (home ?? "") ? null : group));
  });
  const end = (drop: boolean): void => {
    const target = over;
    const into = folder;
    folder = null;
    const moved = ghost !== null;
    ghost?.remove();
    ghost = null;
    start = null;
    hover(null);
    item.classList.remove("dragging");
    if (!moved) return;
    item.dataset.dragged = "1";
    if (drop && into) void moveOut(picked().map((p) => p.parts), into);
    else if (drop && target) void pinTo(target.dataset.entry ?? null, files());
  };
  item.addEventListener("pointerup", () => end(true));
  item.addEventListener("pointercancel", () => end(false));
  // A redraw mid-drag drops the tile and its capture, so the ghost must not stay behind.
  item.addEventListener("lostpointercapture", () => end(false));
}

export function batchTiles(rows: BatchRow[], folder: string[]): HTMLTableRowElement {
  return el("tr", { class: "batch-tiles" }, el("td", { colspan: "3" }, el("div", { class: "grid batch-grid" }, ...rows.map((r) => tile(r, folder)))));
}
