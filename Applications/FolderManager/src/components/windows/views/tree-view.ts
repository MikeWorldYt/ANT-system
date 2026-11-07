// Tree view: indented expandable rows with keyboard navigation.
import { isWorkboard } from "../../../services/store/node-info";
import { isMenuKey, nodeContextMenu, nodeMenuFromKey } from "../../organisms/menus/context-menu";
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { folderIcon } from "../../atoms/icons";
import { statusMark } from "../../atoms/status-mark";
import { hideBox, nameCell } from "../../molecules/name-cell";
import { fromToggle, nodeToggle } from "../../atoms/node-toggle";
import { toggleExpand } from "../../../hooks/ops";
import { keyOf, leaveVirtual, select, startEdit, state, visibleRows } from "../../../services/store/state";
import { virtualNodes } from "../../../services/store/virtual";
import { openVirtual } from "../../../hooks/virtual-nav";
import { entryBeside, headEntry, rootEntries, virtualEntry } from "../../molecules/virtual-entry";
import { editOrOpen, markNodeItem, onItemClick } from "../../molecules/node-item";
import type { TreeNode } from "../../../services/ipc/types";
import { openFolder } from "../../../hooks/path-actions";
import type { ViewMode } from "./view-mode";

export function createTreeView(): ViewMode {
  const tree = el("div", { class: "tree", role: "tree", tabindex: "0", "aria-label": "Storage folders" });
  let rows: TreeNode[] = [];
  // Rows of an open virtual list, with what unfolds under them, come first in `rows`; they stand for folders that live elsewhere.
  let listCount = 0;
  let listDepths: number[] = [];

  const indexOfSelected = () => rows.findIndex((n) => keyOf(n.relParts) === state.selectedKey);
  // The same folder may also show in the real tree below the list; acting there leaves the list.
  const leaveAt = (i: number) => {
    if (i >= listCount) leaveVirtual();
  };
  const pick = (i: number) => {
    const at = Math.max(0, Math.min(rows.length - 1, i));
    const n = rows[at];
    if (!n) return;
    leaveAt(at);
    select(n.relParts);
  };

  function buildRow(n: TreeNode, i: number): HTMLElement {
    const key = keyOf(n.relParts);
    const folds = n.hasChildren;
    const depth = listDepths[i] ?? n.depth;
    const expanded = state.expanded.has(key);
    const caret = el("span", { class: "caret", "aria-hidden": "true" }, ...(folds ? [guiIcon(expanded ? "chevronDown" : "chevronRight")] : []));
    const icon = folderIcon(n, expanded && folds);
    const toggle = folds ? nodeToggle(`${expanded ? "Collapse" : "Expand"} ${n.name}`, expanded, caret, icon) : null;
    const row = el("div", { class: "row", role: "treeitem", id: `row-${i}`, "aria-level": String(depth + 1) }, ...hideBox(n), ...(toggle ? [toggle] : [caret, icon]), ...(isWorkboard(n) && n.status ? [statusMark(n.status)] : []), nameCell(n, true, true));
    row.style.setProperty("--depth", String(depth));
    markNodeItem(row, n);
    if (folds) row.setAttribute("aria-expanded", String(expanded));
    if (n.status && !isWorkboard(n)) row.append(statusMark(n.status));
    toggle?.addEventListener("click", () => {
      leaveAt(i);
      select(n.relParts);
      void toggleExpand(n);
    });
    // Its two clicks already toggled twice, so the double click ends open.
    toggle?.addEventListener("dblclick", () => {
      if (!state.expanded.has(key)) void toggleExpand(n);
    });
    onItemClick(row, n, () => rows.map((r) => keyOf(r.relParts)), () => pick(i), fromToggle);
    row.addEventListener("dblclick", (ev) => {
      if (!fromToggle(ev)) editOrOpen(n, ev);
    });
    row.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, tree));
    return row;
  }

  // An opened folder with nothing in it gets a line under its row, so the row does not look unfinished.
  function emptyLegend(n: TreeNode, depth: number): HTMLElement[] {
    if (n.children?.length !== 0 || !state.expanded.has(keyOf(n.relParts))) return [];
    const legend = el("div", { class: "tree-legend" }, "Empty");
    legend.style.setProperty("--depth", String(depth + 1));
    return [legend];
  }

  function listLegend(text: string): HTMLElement {
    const legend = el("div", { class: "tree-legend" }, text);
    legend.style.setProperty("--depth", "1");
    return legend;
  }

  function onKey(ev: KeyboardEvent): void {
    // Modified arrows belong to the global shortcuts (Ctrl+Right, Alt+Down, Alt+Up).
    if (ev.ctrlKey || ev.altKey || ev.metaKey) return;
    const i = indexOfSelected();
    const n = rows[i];
    const open = n !== undefined && state.expanded.has(keyOf(n.relParts));
    const head = headEntry();
    // The entry rows precede every other row, so with nothing selected the keys act on the current one.
    const onHead = head !== undefined && n === undefined && state.selectedKey === null && state.virtual !== "board";
    const headOpen = head !== undefined && state.virtual === head.id;
    const before = entryBeside(head, -1);
    const after = entryBeside(head, 1);
    if (isMenuKey(ev)) {
      if (n) nodeMenuFromKey(tree, n, tree.querySelector(`#row-${i}`));
      ev.preventDefault();
      return;
    }
    switch (ev.key) {
      case "ArrowDown":
        if (onHead && after && (!headOpen || listCount === 0)) void openVirtual(after.id);
        else pick(i + 1);
        break;
      case "ArrowUp":
        if (onHead) {
          if (before) void openVirtual(before.id);
        } else if (head && i === 0) void openVirtual(listCount > 0 ? head.id : (rootEntries().at(-1)?.id ?? head.id));
        else pick(i < 0 ? 0 : i - 1);
        break;
      case "Home":
        pick(0);
        break;
      case "End":
        pick(rows.length - 1);
        break;
      case "ArrowRight":
        if (head && onHead) {
          if (!headOpen) void openVirtual(head.id);
          else if (listCount > 0) pick(0);
        } else if (n && n.hasChildren && !open) void toggleExpand(n);
        else if (open) pick(i + 1);
        break;
      case "ArrowLeft":
        if (onHead) {
          if (headOpen) select(null);
        } else if (n && open) void toggleExpand(n);
        // A listed entry steps back to its virtual list, not to the real folder it lives in.
        else if (i >= 0 && listDepths[i] === 1 && state.virtual !== null) void openVirtual(state.virtual);
        else if (n && n.relParts.length > 1) {
          leaveAt(i);
          select(n.relParts.slice(0, -1));
        }
        break;
      case "Enter":
        if (n?.file) void openFolder(n);
        else if (n) void toggleExpand(n);
        else if (head && onHead) void (headOpen ? select(null) : openVirtual(head.id));
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
      const list: TreeNode[] = [];
      listDepths = [];
      for (const n of state.virtual !== null ? virtualNodes() : []) {
        const below = state.expanded.has(keyOf(n.relParts)) && n.children ? visibleRows(n.children) : [];
        list.push(n, ...below);
        listDepths.push(1, ...below.map((b) => 1 + b.depth - n.depth));
      }
      listCount = list.length;
      rows = [...list, ...visibleRows()];
      const listRows = list.flatMap((n, i) => [buildRow(n, i), ...emptyLegend(n, listDepths[i] ?? 1)]);
      const entries = rootEntries().flatMap((e) => {
        const open = state.virtual === e.id;
        const entry = virtualEntry(e, "row", "treeitem", { toggle: true, marker: "caret" });
        entry.id = `row-${e.id}`;
        entry.setAttribute("aria-expanded", String(open));
        if (!open) return [entry];
        return [entry, ...(listRows.length > 0 ? listRows : [listLegend(e.empty)])];
      });
      tree.replaceChildren(...entries, ...rows.slice(listCount).flatMap((n, i) => [buildRow(n, i + listCount), ...emptyLegend(n, n.depth)]));
      const i = indexOfSelected();
      const head = rootEntries().find((e) => state.virtual === e.id);
      if (i >= 0) {
        tree.setAttribute("aria-activedescendant", `row-${i}`);
        tree.querySelector(`#row-${i}`)?.scrollIntoView({ block: "nearest" });
      } else if (head && state.selectedKey === null) {
        tree.setAttribute("aria-activedescendant", `row-${head.id}`);
        tree.querySelector(`#row-${head.id}`)?.scrollIntoView({ block: "nearest" });
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
