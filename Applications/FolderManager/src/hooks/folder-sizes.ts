// Asks the backend for the totals of the folders a Details listing shows and paints them as they arrive.
import * as api from "../services/ipc/api";
import { onFolderSize } from "../services/ipc/events";
import { forgetAsked, needSizes, storeSize } from "../services/store/folder-size";
import { keyOf, notify, sortOf } from "../services/store/state";
import { formatSize } from "../components/windows/views/details-cells";
import { reportError } from "../components/organisms/bars/status-line";
import type { TreeNode } from "../services/ipc/types";

let listening: Promise<unknown> | null = null;
let repaint = 0;

// A size column sorted by size needs a new order; otherwise only the cell changes.
function arrived(relParts: string[], bytes: number): void {
  const key = keyOf(relParts);
  storeSize(key, bytes);
  for (const cell of document.querySelectorAll<HTMLElement>(".grid-item .detail-size")) {
    if (cell.parentElement?.dataset.key === key) cell.textContent = formatSize(bytes);
  }
  if (sortOf(keyOf(relParts.slice(0, -1))).by !== "size" || repaint) return;
  repaint = window.setTimeout(() => {
    repaint = 0;
    notify();
  }, 250);
}

// One batched call per listing; the listener is attached first so no answer is missed.
export function requestFolderSizes(nodes: TreeNode[]): void {
  const keys = needSizes(nodes.filter((n) => !n.file).map((n) => keyOf(n.relParts)));
  if (keys.length === 0) return;
  listening ??= onFolderSize(arrived);
  listening
    .then(() => api.folderSizes(keys.map((k) => k.split("/"))))
    .catch((e) => {
      forgetAsked(keys);
      reportError("warning", e, "Folder sizes");
    });
}
