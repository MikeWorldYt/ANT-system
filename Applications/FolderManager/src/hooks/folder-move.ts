// Folders and files dragged inside the window move onto a folder, or copy with Ctrl; HTML5 drag events never reach the webview.
import { dropFiles, moveFiles, moveFolders } from "./ops";
import { dwell, redwell, stopDwell } from "./drag-dwell";
import { dragSources, ITEM } from "./drag-out";
import { cancelOnEscape, leftWindow, modsOf, mousePress, moveGhost, pastThreshold, pointOf, released, type Mods, type Point } from "./pointer";
import { findNode, keyOf, outermost } from "../services/store/state";
import { el } from "../components/dom";
import { notifyDesktop } from "../services/desktop-notify";
import { noteTrip } from "./file-trips";
import { plural } from "../services/plural";
import { reportError } from "../components/organisms/bars/status-line";
import { folderIcon } from "../components/atoms/icons";
import { takesFiles } from "../services/store/system-folders";
import { inSystem, type TreeNode } from "../services/ipc/types";

export const HIGHLIGHT = "drop-target";
export const DROP_TARGET = "[data-key]:not(.graph-file)";

type Movable = Pick<TreeNode, "relParts" | "nodeType" | "reserved" | "ghost" | "locked" | "flags" | "file" | "inYaml">;

// A folder the database does not track moves on disk like a file, even when its name makes it a worknode.
const isPlain = (n: Movable): boolean => !n.file && !n.inYaml;

// Main categories never move.
export const canMove = (n: Movable | null): boolean =>
  n !== null &&
  !n.reserved &&
  !n.ghost &&
  !n.locked &&
  !n.flags.includes("main") &&
  (n.file || isPlain(n) || n.nodeType?.base === "NODE" || n.nodeType?.base === "WKB");

// Files are never a destination; review folders are locked but still take folders and files.
export const canDropOn = (n: Movable | null): boolean =>
  n !== null &&
  !n.file &&
  takesFiles(n.relParts) &&
  !n.reserved &&
  !n.ghost &&
  ((!n.locked && (isPlain(n) || n.nodeType?.base === "NODE" || n.nodeType?.base === "WKB")) || ["REV", "SS", "REC"].includes(n.nodeType?.base ?? ""));

// Neither a folder nor anything inside it can become its own destination.
const holds = (source: string[], target: string[]): boolean => source.length <= target.length && source.every((part, i) => part === target[i]);

export async function moveDragged(sources: string[][], target: string[], copy: boolean): Promise<void> {
  // The native folders lie outside the storage, so their items arrive like files dropped from Explorer, by their absolute path.
  const external = outermost(sources.filter(inSystem)).flatMap((parts) => findNode(parts)?.absPath ?? []);
  if (external.length > 0) await dropFiles(target, external);
  const nodes = outermost(sources.filter((parts) => !inSystem(parts)))
    .filter((parts) => !holds(parts, target))
    .flatMap((parts) => {
      const node = findNode(parts);
      return node ? [{ parts, node }] : [];
    });
  // Plain folders have no record to move, so they travel like files, straight on disk.
  const files = nodes.filter(({ node }) => node.file || isPlain(node)).map(({ parts }) => parts);
  // A folder dropped on a review folder lands in its owner, so a drop where it already sits is a cancel, not an error.
  const folderTarget = findNode(target)?.nodeType?.base === "REV" ? target.slice(0, -1) : target;
  const folders = nodes.filter(({ node, parts }) => !node.file && !isPlain(node) && (copy || keyOf(parts.slice(0, -1)) !== keyOf(folderTarget)));
  if (files.length > 0) await moveFiles(files, target, copy);
  if (folders.length > 0) await moveFolders(folders.map(({ parts }) => parts), target, copy);
  if (files.length > 0 || external.length > 0) announceDrop(target, files.length + external.length, files[0]?.slice(0, -1) ?? (external.length > 0 ? (sources.find(inSystem)?.slice(0, 1) ?? null) : null));
}

// Views that draw where files went, like the graph's packets, listen for this. A node that keeps no files passes them to its review folder,
// which the user is told about, so the files never seem to have vanished.
export function announceDrop(target: string[], count: number, from: string[] | null): void {
  const node = findNode(target);
  const passes = node !== null && node.nodeType?.base === "NODE" && !node.flags.includes("data");
  const review = passes ? (node.children ?? []).find((c) => c.nodeType?.base === "REV") : undefined;
  noteTrip({ from, to: review?.relParts ?? target });
  if (!node || !passes) return;
  const message = `${plural(count, "file")} dropped on ${node.name} moved to ${review?.name ?? "its review folder"}, since this node keeps no files.`;
  void notifyDesktop(message).catch((e: unknown) => reportError("warning", e, "Notification"));
}

// The storage root has the empty key, which is never a node.
// A graph files box carries its folder's key with "#files" after it, so a drop there lands in that folder.
const partsOf = (key: string | undefined): string[] | null => (key ? key.replace(/#files$/, "").split("/") : null);

interface Drag {
  key: string;
  mods: Mods;
  sources: string[][];
  start: Point;
  ghost: HTMLElement | null;
  target: { el: HTMLElement; parts: string[] } | null;
}

let drag: Drag | null = null;

// Like Explorer, the ghost shows what is carried and a badge with the action a release would run.
function itemGhost(sources: string[][]): HTMLElement {
  const first = sources[0] ? findNode(sources[0]) : null;
  const icon = first ? folderIcon(first) : el("span");
  icon.classList.add("drag-ghost-icon");
  const count = sources.length > 1 ? [el("span", { class: "drag-ghost-count" }, String(sources.length))] : [];
  const ghost = el("div", { class: "drag-ghost-item", "aria-hidden": "true" }, icon, ...count, el("span", { class: "drag-ghost-mode" }));
  document.body.append(ghost);
  return ghost;
}

function showMode(ghost: HTMLElement | null, copy: boolean): void {
  const mode = ghost?.querySelector(".drag-ghost-mode");
  if (mode) mode.textContent = copy ? "+ Copy" : "→ Move";
}

export const folderMoveActive = (): boolean => drag?.ghost != null;

const dialogOpen = () => document.querySelector("dialog[open]") !== null;

export function targetAt({ x, y }: Point): Drag["target"] {
  if (dialogOpen()) return null;
  // A file row is never a destination, so the folder holding it is.
  const hit = document.elementFromPoint(x, y)?.closest<HTMLElement>(DROP_TARGET) ?? null;
  const parts = partsOf(hit?.dataset.key);
  return hit && parts && canDropOn(findNode(parts)) ? { el: hit, parts } : null;
}

function retarget(d: Drag, next: Drag["target"]): void {
  if (next?.el === d.target?.el) return;
  d.target?.el.classList.remove(HIGHLIGHT);
  next?.el.classList.add(HIGHLIGHT);
  d.target = next;
}

function cancel(): void {
  stopDwell();
  if (!drag) return;
  drag.target?.el.classList.remove(HIGHLIGHT);
  drag.ghost?.remove();
  drag = null;
}

export function installFolderMove(host: HTMLElement): void {
  host.addEventListener("pointerdown", (ev) => {
    cancel();
    const target = mousePress(ev);
    if (!target || target.closest("input, textarea")) return;
    const item = target.closest<HTMLElement>(ITEM);
    const parts = partsOf(item?.dataset.key);
    if (!item || !parts || !canMove(findNode(parts))) return;
    drag = { key: keyOf(parts), mods: modsOf(ev), sources: [], start: pointOf(ev), ghost: null, target: null };
  });
  host.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    const now = pointOf(ev);
    // Leaving the window hands the gesture to the native drag-out.
    if (released(ev) || leftWindow(now, window.innerWidth, window.innerHeight)) return cancel();
    if (!drag.ghost) {
      if (!pastThreshold(drag.start, now)) return;
      drag.sources = dragSources(drag.key, drag.mods).map((key) => key.split("/")).filter((parts) => canMove(findNode(parts)));
      drag.ghost = itemGhost(drag.sources);
      if (!host.hasPointerCapture(ev.pointerId)) host.setPointerCapture(ev.pointerId);
    }
    moveGhost(drag.ghost, now);
    showMode(drag.ghost, ev.ctrlKey);
    retarget(drag, targetAt(now));
    dwell(now);
  });
  window.addEventListener("wheel", () => folderMoveActive() && redwell(), { passive: true });
  host.addEventListener("pointerup", (ev) => {
    const done = drag;
    const target = done?.ghost ? done.target : null;
    cancel();
    if (!done || !target) return;
    // Letting go over the folder the items already sit in is a cancelled drag, not a move to report as refused.
    const sources = ev.ctrlKey ? done.sources : done.sources.filter((p) => keyOf(p.slice(0, -1)) !== keyOf(target.parts));
    if (sources.length > 0) void moveDragged(sources, target.parts, ev.ctrlKey);
  });
  host.addEventListener("pointercancel", cancel);
  cancelOnEscape(folderMoveActive, cancel);
  // Ctrl pressed or let go without moving the pointer still flips the badge.
  for (const type of ["keydown", "keyup"] as const) window.addEventListener(type, (ev) => ev.key === "Control" && showMode(drag?.ghost ?? null, ev.ctrlKey));
}
