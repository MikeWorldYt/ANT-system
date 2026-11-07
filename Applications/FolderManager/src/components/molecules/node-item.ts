// What every view item shows the same way: its key, selection, missing, locked and reserved marks, and its tooltip.
import { clickSelect, isSelected, keyOf, startEdit } from "../../services/store/state";
import { nodeSummary } from "../../services/store/node-info";
import { openFolder } from "../../hooks/path-actions";
import { inDownloads, type TreeNode } from "../../services/ipc/types";

export function markNodeItem(item: HTMLElement, n: TreeNode): void {
  const key = keyOf(n.relParts);
  const selected = isSelected(key);
  item.dataset.key = key;
  item.setAttribute("aria-selected", String(selected));
  if (selected) item.classList.add("selected");
  if (n.ghost) {
    item.classList.add("ghost");
    item.setAttribute("aria-label", `${n.name} (missing on disk)`);
  }
  if (n.locked) {
    item.classList.add("locked");
    item.setAttribute("aria-disabled", "true");
  }
  item.classList.toggle("reserved", n.reserved);
  item.classList.toggle("rev", n.nodeType?.base === "REV");
  item.title = nodeSummary(n);
}

// As in Explorer the name renames and the icon opens; names in Downloads are not ours, so there a file opens either way.
export function editOrOpen(n: TreeNode, ev: MouseEvent): void {
  const onName = ev.target instanceof Element && ev.target.closest(".name") !== null;
  if (onName && !inDownloads(n.relParts)) startEdit(n.relParts);
  else if (n.file) void openFolder(n);
}

// Ctrl and Shift extend the selection over the listed keys; a plain click runs `pick`.
export function onItemClick(item: HTMLElement, n: TreeNode, keys: () => string[], pick: () => void, ignore?: (ev: MouseEvent) => boolean): void {
  item.addEventListener("click", (ev) => {
    if (ignore?.(ev)) return;
    if (ev.ctrlKey || ev.shiftKey) clickSelect(n.relParts, keys(), { ctrl: ev.ctrlKey, shift: ev.shiftKey });
    else pick();
  });
}
