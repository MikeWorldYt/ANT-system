// A graph files box keeps the size the user dragged and its own view, List, Cards or Grid, across every redraw of the graph.
import { el } from "../../dom";
import { closingItem, menuItem, separator, submenu } from "../../molecules/menu";
import { openMenuWith } from "../../organisms/menus/context-menu";
import { sortSubmenu } from "../../organisms/menus/board-menu";
import { pasteFiles } from "../../../hooks/ops";
import { keyOf, notify, selectKeys, sortOf, state } from "../../../services/store/state";
import { sortedForColumns } from "../../../services/store/sort";
import { nativeImage } from "../../../services/file-images";
import { inDownloads, type TreeNode } from "../../../services/ipc/types";
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
export function keepSize(box: HTMLElement, key: string): void {
  const was = sized.get(key);
  if (was) Object.assign(box.style, was);
  new ResizeObserver(() => {
    if (box.isConnected && (box.style.width || box.style.height)) sized.set(key, { width: box.style.width, height: box.style.height });
  }).observe(box);
}

// Grid shows the picture itself when there is one; Cards adds the kind beside the name.
export function fileArt(c: TreeNode, view: BoxView, icon: Element): Element {
  const src = view === "grid" ? nativeImage(c, true) : null;
  return src ? el("img", { class: "thumb", src, alt: "", draggable: "false" }) : icon;
}

export const fileDetail = (c: TreeNode, view: BoxView): HTMLElement[] => (view === "cards" ? [el("span", { class: "graph-file-kind" }, fileKind(c))] : []);

// The box lists its folder's files in that folder's own sort, the one its Sort by menu sets.
export const sortedFiles = (n: TreeNode): TreeNode[] => sortedForColumns(n.children ?? [], sortOf(keyOf(n.relParts)), state.settings.pinFavorites);

// The empty space of a box has the folder's own menu: paste into it, pick the box's view, sort it, select every file.
export function boxMenu(box: HTMLElement, n: TreeNode): void {
  const key = keyOf(n.relParts);
  box.addEventListener("contextmenu", (ev) => {
    if (ev.target instanceof Element && ev.target.closest(".graph-file")) return;
    ev.preventDefault();
    ev.stopPropagation();
    const now = boxView(key);
    openMenuWith(
      (close) => {
        const action = closingItem(close);
        const paste = action("Paste", !n.reserved && !n.ghost && !inDownloads(n.relParts), () => void pasteFiles(n.relParts));
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
