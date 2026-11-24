// A graph files box keeps the size the user dragged and its own view, List, Cards or Grid, across every redraw of the graph.
import { el } from "../../dom";
import { fileThumb } from "../../atoms/thumb";
import { closingItem, menuItem, separator, submenu } from "../../molecules/menu";
import { openMenuWith } from "../../organisms/menus/context-menu";
import { sortSubmenu } from "../../organisms/menus/board-menu";
import { pasteFiles } from "../../../hooks/ops";
import { keyOf, notify, select, selectKeys, sortOf, state } from "../../../services/store/state";
import { sortedForColumns } from "../../../services/store/sort";
import { inSystem, type TreeNode } from "../../../services/ipc/types";
import { fileKind } from "./details-cells";

export type BoxView = "list" | "cards" | "grid";
const VIEWS: [BoxView, string][] = [
  ["list", "List"],
  ["cards", "Cards"],
  ["grid", "Grid"],
];

const sized = new Map<string, { width: string; height: string }>();
const views = new Map<string, BoxView>();

// A box opens in the app's start view when it fits one; any other falls back to List.
export const boxView = (key: string): BoxView => views.get(key) ?? (VIEWS.some(([v]) => v === state.settings.startView) ? (state.settings.startView as BoxView) : "list");

// The grip resizes natively, with no pointer event on the box, so the size is read whenever it changes.
// The default height stays short, so a box spends the room beside it instead: as many tiles, cards or list columns as fit a wide box.
const ROW_PX = 22;
const ROWS = 14;
const COLUMN_PX: Record<BoxView, number> = { list: 228, cards: 188, grid: 94 };
const MAX_WIDTH = 760;

export function fitWidth(box: HTMLElement, view: BoxView, count: number): void {
  const across = view === "list" ? Math.ceil(count / ROWS) : count;
  box.style.setProperty("--auto-w", `${Math.min(MAX_WIDTH, Math.max(1, across) * COLUMN_PX[view] + 28)}px`);
  box.style.setProperty("--rows", String(Math.max(1, Math.min(count, ROWS))));
}

export function keepSize(box: HTMLElement, key: string): void {
  const was = sized.get(key);
  if (was) Object.assign(box.style, was);
  new ResizeObserver(() => {
    if (!box.isConnected || !(box.style.width || box.style.height)) return;
    sized.set(key, { width: box.style.width, height: box.style.height });
    // A list fills a box made taller by hand down to its new floor before it starts another column.
    if (box.style.height) box.style.setProperty("--rows", String(Math.max(1, Math.floor((box.clientHeight - 12) / ROW_PX))));
  }).observe(box);
  edgeResize(box);
}

// The native grip is only the corner, so the edges drag too: top and bottom change the height, the right edge the width where the box may widen.
const EDGE = 5;
type Edge = "top" | "bottom" | "right" | null;

function edgeAt(box: HTMLElement, x: number, y: number): Edge {
  const r = box.getBoundingClientRect();
  if (x > r.right - 16 && y > r.bottom - 16) return null;
  if (y - r.top < EDGE) return "top";
  if (r.bottom - y < EDGE) return "bottom";
  return r.right - x < EDGE && !box.closest(".graph-under") ? "right" : null;
}

function edgeResize(box: HTMLElement): void {
  box.addEventListener("pointermove", (ev) => {
    if (ev.buttons !== 0) return;
    const edge = edgeAt(box, ev.clientX, ev.clientY);
    box.style.cursor = edge === "right" ? "ew-resize" : edge ? "ns-resize" : "";
  });
  box.addEventListener("pointerdown", (ev) => {
    const edge = ev.button === 0 ? edgeAt(box, ev.clientX, ev.clientY) : null;
    if (!edge) return;
    // The band and the move drag listen for the same press; the edge owns it.
    ev.preventDefault();
    ev.stopImmediatePropagation();
    const start = { x: ev.clientX, y: ev.clientY, w: box.offsetWidth, h: box.offsetHeight };
    box.setPointerCapture(ev.pointerId);
    const move = (m: PointerEvent): void => {
      if (edge === "right") box.style.width = `${Math.max(160, start.w + m.clientX - start.x)}px`;
      else box.style.height = `${Math.max(60, start.h + (edge === "bottom" ? m.clientY - start.y : start.y - m.clientY))}px`;
    };
    const up = (): void => {
      box.removeEventListener("pointermove", move);
      box.removeEventListener("pointerup", up);
      box.removeEventListener("pointercancel", up);
    };
    box.addEventListener("pointermove", move);
    box.addEventListener("pointerup", up);
    box.addEventListener("pointercancel", up);
  });
}

// Grid shows the picture itself when there is one; Cards adds the kind beside the name.
export function fileArt(c: TreeNode, view: BoxView, icon: Element): Element {
  return view === "grid" ? fileThumb(c, () => icon, { draggable: "false" }) : icon;
}

export const fileDetail = (c: TreeNode, view: BoxView): HTMLElement[] => (view === "cards" ? [el("span", { class: "graph-file-kind" }, fileKind(c))] : []);

// The box lists its folder's files in that folder's own sort, the one its Sort by menu sets.
export const sortedFiles = (n: TreeNode): TreeNode[] => sortedForColumns(n.children ?? [], sortOf(keyOf(n.relParts)), state.settings.pinFavorites);

// The empty space of a box has the folder's own menu: paste into it, pick the box's view, sort it, select every file.
export function boxMenu(box: HTMLElement, n: TreeNode): void {
  const key = keyOf(n.relParts);
  // The box stands for its folder, so a click or right-click on its empty space puts the path there, as its node box does.
  const pick = (): void => {
    if (!inSystem(n.relParts) && state.selectedKey !== key) select(n.relParts);
  };
  box.addEventListener("click", (ev) => {
    if (box.dataset.banded) {
      delete box.dataset.banded;
      return;
    }
    if (!(ev.target instanceof Element && ev.target.closest(".graph-file"))) pick();
  });
  box.addEventListener("contextmenu", (ev) => {
    if (ev.target instanceof Element && ev.target.closest(".graph-file")) return;
    ev.preventDefault();
    ev.stopPropagation();
    pick();
    const now = boxView(key);
    openMenuWith(
      (close) => {
        const action = closingItem(close);
        const paste = action("Paste", !n.reserved && !n.ghost && !inSystem(n.relParts), () => void pasteFiles(n.relParts));
        const choices = VIEWS.map(([v, label]) =>
          menuItem(label, "menuitemradio", true, () => {
            close();
            views.set(key, v);
            notify();
          }, "ctx-item", v === now),
        );
        const view = submenu("View in box", choices, true);
        const sort = sortSubmenu(n, close);
        const files = (n.children ?? []).filter((c) => c.file).map((c) => keyOf(c.relParts));
        const all = action("Select All", files.length > 0, () => selectKeys(files));
        return { items: [paste, view.trigger, sort.trigger, all], subs: [view, sort], content: [paste, separator(), view.element, sort.element, separator(), all] };
      },
      `Actions for ${n.name}`,
      ev.clientX,
      ev.clientY,
      () => box.focus(),
    );
  });
}
