// Tree view: indented expandable rows with keyboard navigation.
import { holdsFiles, isWorkboard, nodeSummary } from "../store/node-info";
import { isMenuKey, nodeContextMenu, nodeMenuFromKey } from "./context-menu";
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { folderIcon } from "../ui/icons";
import { statusMark } from "../ui/status-mark";
import { hideBox, nameCell } from "./name-cell";
import { fromToggle, nodeToggle } from "./node-toggle";
import { toggleExpand } from "../actions/ops";
import { keyOf, select, startEdit, state, visibleRows } from "../store/state";
import type { TreeNode } from "../ipc/types";
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
    const icon = folderIcon(n, expanded && n.hasChildren);
    const toggle = n.hasChildren ? nodeToggle(`${expanded ? "Collapse" : "Expand"} ${n.name}`, expanded, caret, icon) : null;
    const row = el("div", { class: "row", role: "treeitem", id: `row-${i}`, "aria-level": String(n.depth + 1) }, ...hideBox(n), ...(toggle ? [toggle] : [caret, icon]), ...(isWorkboard(n) && n.status ? [statusMark(n.status)] : []), nameCell(n, true, true));
    row.style.setProperty("--depth", String(n.depth));
    row.dataset.key = key;
    row.setAttribute("aria-selected", String(key === state.selectedKey));
    if (n.hasChildren) row.setAttribute("aria-expanded", String(expanded));
    if (key === state.selectedKey) row.classList.add("selected");
    if (n.ghost) row.classList.add("ghost");
    if (n.locked) {
      row.classList.add("locked");
      row.setAttribute("aria-disabled", "true");
    }
    row.classList.toggle("reserved", n.reserved);
    row.classList.toggle("rev", n.nodeType?.base === "REV");
    if (n.ghost) row.setAttribute("aria-label", `${n.name} (missing on disk)`);
    if (n.status && !isWorkboard(n)) row.append(statusMark(n.status));
    row.title = nodeSummary(n);
    toggle?.addEventListener("click", () => {
      select(n.relParts);
      void toggleExpand(n);
    });
    row.addEventListener("click", (ev) => {
      if (!fromToggle(ev)) select(n.relParts);
    });
    row.addEventListener("dblclick", (ev) => {
      if (!fromToggle(ev)) startEdit(n.relParts);
    });
    row.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, tree));
    return row;
  }

  // An opened review folder with nothing in it gets a line under its row, so the row does not look unfinished.
  function emptyLegend(n: TreeNode): HTMLElement[] {
    if (!holdsFiles(n) || n.children?.length !== 0 || !state.expanded.has(keyOf(n.relParts))) return [];
    const legend = el("div", { class: "tree-legend" }, "Empty");
    legend.style.setProperty("--depth", String(n.depth + 1));
    return [legend];
  }

  function onKey(ev: KeyboardEvent): void {
    // Modified arrows belong to the global shortcuts (Ctrl+Right, Alt+Down, Alt+Up).
    if (ev.ctrlKey || ev.altKey || ev.metaKey) return;
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
      tree.replaceChildren(...rows.flatMap((n, i) => [buildRow(n, i), ...emptyLegend(n)]));
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
