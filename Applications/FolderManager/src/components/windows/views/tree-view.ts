// The storage tree of the navigation pane: indented expandable rows with keyboard navigation.
import { isWorknode } from "../../../services/store/node-info";
import { isMenuKey, nodeContextMenu, nodeMenuFromKey } from "../../organisms/menus/context-menu";
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { folderIcon } from "../../atoms/icons";
import { statusMark } from "../../atoms/status-mark";
import { hideBox, nameCell } from "../../molecules/name-cell";
import { fromToggle, nodeToggle } from "../../atoms/node-toggle";
import { toggleExpand } from "../../../hooks/ops";
import { keyOf, leaveVirtual, notify, select, startEdit, state, virtualRoot, visibleRows, type VirtualId } from "../../../services/store/state";
import { isUnfolded, unfoldedNodes, unfoldLoading } from "../../../services/store/virtual";
import { goToPlace, openVirtual, unfoldEntry } from "../../../hooks/virtual-nav";
import { leadEntries, tailEntries, virtualEntry, type VirtualEntry } from "../../molecules/virtual-entry";
import { markNodeItem } from "../../molecules/node-item";
import type { TreeNode } from "../../../services/ipc/types";
import type { ViewMode } from "./view-mode";

// Files stay in the views; the pane only charts where they live.
const isFolder = (n: TreeNode): boolean => !n.file;
// A second click on the marked row after the double-click window, and before this, is a slow double click.
const DOUBLE_MS = 500;
const SLOW_MS = 1600;

// A line of the pane: a root entry, or a folder with the entry whose unfolded list shows it (null in the storage tree) and the listed folder it sits under.
type Row = { node: TreeNode; depth: number; list: VirtualId | null; via: string | null };
type Item = { entry: VirtualEntry } | Row;

// ':' and '|' cannot occur in a folder name, so an entry's id never matches a row's, and a folder listed twice keeps two.
const idOf = (it: Item): string => ("entry" in it ? `entry:${it.entry.id}` : `${it.list ?? ""}|${keyOf(it.node.relParts)}`);

export function createTreeView(): ViewMode {
  const tree = el("div", { class: "tree", role: "tree", tabindex: "0", "aria-label": "Storage folders" });
  let items: Item[] = [];

  // A click only marks a line here; the views open a folder on a double click, as in Explorer's navigation pane.
  let marked: string | null = null;
  let lastClick = { id: "", at: 0 };

  // The marked line, else the one standing for what the views show.
  function current(): number {
    const at = marked === null ? -1 : items.findIndex((it) => idOf(it) === marked);
    if (at >= 0) return at;
    const key = state.selectedKey;
    if (key === null) return items.findIndex((it) => "entry" in it && it.entry.id === state.virtual);
    const listed = items.findIndex((it) => "node" in it && it.list === state.virtual && keyOf(it.node.relParts) === key);
    return listed >= 0 ? listed : items.findIndex((it) => "node" in it && it.list === null && keyOf(it.node.relParts) === key);
  }

  const mark = (i: number) => {
    const it = items[Math.max(0, Math.min(items.length - 1, i))];
    if (!it) return;
    marked = idOf(it);
    notify();
  };

  // A listed folder opens through its entry, so the views show it inside that list, as they do after a double click there.
  function openItem(it: Item): void {
    marked = null;
    if ("entry" in it) return void openVirtual(it.entry.id);
    const parts = it.node.relParts;
    if (it.list === null) {
      leaveVirtual();
      state.entering = keyOf(parts);
      if (it.node.hasChildren && !state.expanded.has(state.entering)) void toggleExpand(it.node);
      return select(parts);
    }
    const within = state.virtual === it.list && (virtualRoot(it.list) !== null || state.via === it.via || keyOf(parts) === it.via);
    if (within) select(parts);
    else void goToPlace({ virtual: it.list, via: it.via, key: keyOf(parts) });
  }

  // The line a folder folds back to: its parent row, or the entry that lists it.
  function parentOf(i: number): number {
    const it = items[i];
    if (!it || "entry" in it) return -1;
    if (it.list !== null && it.depth === 1) return items.findIndex((x) => "entry" in x && x.entry.id === it.list);
    const parent = keyOf(it.node.relParts.slice(0, -1));
    for (let j = i - 1; j >= 0; j--) {
      const x = items[j];
      if (x && "node" in x && x.list === it.list && keyOf(x.node.relParts) === parent) return j;
    }
    return -1;
  }

  function buildRow(it: Row, i: number): HTMLElement {
    const n = it.node;
    const key = keyOf(n.relParts);
    const id = idOf(it);
    // Once its contents are known, a folder holding only files has nothing to unfold here.
    const folds = n.hasChildren && (n.children === null || n.children.some(isFolder));
    const expanded = state.expanded.has(key);
    const caret = el("span", { class: "caret", "aria-hidden": "true" }, ...(folds ? [guiIcon(expanded ? "chevronDown" : "chevronRight")] : []));
    const icon = folderIcon(n, expanded && folds);
    const toggle = folds ? nodeToggle(`${expanded ? "Collapse" : "Expand"} ${n.name}`, expanded, caret) : null;
    const row = el("div", { class: "row", role: "treeitem", id: `row-${i}`, "aria-level": String(it.depth + 1) }, ...hideBox(n), toggle ?? caret, icon, ...(isWorknode(n) && n.status ? [statusMark(n.status)] : []), nameCell(n, true, true));
    row.style.setProperty("--depth", String(it.depth));
    markNodeItem(row, n);
    row.classList.toggle("marked", marked === id);
    if (folds) row.setAttribute("aria-expanded", String(expanded));
    if (n.status && !isWorknode(n)) row.append(statusMark(n.status));
    toggle?.addEventListener("click", () => void toggleExpand(n));
    // Its two clicks already toggled twice, so the double click ends open.
    toggle?.addEventListener("dblclick", () => {
      if (!state.expanded.has(key)) void toggleExpand(n);
    });
    row.addEventListener("click", (ev) => {
      if (fromToggle(ev)) return;
      const since = ev.timeStamp - lastClick.at;
      const slow = lastClick.id === id && marked === id && since > DOUBLE_MS && since < SLOW_MS;
      lastClick = { id, at: slow ? 0 : ev.timeStamp };
      // Only a slow second click on the name renames, as in Explorer.
      if (slow && n.actions.rename && ev.target instanceof Element && ev.target.closest(".name")) startEdit(n.relParts);
      else mark(i);
    });
    row.addEventListener("dblclick", (ev) => {
      if (!fromToggle(ev)) openItem(it);
    });
    row.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, tree));
    return row;
  }

  function buildEntry(e: VirtualEntry, i: number): HTMLElement {
    const open = isUnfolded(e.id);
    const item = virtualEntry(e, "row", "treeitem", { marker: "caret", mark: () => mark(i), unfold: { open, set: (on) => void unfoldEntry(e.id, on) } });
    item.id = `row-${i}`;
    item.setAttribute("aria-expanded", String(open));
    item.classList.toggle("marked", marked === `entry:${e.id}`);
    return item;
  }

  // An opened folder with nothing in it gets a line under its row, so the row does not look unfinished.
  function emptyLegend(n: TreeNode, depth: number): HTMLElement[] {
    if (n.children?.length !== 0 || !state.expanded.has(keyOf(n.relParts))) return [];
    const legend = el("div", { class: "tree-legend" }, "Empty");
    legend.style.setProperty("--depth", String(depth + 1));
    return [legend];
  }

  function listLegend(e: VirtualEntry): HTMLElement[] {
    if (!isUnfolded(e.id) || unfoldedNodes(e.id).some(isFolder)) return [];
    const legend = el("div", { class: "tree-legend" }, unfoldLoading(e.id) ? "Loading…" : e.empty);
    legend.style.setProperty("--depth", "1");
    return [legend];
  }

  function listOf(e: VirtualEntry): Item[] {
    if (!isUnfolded(e.id)) return [];
    return unfoldedNodes(e.id).filter(isFolder).flatMap((n) => {
      const via = keyOf(n.relParts);
      const below = state.expanded.has(via) && n.children ? visibleRows(n.children).filter(isFolder) : [];
      return [{ node: n, depth: 1, list: e.id, via }, ...below.map((b) => ({ node: b, depth: 1 + b.depth - n.depth, list: e.id, via }))];
    });
  }

  // Right unfolds and Left folds an entry or a folder; Enter opens it in the views.
  function onKey(ev: KeyboardEvent): void {
    // Modified arrows belong to the global shortcuts (Ctrl+Right, Alt+Down, Alt+Up).
    if (ev.ctrlKey || ev.altKey || ev.metaKey) return;
    const i = current();
    const it = items[i];
    const entry = it && "entry" in it ? it.entry : undefined;
    const n = it && "node" in it ? it.node : undefined;
    const open = n !== undefined && state.expanded.has(keyOf(n.relParts));
    if (isMenuKey(ev)) {
      if (n) nodeMenuFromKey(tree, n, tree.querySelector(`#row-${i}`));
      ev.preventDefault();
      return;
    }
    switch (ev.key) {
      case "ArrowDown":
        mark(i + 1);
        break;
      case "ArrowUp":
        mark(i < 0 ? 0 : i - 1);
        break;
      case "Home":
        mark(0);
        break;
      case "End":
        mark(items.length - 1);
        break;
      case "ArrowRight": {
        const next = items[i + 1];
        if (entry && !isUnfolded(entry.id)) void unfoldEntry(entry.id, true);
        else if (entry && next && "node" in next && next.list === entry.id) mark(i + 1);
        else if (n && n.hasChildren && !open) void toggleExpand(n);
        else if (open) mark(i + 1);
        break;
      }
      case "ArrowLeft":
        if (entry) void unfoldEntry(entry.id, false);
        else if (n && open) void toggleExpand(n);
        else if (parentOf(i) >= 0) mark(parentOf(i));
        break;
      case "Enter":
        if (it) openItem(it);
        break;
      case "F2":
        if (n) startEdit(n.relParts);
        break;
      default:
        return;
    }
    ev.preventDefault();
  }

  let revealed: string | null = null;

  return {
    id: "tree",
    label: "Tree",
    mount(host) {
      tree.addEventListener("keydown", onKey);
      host.append(tree);
    },
    render() {
      const group = (e: VirtualEntry): Item[] => [{ entry: e }, ...listOf(e)];
      const own = visibleRows().filter(isFolder).map((n): Item => ({ node: n, depth: n.depth, list: null, via: null }));
      // Quick access heads the pane, as in Explorer: Favorites, Downloads and Review, then the storage tree, then the other native folders.
      items = [...leadEntries().flatMap(group), ...own, ...tailEntries().flatMap(group)];
      const scroller = tree.parentElement;
      const top = scroller?.scrollTop ?? 0;
      tree.replaceChildren(...items.flatMap((it, i) => ("entry" in it ? [buildEntry(it.entry, i), ...listLegend(it.entry)] : [buildRow(it, i), ...emptyLegend(it.node, it.depth)])));
      if (scroller) scroller.scrollTop = top;
      const i = current();
      if (i >= 0) {
        tree.setAttribute("aria-activedescendant", `row-${i}`);
        // Only a new selection is brought into view; expanding or collapsing rows leaves the scroll where the user put it.
        const shown = `${state.virtual}|${state.selectedKey}`;
        if (shown !== revealed) tree.querySelector(`#row-${i}`)?.scrollIntoView({ block: "nearest" });
        revealed = shown;
      } else {
        tree.removeAttribute("aria-activedescendant");
      }
    },
    focus() {
      tree.focus({ preventScroll: true });
    },
    destroy() {
      tree.remove();
    },
  };
}
