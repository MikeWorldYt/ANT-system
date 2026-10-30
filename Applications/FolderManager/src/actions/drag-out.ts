// Dragging a folder row out of the window hands it to Explorer through the backend's native drag.
import { describeError, startDragOut } from "../ipc/api";
import { setStatus } from "../ui/status";

const THRESHOLD_PX = 6;
export const ITEM = ".row, .col-item, .grid-item";
export const LEFT_BUTTON = 1;
const onWindows = navigator.userAgent.includes("Windows");

export interface Point {
  x: number;
  y: number;
}

export const pastThreshold = (start: Point, now: Point, threshold = THRESHOLD_PX): boolean =>
  Math.hypot(now.x - start.x, now.y - start.y) > threshold;

export const leftWindow = ({ x, y }: Point, width: number, height: number): boolean =>
  x < 0 || y < 0 || x >= width || y >= height;

async function dragOut(relParts: string[]): Promise<void> {
  try {
    await startDragOut(relParts);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

export function installDragOut(host: HTMLElement): void {
  if (!onWindows) return;
  let gesture: { key: string; start: Point } | null = null;
  host.addEventListener("pointerdown", (ev) => {
    gesture = null;
    if (ev.button !== 0 || ev.pointerType !== "mouse" || !(ev.target instanceof Element)) return;
    const item = ev.target.closest<HTMLElement>(ITEM);
    const key = item?.dataset.key;
    if (!item || !key || item.matches(".reserved, .ghost")) return;
    gesture = { key, start: { x: ev.clientX, y: ev.clientY } };
  });
  host.addEventListener("pointermove", (ev) => {
    if (!gesture) return;
    if ((ev.buttons & LEFT_BUTTON) === 0) {
      gesture = null;
      return;
    }
    const now = { x: ev.clientX, y: ev.clientY };
    // Captured after the threshold so the moves keep reaching the host once the pointer is outside the window.
    if (pastThreshold(gesture.start, now) && !host.hasPointerCapture(ev.pointerId)) host.setPointerCapture(ev.pointerId);
    if (!leftWindow(now, window.innerWidth, window.innerHeight)) return;
    const { key } = gesture;
    gesture = null;
    void dragOut(key.split("/"));
  });
  const end = () => (gesture = null);
  host.addEventListener("pointerup", end);
  host.addEventListener("pointercancel", end);
}
