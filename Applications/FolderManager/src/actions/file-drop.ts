// Files dragged in from Explorer: highlight the folder under the pointer and hand the drop to the backend.
import { dropFiles } from "./ops";
import { folderMoveActive } from "./folder-move";
import { setStatus } from "../ui/status";
import type { DragDropEvent } from "@tauri-apps/api/webview";

const HIGHLIGHT = "drop-target";
let marked: HTMLElement | null = null;

const dialogOpen = () => document.querySelector("dialog[open]") !== null;

// Tauri reports physical pixels relative to the webview, while elementFromPoint takes CSS pixels.
function nodeElementAt({ x, y }: { x: number; y: number }): HTMLElement | null {
  const ratio = window.devicePixelRatio;
  return document.elementFromPoint(x / ratio, y / ratio)?.closest<HTMLElement>("[data-key]") ?? null;
}

function mark(next: HTMLElement | null): void {
  if (next === marked) return;
  marked?.classList.remove(HIGHLIGHT);
  next?.classList.add(HIGHLIGHT);
  marked = next;
}

export function handleFileDrag(e: DragDropEvent): void {
  if (folderMoveActive()) return;
  if (e.type === "leave") return mark(null);
  const hit = dialogOpen() ? null : nodeElementAt(e.position);
  // Reserved, missing and locked folders take no files, except review folders; a refused drop still reaches the backend for its message.
  if (e.type !== "drop") return mark(hit && !hit.matches(".reserved, .ghost, .locked:not(.rev)") && hit.dataset.key !== "" ? hit : null);
  mark(null);
  if (dialogOpen()) return;
  const key = hit?.dataset.key;
  if (key === undefined) return setStatus("warning", "Drop onto a folder");
  // The storage root has the empty key; "".split("/") would be [""], so it maps to an empty target for the backend to refuse.
  void dropFiles(key === "" ? [] : key.split("/"), e.paths);
}
