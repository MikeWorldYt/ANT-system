// Grid view: large icons for the contents of one folder, entered with double click or Enter.
import { isMenuKey, nodeContextMenu, nodeMenuFromKey } from "./context-menu";
import { el } from "../ui/dom";
import { folderIcon } from "../ui/icons";
import { statusMark } from "../ui/status-mark";
import { favoriteMark, nameCell } from "./name-cell";
import { nodeSummary } from "../store/node-info";
import { sortedForColumns } from "../store/sort";
import { expand } from "../actions/ops";
import { findNode, keyOf, listed, select, selectedNode, sortOf, startEdit, state } from "../store/state";
import type { TreeNode } from "../ipc/types";
import type { ViewMode } from "./view-mode";

export function createGridView(): ViewMode {
  const root = el("div", { class: "grid", role: "listbox", "aria-label": "Folder contents" });
  const loading = new Set<string>();
  // Key of the folder being shown; null is the Storage root.
  let folder: string | null = null;

  // Selecting inside the folder keeps it; any other selection (breadcrumb, back, search) moves the grid there.
  function syncFolder(): void {
    const key = state.selectedKey;
    if (key === null) {
      folder = null;
      return;
    }
    const parts = key.split("/");
    const parent = parts.length > 1 ? parts.slice(0, -1).join("/") : null;
    if (key === folder || parent === folder) return;
    folder = selectedNode()?.hasChildren ? key : parent;
  }

  function folderNode(): TreeNode | null {
    return folder === null ? null : findNode(folder.split("/"));
  }

  function shownNodes(): TreeNode[] {
    const open = folderNode();
    const base = folder === null ? (state.tree?.nodes ?? []) : (open?.children ?? []);
    return sortedForColumns(listed(base), sortOf(folder ?? ""), state.settings.pinFavorites);
  }

  // The snapshot only carries the first level, so the folder's children are fetched when it is shown.
  function loadChildren(): void {
    const open = folderNode();
    if (!open || !open.hasChildren || open.children !== null || folder === null || loading.has(folder)) return;
    const key = folder;
    loading.add(key);
    void expand(open).finally(() => loading.delete(key));
  }

  function enter(n: TreeNode): void {
    if (!n.hasChildren) return;
    folder = keyOf(n.relParts);
    select(n.relParts);
  }

  function buildItem(n: TreeNode, tabbable: boolean): HTMLElement {
    const selected = keyOf(n.relParts) === state.selectedKey;
    const art = el("span", { class: "grid-art" }, folderIcon(n));
    if (n.favorite) art.append(favoriteMark());
    if (n.status) art.append(statusMark(n.status));
    const item = el("div", { class: "grid-item", role: "option", tabindex: tabbable ? "0" : "-1" }, art, nameCell(n, false));
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
    item.title = nodeSummary(n);
    item.addEventListener("click", () => select(n.relParts));
    item.addEventListener("dblclick", () => (n.hasChildren ? enter(n) : startEdit(n.relParts)));
    item.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, root));
    return item;
  }

  function perRow(): number {
    const items = [...root.querySelectorAll<HTMLElement>(".grid-item")];
    const top = items[0]?.offsetTop;
    return Math.max(1, items.filter((i) => i.offsetTop === top).length);
  }

  function onKey(ev: KeyboardEvent): void {
    const n = selectedNode();
    if (isMenuKey(ev)) {
      if (n) nodeMenuFromKey(root, n, root.querySelector(".grid-item.selected"));
      ev.preventDefault();
      return;
    }
    const nodes = shownNodes();
    const i = n ? nodes.indexOf(n) : -1;
    const go = (j: number) => {
      const target = nodes[Math.max(0, Math.min(nodes.length - 1, j))];
      if (target) select(target.relParts);
    };
    switch (ev.key) {
      case "ArrowRight":
        go(i + 1);
        break;
      case "ArrowLeft":
        go(i < 0 ? 0 : i - 1);
        break;
      case "ArrowDown":
        go(i < 0 ? 0 : i + perRow());
        break;
      case "ArrowUp":
        go(i < 0 ? 0 : i - perRow());
        break;
      case "Home":
        go(0);
        break;
      case "End":
        go(nodes.length - 1);
        break;
      case "F2":
        if (n) startEdit(n.relParts);
        break;
      case "Enter":
        if (i < 0) go(0);
        else if (n) enter(n);
        break;
      default:
        return;
    }
    ev.preventDefault();
  }

  return {
    id: "grid",
    label: "Grid",
    mount(host) {
      root.addEventListener("keydown", onKey);
      host.append(root);
    },
    render() {
      syncFolder();
      const hadFocus = root.contains(document.activeElement);
      const nodes = shownNodes();
      const tabbable = nodes.find((n) => keyOf(n.relParts) === state.selectedKey) ?? nodes[0];
      root.replaceChildren(...nodes.map((n) => buildItem(n, n === tabbable)));
      root.querySelector(".grid-item.selected")?.scrollIntoView({ block: "nearest" });
      if (hadFocus) root.querySelector<HTMLElement>('.grid-item[tabindex="0"]')?.focus({ preventScroll: true });
      loadChildren();
    },
    focus() {
      root.querySelector<HTMLElement>('.grid-item[tabindex="0"]')?.focus();
    },
    destroy() {
      root.remove();
    },
  };
}
