// Folders and files dragged inside the window move onto a folder, or copy with Ctrl; HTML5 drag events never reach the webview.
import { moveFiles, moveFolders } from "./ops";
import { dragSources, ITEM, LEFT_BUTTON, leftWindow, pastThreshold, type Point } from "./drag-out";
import { el } from "../components/dom";
import { warningConfirmDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { findNode, keyOf, outermost } from "../services/store/state";
import type { TreeNode } from "../services/ipc/types";

const HIGHLIGHT = "drop-target";
const GHOST_OFFSET_PX = 12;

type Movable = Pick<TreeNode, "nodeType" | "reserved" | "ghost" | "locked" | "flags" | "file">;

// Main categories never move; an unflagged node moves only after a warning.
export const canMove = (n: Movable | null): boolean =>
  n !== null &&
  !n.reserved &&
  !n.ghost &&
  !n.locked &&
  !n.flags.includes("main") &&
  (n.file || n.nodeType?.base === "NODE" || n.nodeType?.base === "WKB");

// Files are never a destination; review folders are locked but still take folders and files.
export const canDropOn = (n: Movable | null): boolean =>
  n !== null &&
  !n.file &&
  !n.reserved &&
  !n.ghost &&
  ((!n.locked && (n.nodeType?.base === "NODE" || n.nodeType?.base === "WKB")) || n.nodeType?.base === "REV");

// Neither a folder nor anything inside it can become its own destination.
const holds = (source: string[], target: string[]): boolean => source.length <= target.length && source.every((part, i) => part === target[i]);

// An unflagged node can break its paths in other apps, so moving it asks first; copies never do.
async function moveDragged(sources: string[][], target: string[], copy: boolean): Promise<void> {
  const nodes = outermost(sources)
    .filter((parts) => !holds(parts, target))
    .flatMap((parts) => {
      const node = findNode(parts);
      return node ? [{ parts, node }] : [];
    });
  const files = nodes.filter(({ node }) => node.file).map(({ parts }) => parts);
  const folders = nodes.filter(({ node }) => !node.file);
  const unflagged = folders.filter(({ node }) => node.nodeType?.base === "NODE" && !node.flags.includes("data"));
  if (unflagged.length > 0 && !copy) {
    const warnings = unflagged.map(({ node }) => `"${node.name}" has no data flag. Moving it can break its paths in other apps.`);
    if (!(await warningConfirmDialog("Move folder", warnings, "Move", "danger"))) return;
  }
  if (files.length > 0) await moveFiles(files, target, copy);
  if (folders.length > 0) await moveFolders(folders.map(({ parts }) => parts), target, copy);
}

// The storage root has the empty key, which is never a node.
const partsOf = (key: string | undefined): string[] | null => (key ? key.split("/") : null);

interface Drag {
  key: string;
  mods: { ctrl: boolean; shift: boolean };
  sources: string[][];
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
    const parts = partsOf(item?.dataset.key);
    if (!item || !parts || !canMove(findNode(parts))) return;
    drag = { key: keyOf(parts), mods: { ctrl: ev.ctrlKey, shift: ev.shiftKey }, sources: [], start: { x: ev.clientX, y: ev.clientY }, ghost: null, target: null };
  });
  host.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    const now = { x: ev.clientX, y: ev.clientY };
    // Leaving the window hands the gesture to the native drag-out.
    if ((ev.buttons & LEFT_BUTTON) === 0 || leftWindow(now, window.innerWidth, window.innerHeight)) return cancel();
    if (!drag.ghost) {
      if (!pastThreshold(drag.start, now)) return;
      drag.sources = dragSources(drag.key, drag.mods).map((key) => key.split("/")).filter((parts) => canMove(findNode(parts)));
      const label = drag.sources.length === 1 ? (findNode(drag.sources[0])?.name ?? "") : `${drag.sources.length} items`;
      drag.ghost = el("div", { class: "drag-ghost", "aria-hidden": "true" }, label);
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
    if (done && target) void moveDragged(done.sources, target.parts, ev.ctrlKey);
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
