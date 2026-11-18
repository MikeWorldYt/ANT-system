// Files dragged in from Explorer: highlight the folder under the pointer and hand the drop to the backend.
import { dropFiles } from "./ops";
import { announceDrop, DROP_TARGET, folderMoveActive } from "./folder-move";
import { setStatus } from "../components/organisms/bars/status-line";
import type { DragDropEvent } from "@tauri-apps/api/webview";
import { DOWNLOADS_ROOT } from "../services/ipc/types";
import { batch } from "./automations/batch";
import { dropIntoBatch, groupAt } from "./automations/batch-drop";

const HIGHLIGHT = "drop-target";
let marked: HTMLElement | null = null;

const dialogOpen = () => document.querySelector("dialog[open]") !== null;

// Tauri reports physical pixels relative to the webview, while elementFromPoint takes CSS pixels.
function nodeElementAt({ x, y }: { x: number; y: number }): HTMLElement | null {
  const ratio = window.devicePixelRatio;
  return document.elementFromPoint(x / ratio, y / ratio)?.closest<HTMLElement>(DROP_TARGET) ?? null;
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
  // While a batch is open its view takes the drop: the files land in its folder, and a group under the pointer may claim them.
  if (batch && !dialogOpen()) {
    const ratio = window.devicePixelRatio;
    const under = document.elementFromPoint(e.position.x / ratio, e.position.y / ratio);
    if (under?.closest(".batch")) {
      const group = groupAt(under);
      if (e.type !== "drop") return mark(group ?? under.closest<HTMLElement>(".batch-scroll"));
      mark(null);
      void dropIntoBatch(e.paths, group);
      return;
    }
  }
  const hit = dialogOpen() ? null : nodeElementAt(e.position);
  // Reserved, missing and locked folders take no files, except review folders; a refused drop still reaches the backend for its message.
  const outside = hit?.dataset.key?.startsWith(DOWNLOADS_ROOT) === true;
  if (e.type !== "drop") return mark(hit && !outside && !hit.matches(".reserved, .ghost, .locked:not(.rev)") && hit.dataset.key !== "" ? hit : null);
  mark(null);
  if (dialogOpen()) return;
  // A graph box carries its folder's key with "#files" after it; the files land in that folder.
  const key = hit?.dataset.key?.replace(/#files$/, "");
  if (key === undefined) return setStatus("warning", "Drop onto a folder");
  // The storage root has the empty key; "".split("/") would be [""], so it maps to an empty target for the backend to refuse.
  const target = key === "" ? [] : key.split("/");
  void dropFiles(target, e.paths).then(() => announceDrop(target, e.paths.length, null));
}
