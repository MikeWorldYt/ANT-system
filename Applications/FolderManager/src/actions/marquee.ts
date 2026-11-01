// Dragging on empty space draws a band that selects every item it touches, as in Explorer.
import { el } from "../ui/dom";
import { selectKeys, state } from "../store/state";
import { ITEM, LEFT_BUTTON, pastThreshold, type Point } from "./drag-out";

interface Band {
  start: Point;
  base: string[];
  box: HTMLElement | null;
}

const NOT_EMPTY = `${ITEM}, button, input, textarea, a`;

let band: Band | null = null;
// The click that ends a band lands on whatever is under the pointer, so it is dropped.
let swallowClick = false;

function hits(host: HTMLElement, a: Point, b: Point): string[] {
  const left = Math.min(a.x, b.x);
  const right = Math.max(a.x, b.x);
  const top = Math.min(a.y, b.y);
  const bottom = Math.max(a.y, b.y);
  return [...host.querySelectorAll<HTMLElement>(ITEM)].flatMap((item) => {
    const rect = item.getBoundingClientRect();
    const key = item.dataset.key;
    return key && rect.left < right && rect.right > left && rect.top < bottom && rect.bottom > top ? [key] : [];
  });
}

function place(box: HTMLElement, a: Point, b: Point): void {
  box.style.left = `${Math.min(a.x, b.x)}px`;
  box.style.top = `${Math.min(a.y, b.y)}px`;
  box.style.width = `${Math.abs(b.x - a.x)}px`;
  box.style.height = `${Math.abs(b.y - a.y)}px`;
}

function sameKeys(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((key, i) => key === b[i]);
}

export function installMarquee(host: HTMLElement): void {
  const end = () => {
    if (band?.box) {
      band.box.remove();
      swallowClick = true;
    }
    band = null;
  };
  host.addEventListener("pointerdown", (ev) => {
    band = null;
    swallowClick = false;
    if (ev.button !== 0 || ev.pointerType !== "mouse" || !(ev.target instanceof Element)) return;
    if (ev.target.closest(NOT_EMPTY)) return;
    band = { start: { x: ev.clientX, y: ev.clientY }, base: ev.ctrlKey ? [...state.selectedKeys] : [], box: null };
  });
  host.addEventListener("pointermove", (ev) => {
    if (!band) return;
    if ((ev.buttons & LEFT_BUTTON) === 0) return end();
    const now = { x: ev.clientX, y: ev.clientY };
    if (!band.box) {
      if (!pastThreshold(band.start, now)) return;
      band.box = el("div", { class: "marquee", "aria-hidden": "true" });
      document.body.append(band.box);
      host.setPointerCapture(ev.pointerId);
    }
    place(band.box, band.start, now);
    const keys = [...new Set([...band.base, ...hits(host, band.start, now)])];
    if (!sameKeys(keys, state.selectedKeys)) selectKeys(keys);
  });
  host.addEventListener("pointerup", end);
  host.addEventListener("pointercancel", end);
  host.addEventListener(
    "click",
    (ev) => {
      if (!swallowClick) return;
      swallowClick = false;
      ev.stopPropagation();
      ev.preventDefault();
    },
    true,
  );
}
