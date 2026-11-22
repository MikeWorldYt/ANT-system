// What every view item shows the same way: its key, selection, missing, locked and reserved marks, and its tooltip.
import { clickSelect, isSelected, keyOf, startEdit, state } from "../../services/store/state";
import { nodeSummary } from "../../services/store/node-info";
import { openFolder } from "../../hooks/path-actions";
import type { TreeNode } from "../../services/ipc/types";

// Explorer's slow double click: a click on the name of the only selected item, held apart from a double click, renames it.
const SLOW_CLICK_MS = 600;

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
  item.classList.toggle("locked", n.locked);
  item.classList.toggle("reserved", n.reserved);
  item.classList.toggle("concealed", n.concealed);
  item.classList.toggle("rev", n.nodeType?.base === "REV");
  item.title = nodeSummary(n);
  renameOnSlowClick(item, n, key);
}

function renameOnSlowClick(item: HTMLElement, n: TreeNode, key: string): void {
  let wasOnly = false;
  let timer: number | undefined;
  item.addEventListener("pointerdown", () => {
    wasOnly = state.selectedKeys.length === 1 && isSelected(key);
  });
  item.addEventListener("click", (ev) => {
    window.clearTimeout(timer);
    const onName = ev.target instanceof Element && ev.target.closest(".name-text") !== null;
    if (!wasOnly || !onName || ev.detail > 1 || ev.ctrlKey || ev.shiftKey) return;
    timer = window.setTimeout(() => {
      if (state.selectedKeys.length === 1 && isSelected(key)) startEdit(n.relParts);
    }, SLOW_CLICK_MS);
  });
  item.addEventListener("dblclick", () => window.clearTimeout(timer));
}

export function openOnDouble(n: TreeNode): void {
  if (n.file) void openFolder(n);
}

// Ctrl and Shift extend the selection over the listed keys; a plain click runs `pick`.
export function onItemClick(item: HTMLElement, n: TreeNode, keys: () => string[], pick: () => void, ignore?: (ev: MouseEvent) => boolean): void {
  item.addEventListener("click", (ev) => {
    if (ignore?.(ev)) return;
    if (ev.ctrlKey || ev.shiftKey) clickSelect(n.relParts, keys(), { ctrl: ev.ctrlKey, shift: ev.shiftKey });
    else pick();
  });
}
