// Cards move between and within the board columns with the pointer; HTML5 drag events never reach the webview.
import { el } from "../components/dom";
import { cancelOnEscape, countAbove, dragGhost, moveGhost, pastThreshold, pointOf, released, type Point } from "./pointer";
import { BOARD_COLUMNS } from "../services/store/board";
import type { BoardColumn } from "../services/ipc/types";

interface Slot {
  list: HTMLElement;
  column: BoardColumn;
  index: number;
}

interface Drag {
  key: string;
  start: Point;
  ghost: HTMLElement | null;
  slot: Slot | null;
}

const othersIn = (list: HTMLElement, key: string): HTMLElement[] =>
  [...list.querySelectorAll<HTMLElement>(".board-card")].filter((c) => c.dataset.card !== key);

// The index counts the other cards above the pointer, as the dragged one leaves its place.
function slotAt({ x, y }: Point, key: string): Slot | null {
  const col = document.elementFromPoint(x, y)?.closest<HTMLElement>(".board-col");
  const list = col?.querySelector<HTMLElement>(".board-list");
  const column = BOARD_COLUMNS.find(([id]) => id === col?.dataset.column)?.[0];
  if (!list || !column) return null;
  return { list, column, index: countAbove(othersIn(list, key), y) };
}

function mark(slot: Slot | null, key: string): void {
  document.querySelector(".board-slot")?.remove();
  if (!slot) return;
  const line = el("li", { class: "board-slot", "aria-hidden": "true" });
  const before = othersIn(slot.list, key)[slot.index];
  if (before) before.before(line);
  else slot.list.append(line);
}

export function installBoardDrag(root: HTMLElement, drop: (key: string, column: BoardColumn, index: number) => void): void {
  let drag: Drag | null = null;
  const cancel = () => {
    drag?.ghost?.remove();
    mark(null, "");
    drag = null;
  };
  root.addEventListener("pointerdown", (ev) => {
    cancel();
    if (ev.button !== 0 || !(ev.target instanceof Element) || ev.target.closest("button")) return;
    const key = ev.target.closest<HTMLElement>(".board-card")?.dataset.card;
    if (key) drag = { key, start: pointOf(ev), ghost: null, slot: null };
  });
  root.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    if (released(ev)) return cancel();
    const now = pointOf(ev);
    if (!drag.ghost) {
      if (!pastThreshold(drag.start, now)) return;
      const name = root.querySelector(`[data-card="${CSS.escape(drag.key)}"] .board-name`)?.textContent ?? "";
      drag.ghost = dragGhost(name);
      root.setPointerCapture(ev.pointerId);
    }
    moveGhost(drag.ghost, now);
    drag.slot = slotAt(now, drag.key);
    mark(drag.slot, drag.key);
  });
  root.addEventListener("pointerup", () => {
    const done = drag;
    cancel();
    if (done?.ghost && done.slot) drop(done.key, done.slot.column, done.slot.index);
  });
  root.addEventListener("pointercancel", cancel);
  cancelOnEscape(() => !!drag?.ghost, cancel);
}
