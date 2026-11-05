// A search hit can be dragged straight onto a folder or a board column, so a found item is placed without browsing to it.
import { el } from "../components/dom";
import { folderIcon, nameIcon } from "../components/atoms/icons";
import { cancelOnEscape, moveGhost, pastThreshold, pointOf, released, type Point } from "./pointer";
import { canMove, HIGHLIGHT, moveDragged, targetAt } from "./folder-move";
import { moveCard } from "./board";
import { BOARD_COLUMNS } from "../services/store/board";
import { findNode, keyOf, reveal } from "../services/store/state";
import { isWorkboard } from "../services/store/node-info";
import type { BoardColumn, TreeNode } from "../services/ipc/types";

type Target = { kind: "folder"; el: HTMLElement; parts: string[] } | { kind: "board"; el: HTMLElement; column: BoardColumn; index: number };

interface Drag {
  parts: string[];
  start: Point;
  ghost: HTMLElement | null;
  target: Target | null;
}

export const hitIcon = (parts: string[]): HTMLElement => {
  const node = findNode(parts);
  return node ? folderIcon(node) : nameIcon(parts[parts.length - 1] ?? "");
};

function ghostFor(parts: string[]): HTMLElement {
  const icon = hitIcon(parts);
  icon.classList.add("drag-ghost-icon");
  const ghost = el("div", { class: "drag-ghost-item", "aria-hidden": "true" }, icon, el("span", { class: "drag-ghost-mode" }, "→ Move"));
  document.body.append(ghost);
  return ghost;
}

function boardTarget({ x, y }: Point): Target | null {
  const col = document.elementFromPoint(x, y)?.closest<HTMLElement>(".board-col");
  const column = BOARD_COLUMNS.find(([id]) => id === col?.dataset.column)?.[0];
  if (!col || !column) return null;
  const above = [...col.querySelectorAll<HTMLElement>(".board-card")].filter((c) => c.getBoundingClientRect().top + c.offsetHeight / 2 < y);
  return { kind: "board", el: col, column, index: above.length };
}

// The node may sit past the loaded levels, so its folder is opened first to get its details.
async function nodeOf(parts: string[]): Promise<TreeNode | null> {
  if (!findNode(parts)) await reveal(parts);
  return findNode(parts);
}

async function drop(parts: string[], target: Target, copy: boolean): Promise<void> {
  if (target.kind === "folder") return moveDragged([parts], target.parts, copy);
  const node = await nodeOf(parts);
  if (node && isWorkboard(node)) await moveCard(node, target.column, target.index);
}

export function installSearchDrag(list: HTMLElement, closeList: () => void): void {
  let drag: Drag | null = null;
  let dragged = false;
  const retarget = (next: Target | null) => {
    if (next?.el !== drag?.target?.el) {
      drag?.target?.el.classList.remove(HIGHLIGHT);
      next?.el.classList.add(HIGHLIGHT);
    }
    if (drag) drag.target = next;
  };
  const cancel = () => {
    retarget(null);
    drag?.ghost?.remove();
    drag = null;
  };
  list.addEventListener("pointerdown", (ev) => {
    cancel();
    dragged = false;
    const key = ev.target instanceof Element ? ev.target.closest<HTMLElement>(".search-hit")?.dataset.key : undefined;
    if (ev.button !== 0 || !key) return;
    const parts = key.split("/");
    const node = findNode(parts);
    if (node && !canMove(node)) return;
    drag = { parts, start: pointOf(ev), ghost: null, target: null };
  });
  list.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    if (released(ev)) return cancel();
    const now = pointOf(ev);
    if (!drag.ghost) {
      if (!pastThreshold(drag.start, now)) return;
      drag.ghost = ghostFor(drag.parts);
      list.setPointerCapture(ev.pointerId);
    }
    moveGhost(drag.ghost, now);
    const mode = drag.ghost.querySelector(".drag-ghost-mode");
    if (mode) mode.textContent = ev.ctrlKey ? "+ Copy" : "→ Move";
    const folder = targetAt(now);
    retarget(folder && keyOf(folder.parts) !== keyOf(drag.parts) ? { kind: "folder", ...folder } : boardTarget(now));
  });
  list.addEventListener("pointerup", (ev) => {
    const done = drag;
    const target = done?.ghost ? done.target : null;
    dragged = done?.ghost != null;
    cancel();
    if (!done || !target) return;
    closeList();
    void drop(done.parts, target, ev.ctrlKey);
  });
  // A drag ends with a click on the hit, which must not also open it.
  list.addEventListener("click", (ev) => {
    if (!dragged) return;
    dragged = false;
    ev.stopPropagation();
  }, true);
  list.addEventListener("pointercancel", cancel);
  cancelOnEscape(() => !!drag?.ghost, cancel);
}
