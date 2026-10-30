// Folders dragged inside the window move onto another folder, or copy with Ctrl; HTML5 drag events never reach the webview.
import { moveFolder } from "./ops";
import { ITEM, LEFT_BUTTON, leftWindow, pastThreshold, type Point } from "./drag-out";
import { el } from "../ui/dom";
import { findNode } from "../store/state";
import type { TreeNode } from "../ipc/types";

const HIGHLIGHT = "drop-target";
const GHOST_OFFSET_PX = 12;

type Movable = Pick<TreeNode, "nodeType" | "reserved" | "ghost" | "locked">;

export const canMove = (n: Movable | null): boolean =>
  n !== null && !n.reserved && !n.ghost && !n.locked && (n.nodeType?.base === "NODE" || n.nodeType?.base === "WKB");

// Review folders are locked but still take folders, as they take files.
export const canDropOn = (n: Movable | null): boolean =>
  canMove(n) || (n !== null && !n.reserved && !n.ghost && n.nodeType?.base === "REV");

// The storage root has the empty key, which is never a node.
const partsOf = (key: string | undefined): string[] | null => (key ? key.split("/") : null);

interface Drag {
  source: string[];
  name: string;
  start: Point;
  ghost: HTMLElement | null;
  target: { el: HTMLElement; parts: string[] } | null;
}

let drag: Drag | null = null;

export const folderMoveActive = (): boolean => drag?.ghost != null;

const dialogOpen = () => document.querySelector("dialog[open]") !== null;

function targetAt({ x, y }: Point): Drag["target"] {
  if (dialogOpen()) return null;
  const hit = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-key]") ?? null;
  const parts = partsOf(hit?.dataset.key);
  return hit && parts && canDropOn(findNode(parts)) ? { el: hit, parts } : null;
}

function retarget(d: Drag, next: Drag["target"]): void {
  if (next?.el === d.target?.el) return;
  d.target?.el.classList.remove(HIGHLIGHT);
  next?.el.classList.add(HIGHLIGHT);
  d.target = next;
}

function place(ghost: HTMLElement, { x, y }: Point): void {
  ghost.style.left = `${x + GHOST_OFFSET_PX}px`;
  ghost.style.top = `${y + GHOST_OFFSET_PX}px`;
}

function cancel(): void {
  if (!drag) return;
  drag.target?.el.classList.remove(HIGHLIGHT);
  drag.ghost?.remove();
  drag = null;
}

export function installFolderMove(host: HTMLElement): void {
  host.addEventListener("pointerdown", (ev) => {
    cancel();
    if (ev.button !== 0 || ev.pointerType !== "mouse" || !(ev.target instanceof Element)) return;
    if (ev.target.closest("input, textarea")) return;
    const item = ev.target.closest<HTMLElement>(ITEM);
    const source = partsOf(item?.dataset.key);
    if (!item || !source) return;
    const node = findNode(source);
    if (!node || !canMove(node)) return;
    drag = { source, name: node.name, start: { x: ev.clientX, y: ev.clientY }, ghost: null, target: null };
  });
  host.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    const now = { x: ev.clientX, y: ev.clientY };
    // Leaving the window hands the gesture to the native drag-out.
    if ((ev.buttons & LEFT_BUTTON) === 0 || leftWindow(now, window.innerWidth, window.innerHeight)) return cancel();
    if (!drag.ghost) {
      if (!pastThreshold(drag.start, now)) return;
      drag.ghost = el("div", { class: "drag-ghost", "aria-hidden": "true" }, drag.name);
      document.body.append(drag.ghost);
      if (!host.hasPointerCapture(ev.pointerId)) host.setPointerCapture(ev.pointerId);
    }
    place(drag.ghost, now);
    retarget(drag, targetAt(now));
  });
  host.addEventListener("pointerup", (ev) => {
    const done = drag;
    const target = done?.ghost ? done.target : null;
    cancel();
    if (done && target) void moveFolder(done.source, target.parts, ev.ctrlKey);
  });
  host.addEventListener("pointercancel", cancel);
  window.addEventListener(
    "keydown",
    (ev) => {
      if (ev.key !== "Escape" || !folderMoveActive()) return;
      ev.stopPropagation();
      ev.preventDefault();
      cancel();
    },
    true,
  );
  window.addEventListener("blur", cancel);
}
