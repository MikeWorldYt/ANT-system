// Dragging a folder or file row out of the window hands it to Explorer as a move through the backend's native drag.
import { startDragOut } from "../services/ipc/api";
import { forgetMoved, refresh } from "./ops";
import { findNode, isSelected, select, state } from "../services/store/state";
import { inDownloads } from "../services/ipc/types";
import { leftWindow, modsOf, mousePress, pastThreshold, pointOf, released, type Mods, type Point } from "./pointer";
import { setStatus, reportError } from "../components/organisms/bars/status-line";

// Explorer's drop effect for a move.
const DROPEFFECT_MOVE = 2;
export const ITEM = ".row, .col-item, .grid-item";
const onWindows = navigator.userAgent.includes("Windows");

// A pick inside the selection drags all of it; any other item drags alone and becomes the selection.
export function dragSources(key: string, mods: Mods): string[] {
  if (isSelected(key)) return [...state.selectedKeys];
  if (!mods.ctrl && !mods.shift) select(key.split("/"));
  return [key];
}

// Main categories are top-level, so the backend refuses them; checking here keeps one of them from failing the whole drag.
const dragOutable = (parts: string[]): boolean => {
  const node = findNode(parts);
  return parts.length > 1 && !inDownloads(parts) && node !== null && !node.reserved && !node.ghost && !node.flags.includes("main");
};

async function dragOut(sources: string[][]): Promise<void> {
  // Explorer cannot be asked mid-drag, so the warning shows in the status line before it starts.
  const unflagged = sources.flatMap((parts) => {
    const node = findNode(parts);
    return node?.nodeType?.base === "NODE" && !node.flags.includes("data") ? [node.name] : [];
  });
  if (unflagged.length > 0) setStatus("warning", `Moving "${unflagged.join('", "')}" out can break its paths in other apps`);
  try {
    const effect = await startDragOut(sources);
    await refresh();
    if (effect === DROPEFFECT_MOVE) await forgetMoved(sources);
  } catch (e) {
    reportError("warning", e);
  }
}

export function installDragOut(host: HTMLElement): void {
  if (!onWindows) return;
  let gesture: { key: string; mods: Mods; start: Point } | null = null;
  host.addEventListener("pointerdown", (ev) => {
    gesture = null;
    const target = mousePress(ev);
    if (!target) return;
    const item = target.closest<HTMLElement>(ITEM);
    const key = item?.dataset.key;
    if (!item || !key || item.matches(".reserved, .ghost") || inDownloads(key.split("/")) || findNode(key.split("/"))?.flags.includes("main")) return;
    gesture = { key, mods: modsOf(ev), start: pointOf(ev) };
  });
  host.addEventListener("pointermove", (ev) => {
    if (!gesture) return;
    if (released(ev)) {
      gesture = null;
      return;
    }
    const now = pointOf(ev);
    // Captured after the threshold so the moves keep reaching the host once the pointer is outside the window.
    if (pastThreshold(gesture.start, now) && !host.hasPointerCapture(ev.pointerId)) host.setPointerCapture(ev.pointerId);
    if (!leftWindow(now, window.innerWidth, window.innerHeight)) return;
    const { key, mods } = gesture;
    gesture = null;
    const sources = dragSources(key, mods).map((k) => k.split("/")).filter(dragOutable);
    if (sources.length > 0) void dragOut(sources);
  });
  const end = () => (gesture = null);
  host.addEventListener("pointerup", end);
  host.addEventListener("pointercancel", end);
}
