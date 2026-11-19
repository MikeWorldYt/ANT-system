// While a drag carries items, resting the pointer still on a place opens it, so the drag can browse to where it is going.
import { toggleExpand } from "./ops";
import { findNode, keyOf, select, state } from "../services/store/state";
import type { TreeNode } from "../services/ipc/types";
import type { Point } from "./pointer";

const FOLDER_MS = 2500;
const QUICK_MS = 1000;
const STILL_PX = 4;
const QUICK = ".tab-name, .crumb-btn, .view-picker:not(.sort-btn), .view-choice";
// Grid items and graph file rows enter a folder on double click; the other views open it by selecting and expanding.
const ENTERS = ".grid-item, .graph-file";

let anchor: Point | null = null;
let timer: number | undefined;

export function stopDwell(): void {
  window.clearTimeout(timer);
  anchor = null;
}

function open(item: HTMLElement, n: TreeNode): void {
  if (item.matches(ENTERS)) {
    item.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    return;
  }
  select(n.relParts);
  if (n.hasChildren && !state.expanded.has(keyOf(n.relParts))) void toggleExpand(n);
}

// Only a pointer that stays put starts the count; passing over a place or scrolling under it starts again.
export function dwell(now: Point): void {
  if (anchor && Math.hypot(now.x - anchor.x, now.y - anchor.y) <= STILL_PX) return;
  stopDwell();
  anchor = now;
  const hit = document.elementFromPoint(now.x, now.y);
  const quick = hit?.closest<HTMLElement>(QUICK);
  if (quick) {
    timer = window.setTimeout(() => quick.click(), QUICK_MS);
    return;
  }
  const item = hit?.closest<HTMLElement>("[data-key]");
  const key = item?.dataset.key;
  const n = key && !key.endsWith("#files") ? findNode(key.split("/")) : null;
  if (item && n && !n.file) timer = window.setTimeout(() => open(item, n), FOLDER_MS);
}

export function redwell(): void {
  const at = anchor;
  stopDwell();
  if (at) requestAnimationFrame(() => dwell(at));
}
