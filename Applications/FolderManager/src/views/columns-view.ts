// Columns view: one column per depth of the selected path, children of the selection in the next one.
import { folderContextMenu, isMenuKey, nodeContextMenu, nodeMenuFromKey } from "./context-menu";
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { folderIcon } from "../ui/icons";
import { statusMark } from "../ui/status-mark";
import { favoriteMark, nameCell } from "./name-cell";
import { nodeSummary } from "../store/node-info";
import { sortedForColumns } from "../store/sort";
import { expand } from "../actions/ops";
import { findNode, keyOf, listed, select, selectedNode, sortOf, startEdit, state } from "../store/state";
import type { TreeNode } from "../ipc/types";
import type { ViewMode } from "./view-mode";

interface Column {
  nodes: TreeNode[];
  loading: boolean;
}

function buildColumns(): Column[] {
  const cols: Column[] = [];
  const path = state.selectedKey?.split("/") ?? [];
  let nodes: TreeNode[] = state.tree?.nodes ?? [];
  for (let depth = 0; ; depth++) {
    const shown = sortedForColumns(listed(nodes), sortOf(path.slice(0, depth).join("/")), state.settings.pinFavorites);
    cols.push({ nodes: shown, loading: false });
    const open = nodes.find((n) => n.name === path[depth]);
    if (!open || !open.hasChildren) break;
    if (open.children === null) {
      cols.push({ nodes: [], loading: true });
      break;
    }
    if (open.children.length === 0) break;
    nodes = open.children;
  }
  return cols;
}

export function createColumnsView(): ViewMode {
  const root = el("div", { class: "columns", role: "group", "aria-label": "Storage columns" });
  const loading = new Set<string>();

  function buildItem(n: TreeNode, onPath: boolean, tabbable: boolean): HTMLElement {
    const selected = keyOf(n.relParts) === state.selectedKey;
    const item = el("div", { class: "col-item", role: "option", tabindex: tabbable ? "0" : "-1" }, folderIcon(n, (selected || onPath) && n.hasChildren), nameCell(n, false));
    item.dataset.key = keyOf(n.relParts);
    item.setAttribute("aria-selected", String(selected));
    if (selected) item.classList.add("selected");
    else if (onPath) item.classList.add("on-path");
    if (n.ghost) item.classList.add("ghost");
    if (n.locked) {
      item.classList.add("locked");
      item.setAttribute("aria-disabled", "true");
    }
    item.classList.toggle("reserved", n.reserved);
    item.classList.toggle("rev", n.nodeType?.base === "REV");
    if (n.ghost) item.setAttribute("aria-label", `${n.name} (missing on disk)`);
    // Without a status the star takes the icon's place at the end of the row.
    if (n.favorite) item.append(favoriteMark());
    if (n.status) item.append(statusMark(n.status));
    item.title = nodeSummary(n);
    if (n.hasChildren) item.append(el("span", { class: "chevron", "aria-hidden": "true" }, guiIcon("chevronRight")));
    item.addEventListener("click", () => select(n.relParts));
    item.addEventListener("dblclick", () => startEdit(n.relParts));
    item.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, root));
    return item;
  }

  // The folder whose children fill the column; the first column is the storage root.
  const parentOf = (path: string[], index: number): TreeNode | null => (index === 0 ? null : findNode(path.slice(0, index)));

  function buildColumn(col: Column, index: number, path: string[], parent: string): HTMLElement {
    if (col.loading) return el("div", { class: "column loading", "aria-busy": "true" }, "Loading…");
    const column = el("div", { class: "column", role: "listbox", "aria-label": `Contents of ${parent}` });
    const parentNode = parentOf(path, index);
    // The empty area of a column is a drop target for the folder it lists; "" is the storage root.
    column.dataset.key = parentNode ? keyOf(parentNode.relParts) : "";
    column.addEventListener("contextmenu", (ev) => folderContextMenu(ev, parentNode, root));
    const first = col.nodes[0];
    const tabbableKey = state.selectedKey ?? (index === 0 && first ? keyOf(first.relParts) : null);
    column.append(...col.nodes.map((n) => buildItem(n, n.name === path[index] && index < path.length - 1, keyOf(n.relParts) === tabbableKey)));
    return column;
  }

  async function enterChildren(n: TreeNode): Promise<void> {
    if (n.children === null) await expand(n);
    const first = sortedForColumns(listed(n.children ?? []), sortOf(keyOf(n.relParts)), state.settings.pinFavorites)[0];
    if (first) select(first.relParts);
  }

  function onKey(ev: KeyboardEvent): void {
    const n = selectedNode();
    if (isMenuKey(ev)) {
      if (n) nodeMenuFromKey(root, n, root.querySelector(".col-item.selected"));
      ev.preventDefault();
      return;
    }
    const cols = buildColumns();
    const siblings = (n ? cols[n.relParts.length - 1]?.nodes : cols[0]?.nodes) ?? [];
    const i = n ? siblings.indexOf(n) : -1;
    const go = (j: number) => {
      const target = siblings[Math.max(0, Math.min(siblings.length - 1, j))];
      if (target) select(target.relParts);
    };
    switch (ev.key) {
      case "ArrowDown":
        go(i + 1);
        break;
      case "ArrowUp":
        go(i < 0 ? 0 : i - 1);
        break;
      case "Home":
        go(0);
        break;
      case "End":
        go(siblings.length - 1);
        break;
      case "ArrowRight":
        if (n?.hasChildren) void enterChildren(n);
        break;
      case "ArrowLeft":
        if (n && n.relParts.length > 1) select(n.relParts.slice(0, -1));
        break;
      case "F2":
        if (n) startEdit(n.relParts);
        break;
      case "Enter":
      case " ":
        go(Math.max(i, 0));
        break;
      default:
        return;
    }
    ev.preventDefault();
  }

  // The snapshot only carries the first level, so deeper children are fetched on selection.
  function loadSelectedChildren(): void {
    const n = selectedNode();
    if (!n || !n.hasChildren || n.children !== null) return;
    const key = keyOf(n.relParts);
    if (loading.has(key)) return;
    loading.add(key);
    void expand(n).finally(() => loading.delete(key));
  }

  return {
    id: "columns",
    label: "Columns",
    mount(host) {
      root.addEventListener("keydown", onKey);
      // Space beside the columns belongs to the deepest one.
      root.addEventListener("contextmenu", (ev) => folderContextMenu(ev, parentOf(state.selectedKey?.split("/") ?? [], buildColumns().length - 1), root));
      host.append(root);
    },
    render() {
      const hadFocus = root.contains(document.activeElement);
      const path = state.selectedKey?.split("/") ?? [];
      root.replaceChildren(...buildColumns().map((c, i) => buildColumn(c, i, path, path[i - 1] ?? "storage")));
      root.querySelectorAll(".col-item.on-path, .col-item.selected").forEach((e) => e.scrollIntoView({ block: "nearest", inline: "nearest" }));
      root.lastElementChild?.scrollIntoView({ block: "nearest", inline: "nearest" });
      if (hadFocus) root.querySelector<HTMLElement>('.col-item[tabindex="0"]')?.focus({ preventScroll: true });
      loadSelectedChildren();
    },
    focus() {
      root.querySelector<HTMLElement>('.col-item[tabindex="0"]')?.focus();
    },
    destroy() {
      root.remove();
    },
  };
}
