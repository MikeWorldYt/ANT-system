// Pointer gestures every drag in the app shares; HTML5 drag events never reach the webview, so each drag is built on these.
import { el } from "../components/dom";

const THRESHOLD_PX = 6;
const GHOST_OFFSET_PX = 12;
const LEFT_BUTTON = 1;

export interface Point {
  x: number;
  y: number;
}

export interface Mods {
  ctrl: boolean;
  shift: boolean;
}

export const pointOf = (ev: MouseEvent): Point => ({ x: ev.clientX, y: ev.clientY });

export const modsOf = (ev: MouseEvent): Mods => ({ ctrl: ev.ctrlKey, shift: ev.shiftKey });

// The element under a primary mouse press; touch and pen keep the webview's own gestures.
export const mousePress = (ev: PointerEvent): Element | null => (ev.button === 0 && ev.pointerType === "mouse" && ev.target instanceof Element ? ev.target : null);

// A move with the left button up means the release happened outside the window.
export const released = (ev: PointerEvent): boolean => (ev.buttons & LEFT_BUTTON) === 0;

export const pastThreshold = (start: Point, now: Point, threshold = THRESHOLD_PX): boolean =>
  Math.hypot(now.x - start.x, now.y - start.y) > threshold;

export const leftWindow = ({ x, y }: Point, width: number, height: number): boolean =>
  x < 0 || y < 0 || x >= width || y >= height;

// The drop index in a vertical list: how many items have their middle above the pointer.
export const countAbove = (items: Element[], y: number): number =>
  items.filter((item) => {
    const r = item.getBoundingClientRect();
    return r.top + r.height / 2 < y;
  }).length;

export function dragGhost(label: string): HTMLElement {
  const ghost = el("div", { class: "drag-ghost", "aria-hidden": "true" }, label);
  document.body.append(ghost);
  return ghost;
}

export function moveGhost(ghost: HTMLElement, { x, y }: Point): void {
  ghost.style.left = `${x + GHOST_OFFSET_PX}px`;
  ghost.style.top = `${y + GHOST_OFFSET_PX}px`;
}

// Captured, so Escape ends a running drag before any view or dialog sees the key; losing the window ends it too.
export function cancelOnEscape(active: () => boolean, cancel: () => void): void {
  window.addEventListener(
    "keydown",
    (ev) => {
      if (ev.key !== "Escape" || !active()) return;
      ev.stopPropagation();
      ev.preventDefault();
      cancel();
    },
    true,
  );
  window.addEventListener("blur", cancel);
}
