// Folders and files dragged inside the window move onto a folder, or copy with Ctrl; HTML5 drag events never reach the webview.
import { dropFiles, moveFiles, moveFolders } from "./ops";
import { dragSources, ITEM } from "./drag-out";
import { cancelOnEscape, leftWindow, modsOf, mousePress, moveGhost, pastThreshold, pointOf, released, type Mods, type Point } from "./pointer";
import { warningConfirmDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { findNode, keyOf, outermost } from "../services/store/state";
import { nameWithoutIdentifier } from "../services/store/node-info";
import { readPref, savePref } from "../services/store/prefs";
import { el } from "../components/dom";
import { folderIcon } from "../components/atoms/icons";
import { inDownloads, type TreeNode } from "../services/ipc/types";

export const HIGHLIGHT = "drop-target";

type Movable = Pick<TreeNode, "relParts" | "nodeType" | "reserved" | "ghost" | "locked" | "flags" | "file" | "inYaml">;

// A folder the database does not track moves on disk like a file, even when its name makes it a workboard.
const isPlain = (n: Movable): boolean => !n.file && !n.inYaml;

// Main categories never move; an unflagged node moves only after a warning.
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
  !inDownloads(n.relParts) &&
  !n.reserved &&
  !n.ghost &&
  ((!n.locked && (isPlain(n) || n.nodeType?.base === "NODE" || n.nodeType?.base === "WKB")) || n.nodeType?.base === "REV");

// Neither a folder nor anything inside it can become its own destination.
const holds = (source: string[], target: string[]): boolean => source.length <= target.length && source.every((part, i) => part === target[i]);

const SKIP_KEY = "fm.skipNodeMoveWarning";

// Level 2 holds the parent's subcategories, so only a move there gets the structure reminder.
async function confirmNodeMove(count: number, target: string[]): Promise<boolean> {
  if (readPref(SKIP_KEY) === "1") return true;
  const level = target.length + 1;
  const parent = findNode(target);
  const subject = count === 1 ? "this node" : `these ${count} nodes`;
  const skip = el("input", { type: "checkbox", id: "skip-node-move" }) as HTMLInputElement;
  const paragraphs: (string | HTMLElement)[] = [
    ...(level === 2 ? [`Level 2 must use subcategories of the parent (${parent ? nameWithoutIdentifier(parent) : "the storage"}).`] : []),
    `${level === 2 ? "Also, recent" : "Recent"} files and other apps that point inside it will stop finding their files once it moves, and you will need to relink them.`,
    "Move it only if you are reorganizing your storage.",
    el("label", { class: "warning-skip", for: "skip-node-move" }, skip, "Don't show this again, I know the risks"),
  ];
  const ok = await warningConfirmDialog(`Move ${subject} to level ${level}?`, paragraphs, "Move anyway", "danger");
  if (ok && skip.checked) savePref(SKIP_KEY, "1");
  return ok;
}

// An unflagged node can break its paths in other apps, so moving it asks first; copies never do.
export async function moveDragged(sources: string[][], target: string[], copy: boolean): Promise<void> {
  // Downloads lies outside the storage, so its items arrive like files dropped from Explorer, by their absolute path.
  const external = outermost(sources.filter(inDownloads)).flatMap((parts) => findNode(parts)?.absPath ?? []);
  if (external.length > 0) await dropFiles(target, external);
  const nodes = outermost(sources.filter((parts) => !inDownloads(parts)))
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
  const unflagged = folders.filter(({ node }) => node.nodeType?.base === "NODE" && !node.flags.includes("data"));
  if (unflagged.length > 0 && !copy && !(await confirmNodeMove(unflagged.length, folderTarget))) return;
  if (files.length > 0) await moveFiles(files, target, copy);
  if (folders.length > 0) await moveFolders(folders.map(({ parts }) => parts), target, copy);
}

// The storage root has the empty key, which is never a node.
const partsOf = (key: string | undefined): string[] | null => (key ? key.split("/") : null);

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

function cancel(): void {
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
  });
  host.addEventListener("pointerup", (ev) => {
    const done = drag;
    const target = done?.ghost ? done.target : null;
    cancel();
    if (done && target) void moveDragged(done.sources, target.parts, ev.ctrlKey);
  });
  host.addEventListener("pointercancel", cancel);
  cancelOnEscape(folderMoveActive, cancel);
  // Ctrl pressed or let go without moving the pointer still flips the badge.
  for (const type of ["keydown", "keyup"] as const) window.addEventListener(type, (ev) => ev.key === "Control" && showMode(drag?.ghost ?? null, ev.ctrlKey));
}
