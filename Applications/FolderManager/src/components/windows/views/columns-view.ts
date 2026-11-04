// Columns view: one column per depth of the selected path, children of an opened selection in the next one.
import { folderContextMenu, isMenuKey, nodeContextMenu, nodeMenuFromKey } from "../../organisms/menus/context-menu";
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { folderIcon } from "../../atoms/icons";
import { statusMark } from "../../atoms/status-mark";
import { columnSortToggle } from "../../atoms/column-sort";
import { favoriteMark, hideBox, importantMark, nameCell } from "../../molecules/name-cell";
import { fromToggle, nodeToggle } from "../../atoms/node-toggle";
import { hiddenCategories, isReviewRoot, isWorkboard, linkCategory } from "../../../services/store/node-info";
import { sortedForColumns } from "../../../services/store/sort";
import { buildColumns, type Column } from "../../../hooks/column-model";
import { expand } from "../../../hooks/ops";
import { isManagedFolder, manageBox, manageHeader, managedNodes, manageSnapshot, sameManage, type ManageSnapshot } from "../../molecules/manage-row";
import { findNode, keyOf, leaveVirtual, listed, select, selectedNode, sortOf, startEdit, state } from "../../../services/store/state";
import { virtualNodes } from "../../../services/store/virtual";
import { openVirtual } from "../../../hooks/virtual-nav";
import { openEntry, ROOT_ENTRIES, virtualEntries } from "../../molecules/virtual-entry";
import { markNodeItem, onItemClick } from "../../molecules/node-item";
import type { TreeNode } from "../../../services/ipc/types";
import type { ViewMode } from "./view-mode";

// Why an empty column is empty: a filter hides the review list, or the folder has no content.
function emptyLegend(parent: TreeNode | null): string | null {
  if (parent === null) return null;
  if (isReviewRoot(parent)) return hiddenCategories(parent).length > 0 ? "No active filters" : null;
  return "Empty";
}

export function createColumnsView(): ViewMode {
  const root = el("div", { class: "columns", role: "group", "aria-label": "Storage columns" });
  const loading = new Set<string>();
  let openKey: string | null = null;
  let built: ManageSnapshot | null = null;
  let revealed: string | null = null;

  function isOpen(n: TreeNode): boolean {
    const key = keyOf(n.relParts);
    const sel = state.selectedKey;
    return n.hasChildren && (sel === key ? openKey === key : (sel?.startsWith(`${key}/`) ?? false));
  }

  function openColumn(n: TreeNode): void {
    openKey = keyOf(n.relParts);
    select(n.relParts);
  }

  function pick(n: TreeNode): void {
    if (n.hasChildren && state.settings.autoOpenColumns) openColumn(n);
    else select(n.relParts);
  }

  function buildItem(n: TreeNode, onPath: boolean, tabbable: boolean, siblings: string[]): HTMLElement {
    const open = isOpen(n);
    const icon = folderIcon(n, open);
    const lead = n.hasChildren ? nodeToggle(`${open ? "Close" : "Open"} ${n.name}`, open, icon) : icon;
    // A workboard shows its status before its name; other rows keep the status after the star.
    const item = el("div", { class: "col-item", role: "option", tabindex: tabbable ? "0" : "-1" }, ...hideBox(n), lead, ...(isWorkboard(n) && n.status ? [statusMark(n.status)] : []), nameCell(n, false, true));
    markNodeItem(item, n);
    item.prepend(...manageBox(n, item));
    if (onPath && !item.classList.contains("selected")) item.classList.add("on-path");
    // Without a status the star takes the icon's place at the end of the row.
    if (n.favorite) item.append(favoriteMark());
    if (n.important) item.append(importantMark());
    if (n.status && !isWorkboard(n)) item.append(statusMark(n.status));
    if (n.hasChildren) {
      lead.addEventListener("dblclick", () => openColumn(n));
      const chevron = nodeToggle(`Show contents of ${n.name}`, null, guiIcon("chevronRight"));
      chevron.classList.add("chevron");
      chevron.addEventListener("dblclick", () => openColumn(n));
      item.append(chevron);
    }
    onItemClick(item, n, () => siblings, () => pick(n));
    item.addEventListener("dblclick", (ev) => {
      if (!fromToggle(ev)) startEdit(n.relParts);
    });
    item.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, root));
    return item;
  }

  function buildColumn(col: Column, index: number): HTMLElement {
    if (col.loading) return el("div", { class: "column loading", "aria-busy": "true" }, "Loading…");
    const listing = col.listing ? openEntry() : null;
    const column = el("div", { class: "column", role: "listbox", "aria-label": listing ? listing.label : `Contents of ${col.parent?.name ?? "storage"}` });
    const parentNode = col.parent;
    // The review list holds long link names, so it gets twice the width of an ordinary column.
    column.classList.toggle("review-list", parentNode !== null && isReviewRoot(parentNode));
    // The empty area of a column is a drop target for the folder it lists; "" is the storage root. A virtual list is no folder.
    if (!listing) column.dataset.key = parentNode ? keyOf(parentNode.relParts) : "";
    column.addEventListener("contextmenu", (ev) => folderContextMenu(ev, parentNode, root));
    const managed = !listing && isManagedFolder(column.dataset.key ?? "");
    const nodes = managed ? (managedNodes() ?? col.nodes) : col.nodes;
    const onPath = col.pathKey !== state.selectedKey ? nodes.find((n) => keyOf(n.relParts) === col.pathKey) : undefined;
    // The empty area of a column steps back to its row on the selected path.
    column.addEventListener("click", (ev) => {
      if (!onPath || (ev.target instanceof Element && ev.target.closest(".col-item"))) return;
      pick(onPath);
      // The column background cannot take focus, so keys go back to the row that now holds it.
      root.querySelector<HTMLElement>('.col-item[tabindex="0"]')?.focus({ preventScroll: true });
    });
    // With nothing selected the FAVORITES row heading the root column holds the roving focus.
    const tabbableKey = state.selectedKey;
    // Not an item, so it cannot be selected; it only tells why the review list is empty.
    const legend = nodes.length === 0 ? (listing?.empty ?? emptyLegend(parentNode)) : null;
    const siblings = nodes.map((n) => keyOf(n.relParts));
    const rows = nodes.flatMap((n, i) => {
      const prev = nodes[i - 1];
      // Review links are grouped by category, so a change of category gets a line between the groups.
      const groupStart = prev?.linked && n.linked && linkCategory(prev) !== linkCategory(n);
      const item = buildItem(n, n === onPath, keyOf(n.relParts) === tabbableKey, siblings);
      return groupStart ? [el("div", { class: "col-sep", role: "separator" }), item] : [item];
    });
    column.append(
      ...(listing ? [] : [columnSortToggle(column.dataset.key ?? "")]),
      ...(managed ? [manageHeader("col-item")] : []),
      ...(index === 0 ? virtualEntries("col-item", "option", { tabbable: state.selectedKey === null, marker: "chevron" }) : []),
      ...rows,
      ...(legend ? [el("p", { class: "column-legend" }, legend)] : []),
    );
    return column;
  }

  async function enterChildren(n: TreeNode): Promise<void> {
    if (n.children === null) await expand(n);
    const first = sortedForColumns(listed(n.children ?? []), sortOf(keyOf(n.relParts)), state.settings.pinFavorites)[0];
    if (first) select(first.relParts);
  }

  function onKey(ev: KeyboardEvent): void {
    // Modified arrows belong to the global shortcuts (Ctrl+Right, Alt+Down, Alt+Up).
    if (ev.ctrlKey || ev.altKey || ev.metaKey) return;
    // A focused hide box keeps its own Space.
    if (ev.target instanceof HTMLInputElement) return;
    const n = selectedNode();
    if (isMenuKey(ev)) {
      if (n) nodeMenuFromKey(root, n, root.querySelector(".col-item.selected"));
      ev.preventDefault();
      return;
    }
    const cols = buildColumns(openKey);
    const head = ROOT_ENTRIES[0];
    // FAVORITES heads the root column, so with nothing selected the arrows start from it.
    if (n === null && head && state.virtual !== "board") {
      const first = cols[0]?.nodes[0];
      switch (ev.key) {
        case "ArrowDown":
          if (first) {
            leaveVirtual();
            pick(first);
          }
          break;
        case "ArrowRight": {
          const favorite = state.virtual === head.id ? virtualNodes()[0] : undefined;
          if (favorite) select(favorite.relParts);
          else void openVirtual(head.id);
          break;
        }
        default:
          return;
      }
      ev.preventDefault();
      return;
    }
    // A folder listed by a virtual entry also sits in its real column, so the later column is the one the selection is in.
    const siblings = (n ? [...cols].reverse().find((c) => c.nodes.includes(n)) : cols[cols.length - 1])?.nodes ?? [];
    const i = n ? siblings.indexOf(n) : -1;
    const at = (j: number): TreeNode | undefined => siblings[Math.max(0, Math.min(siblings.length - 1, j))];
    const go = (j: number) => {
      const target = at(j);
      if (target) select(target.relParts);
    };
    const move = (j: number) => {
      const target = at(j);
      if (target) pick(target);
    };
    switch (ev.key) {
      case "ArrowDown":
        move(i + 1);
        break;
      case "ArrowUp":
        if (head && i === 0 && siblings === cols[0]?.nodes) void openVirtual(head.id);
        else move(i < 0 ? 0 : i - 1);
        break;
      case "Home":
        move(0);
        break;
      case "End":
        move(siblings.length - 1);
        break;
      case "ArrowRight":
        if (n?.hasChildren) void enterChildren(n);
        break;
      case "ArrowLeft": {
        // A listed entry steps back to its virtual list, not to the real folder it lives in.
        if (state.virtual !== null && (n === null || keyOf(n.relParts) === state.via)) {
          void openVirtual(state.virtual);
          break;
        }
        const parent = n && n.relParts.length > 1 ? findNode(n.relParts.slice(0, -1)) : null;
        // The column just left stays visible beside its folder.
        if (parent) openColumn(parent);
        break;
      }
      case "F2":
        if (n) startEdit(n.relParts);
        break;
      case "Enter":
        go(Math.max(i, 0));
        break;
      case " ":
        if (n?.hasChildren) openColumn(n);
        else go(Math.max(i, 0));
        break;
      default:
        return;
    }
    ev.preventDefault();
  }

  // The snapshot only carries the first level, so deeper children are fetched when a column opens.
  function loadSelectedChildren(): void {
    const n = selectedNode();
    if (!n || !n.hasChildren || n.children !== null) return;
    const key = keyOf(n.relParts);
    if (loading.has(key) || key !== openKey) return;
    loading.add(key);
    void expand(n).finally(() => loading.delete(key));
  }

  return {
    id: "columns",
    label: "Columns",
    mount(host) {
      root.addEventListener("keydown", onKey);
      // Space beside the columns belongs to the deepest one.
      root.addEventListener("contextmenu", (ev) => folderContextMenu(ev, buildColumns(openKey).at(-1)?.parent ?? null, root));
      host.append(root);
    },
    render() {
      // An opened column lasts only while its folder stays selected; selecting another row closes it.
      if (openKey !== state.selectedKey) openKey = null;
      if (sameManage(built)) return;
      const hadFocus = root.contains(document.activeElement);
      // Rebuilding the columns resets every scroll offset, so each one is kept by its folder key.
      const offsets = new Map(Array.from(root.querySelectorAll<HTMLElement>(".column[data-key]"), (c): [string, number] => [c.dataset.key ?? "", c.scrollTop]));
      const left = root.scrollLeft;
      root.replaceChildren(...buildColumns(openKey).map(buildColumn));
      built = manageSnapshot();
      root.scrollLeft = left;
      root.querySelectorAll<HTMLElement>(".column[data-key]").forEach((c) => {
        c.scrollTop = offsets.get(c.dataset.key ?? "") ?? 0;
      });
      // Only a move of the path or a new column scrolls; any other render must not fight the user's scrolling.
      const reveal = `${state.selectedKey}|${openKey}|${root.childElementCount}`;
      if (reveal !== revealed) {
        revealed = reveal;
        root.querySelectorAll<HTMLElement>(".col-item.on-path, .col-item.selected").forEach((e) => {
          // A multi-selection marks many rows; only the path's own row is followed.
          if (!e.classList.contains("selected") || e.dataset.key === state.selectedKey) e.scrollIntoView({ block: "nearest", inline: "nearest" });
        });
        root.lastElementChild?.scrollIntoView({ block: "nearest", inline: "nearest" });
      }
      if (hadFocus) root.querySelector<HTMLElement>('.col-item[tabindex="0"]')?.focus({ preventScroll: true });
      loadSelectedChildren();
    },
    focus() {
      root.querySelector<HTMLElement>('.col-item[tabindex="0"]')?.focus();
    },
    destroy() {
      revealed = null;
      root.remove();
    },
  };
}