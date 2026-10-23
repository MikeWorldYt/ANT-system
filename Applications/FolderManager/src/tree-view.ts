// Tree view: indented expandable rows with keyboard navigation.
import { nodeSummary } from "./node-info";
import { isMenuKey, nodeContextMenu, nodeMenuFromKey } from "./context-menu";
import { badge, el } from "./dom";
import { guiIcon } from "./gui-icons";
import { folderIcon } from "./icons";
import { nameCell } from "./name-cell";
import { toggleExpand } from "./ops";
import { keyOf, select, startEdit, state, visibleRows } from "./state";
import type { TreeNode } from "./types";
import type { ViewMode } from "./view-mode";

export function createTreeView(): ViewMode {
  const tree = el("div", { class: "tree", role: "tree", tabindex: "0", "aria-label": "Storage folders" });
  let rows: TreeNode[] = [];

  const indexOfSelected = () => rows.findIndex((n) => keyOf(n.relParts) === state.selectedKey);
  const pick = (i: number) => {
    const n = rows[Math.max(0, Math.min(rows.length - 1, i))];
    if (n) select(n.relParts);
  };

  function buildRow(n: TreeNode, i: number): HTMLElement {
    const key = keyOf(n.relParts);
    const expanded = state.expanded.has(key);
    const caret = el("span", { class: "caret", "aria-hidden": "true" }, ...(n.hasChildren ? [guiIcon(expanded ? "chevronDown" : "chevronRight")] : []));
    const row = el("div", { class: "row", role: "treeitem", id: `row-${i}`, "aria-level": String(n.depth + 1) }, caret, folderIcon(n), nameCell(n));
    row.style.setProperty("--depth", String(n.depth));
    row.setAttribute("aria-selected", String(key === state.selectedKey));
    if (n.hasChildren) row.setAttribute("aria-expanded", String(expanded));
    if (key === state.selectedKey) row.classList.add("selected");
    if (n.ghost) row.classList.add("ghost");
    if (n.locked) row.classList.add("locked");
    if (n.status) row.append(badge(`status st-${n.status}`, n.status));
    row.title = nodeSummary(n);
    caret.addEventListener("click", (ev) => {
      ev.stopPropagation();
      select(n.relParts);
      void toggleExpand(n);
    });
    row.addEventListener("click", () => select(n.relParts));
    row.addEventListener("dblclick", () => startEdit(n.relParts));
    row.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, tree));
    return row;
  }

  function onKey(ev: KeyboardEvent): void {
    const i = indexOfSelected();
    const n = rows[i];
    const open = n !== undefined && state.expanded.has(keyOf(n.relParts));
    if (isMenuKey(ev)) {
      if (n) nodeMenuFromKey(tree, n, tree.querySelector(`#row-${i}`));
      ev.preventDefault();
      return;
    }
    switch (ev.key) {
      case "ArrowDown":
        pick(i + 1);
        break;
      case "ArrowUp":
        pick(i < 0 ? 0 : i - 1);
        break;
      case "Home":
        pick(0);
        break;
      case "End":
        pick(rows.length - 1);
        break;
      case "ArrowRight":
        if (n && n.hasChildren && !open) void toggleExpand(n);
        else if (open) pick(i + 1);
        break;
      case "ArrowLeft":
        if (n && open) void toggleExpand(n);
        else if (n && n.relParts.length > 1) select(n.relParts.slice(0, -1));
        break;
      case "Enter":
        if (n) void toggleExpand(n);
        break;
      case "F2":
        if (n) startEdit(n.relParts);
        break;
      default:
        return;
    }
    ev.preventDefault();
  }

  return {
    id: "tree",
    label: "Tree",
    mount(host) {
      tree.addEventListener("keydown", onKey);
      host.append(tree);
    },
    render() {
      rows = visibleRows();
      tree.replaceChildren(...rows.map(buildRow));
      const i = indexOfSelected();
      if (i >= 0) {
        tree.setAttribute("aria-activedescendant", `row-${i}`);
        tree.querySelector(`#row-${i}`)?.scrollIntoView({ block: "nearest" });
      } else {
        tree.removeAttribute("aria-activedescendant");
      }
    },
    focus() {
      tree.focus();
    },
    destroy() {
      tree.remove();
    },
  };
}
