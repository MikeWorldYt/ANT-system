// Dragging on empty space draws a band that selects every item it touches, as in Explorer.
import { el } from "../components/dom";
import { selectKeys, state } from "../services/store/state";
import { ITEM } from "./drag-out";
import { mousePress, pastThreshold, pointOf, released, type Point } from "./pointer";

interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

interface Band {
  // The press in viewport space, for the drag threshold only.
  origin: Point;
  // The anchor in the scroller's content space, so scrolling never moves it.
  start: Point;
  scroller: HTMLElement;
  scrollerClass: string;
  scrollerKey: string | undefined;
  pointer: Point;
  base: string[];
  box: HTMLElement | null;
  frame: number;
  seen: string;
}

// A graph box is content, and its corner resizes it, so a press there never starts a band.
const NOT_EMPTY = `${ITEM}, button, input, textarea, a, .graph-files-box`;
const SCROLLABLE = /auto|scroll/;
const EDGE_PX = 40;
const MAX_STEP_PX = 14;

let band: Band | null = null;
// The click that ends a band lands on whatever is under the pointer, so it is dropped.
let swallowClick = false;

// The nearest scroller under the press: each column scrolls on its own, so a band stays in the one it started in.
function scrollerOf(target: Element, host: HTMLElement): HTMLElement {
  for (let e: Element | null = target; e; e = e.parentElement) {
    if (e instanceof HTMLElement) {
      const style = getComputedStyle(e);
      if (SCROLLABLE.test(style.overflowY) || SCROLLABLE.test(style.overflowX)) return e;
    }
    if (e === host) break;
  }
  return host;
}

// The visible area of a scroller, without borders and scrollbars.
function clientBox(s: HTMLElement): Box {
  const r = s.getBoundingClientRect();
  const left = r.left + s.clientLeft;
  const top = r.top + s.clientTop;
  return { left, top, right: left + s.clientWidth, bottom: top + s.clientHeight };
}

// Every selection re-render rebuilds the columns, so a replaced scroller is found again by its folder key.
function resolve(host: HTMLElement, b: Band): HTMLElement | null {
  if (b.scroller.isConnected) return b.scroller;
  if (!b.scrollerClass) return null;
  const found = [...host.querySelectorAll<HTMLElement>(`.${CSS.escape(b.scrollerClass)}`)].find((e) => e.dataset.key === b.scrollerKey);
  if (found) b.scroller = found;
  return found ?? null;
}

// Scrolls faster the closer the pointer is to the edge, and keeps going past it.
function edgeStep(pos: number, lo: number, hi: number): number {
  const zone = Math.min(EDGE_PX, (hi - lo) / 4);
  const inside = Math.min(pos - lo, hi - pos);
  if (inside >= zone) return 0;
  return (pos - lo < hi - pos ? -1 : 1) * Math.min((zone - inside) / zone, 2) * MAX_STEP_PX;
}

const clamp = (v: number, lo: number, hi: number): number => Math.min(Math.max(v, lo), hi);

function hits(scroller: HTMLElement, band: Box): string[] {
  return [...scroller.querySelectorAll<HTMLElement>(ITEM)].flatMap((item) => {
    const rect = item.getBoundingClientRect();
    const key = item.dataset.key;
    // Level 1 folders are the fixed frame of the storage, so a band never takes them.
    return key && key.includes("/") && rect.left < band.right && rect.right > band.left && rect.top < band.bottom && rect.bottom > band.top ? [key] : [];
  });
}

function place(box: HTMLElement, area: Box, view: Box): void {
  const left = Math.max(area.left, view.left);
  const top = Math.max(area.top, view.top);
  box.style.left = `${left}px`;
  box.style.top = `${top}px`;
  box.style.width = `${Math.max(0, Math.min(area.right, view.right) - left)}px`;
  box.style.height = `${Math.max(0, Math.min(area.bottom, view.bottom) - top)}px`;
}

function sameKeys(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((key, i) => key === b[i]);
}

export function installMarquee(host: HTMLElement): void {
  const end = () => {
    if (band) cancelAnimationFrame(band.frame);
    if (band?.box) {
      band.box.remove();
      swallowClick = true;
    }
    band = null;
  };
  const step = () => {
    const b = band;
    if (!b?.box) return;
    const scroller = resolve(host, b);
    if (!scroller) return end();
    const view = clientBox(scroller);
    scroller.scrollLeft += edgeStep(b.pointer.x, view.left, view.right);
    scroller.scrollTop += edgeStep(b.pointer.y, view.top, view.bottom);
    // Held inside the scroller, so the band grows with the scroll instead of past the edge.
    const x = clamp(b.pointer.x, view.left, view.right);
    const y = clamp(b.pointer.y, view.top, view.bottom);
    const seen = `${x}|${y}|${scroller.scrollLeft}|${scroller.scrollTop}`;
    if (seen === b.seen) return;
    b.seen = seen;
    const shiftX = view.left - scroller.scrollLeft;
    const shiftY = view.top - scroller.scrollTop;
    const nowX = x - shiftX;
    const nowY = y - shiftY;
    const area = {
      left: Math.min(b.start.x, nowX) + shiftX,
      right: Math.max(b.start.x, nowX) + shiftX,
      top: Math.min(b.start.y, nowY) + shiftY,
      bottom: Math.max(b.start.y, nowY) + shiftY,
    };
    place(b.box, area, view);
    const keys = [...new Set([...b.base, ...hits(scroller, area)])];
    if (!sameKeys(keys, state.selectedKeys)) selectKeys(keys);
  };
  // Runs every frame so the scroll goes on while the pointer rests at the edge.
  const tick = () => {
    step();
    if (band?.box) band.frame = requestAnimationFrame(tick);
  };
  host.addEventListener("pointerdown", (ev) => {
    end();
    swallowClick = false;
    const target = mousePress(ev);
    if (!target || target.closest(NOT_EMPTY)) return;
    const scroller = scrollerOf(target, host);
    const view = clientBox(scroller);
    // A press on the scrollbar drags the scrollbar, not a band.
    if (ev.clientX >= view.right || ev.clientY >= view.bottom) return;
    const origin = pointOf(ev);
    const start = { x: origin.x - view.left + scroller.scrollLeft, y: origin.y - view.top + scroller.scrollTop };
    band = {
      origin,
      start,
      scroller,
      scrollerClass: scroller.classList[0] ?? "",
      scrollerKey: scroller.dataset.key,
      pointer: origin,
      base: ev.ctrlKey ? [...state.selectedKeys] : [],
      box: null,
      frame: 0,
      seen: "",
    };
  });
  host.addEventListener("pointermove", (ev) => {
    if (!band) return;
    if (released(ev)) return end();
    band.pointer = pointOf(ev);
    if (band.box || !pastThreshold(band.origin, band.pointer)) return;
    band.box = el("div", { class: "marquee", "aria-hidden": "true" });
    document.body.append(band.box);
    host.setPointerCapture(ev.pointerId);
    band.frame = requestAnimationFrame(tick);
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
