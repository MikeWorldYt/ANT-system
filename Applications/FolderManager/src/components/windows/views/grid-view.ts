// Grid, List, Details and Cards views: the contents of one folder as large icons, plain rows, rows with columns or icons with details, entered with double click or Enter.
import { folderContextMenu, isMenuKey, nodeContextMenu, nodeMenuFromKey } from "../../organisms/menus/context-menu";
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { folderIcon } from "../../atoms/icons";
import { fileThumb } from "../../atoms/thumb";
import { statusMark } from "../../atoms/status-mark";
import { favoriteMark, hideBox, importantMark, nameCell } from "../../molecules/name-cell";
import { nodeToggle } from "../../atoms/node-toggle";
import { sortedForColumns } from "../../../services/store/sort";
import { expand } from "../../../hooks/ops";
import { isManagedFolder, manageBox, manageHeader, managedNodes, manageSnapshot, sameManage, type ManageSnapshot } from "../../molecules/manage-row";
import { findNode, keyOf, listed, select, selectedNode, notify, sortOf, startEdit, state, virtualRoot, type VirtualId } from "../../../services/store/state";
import { virtualNodes } from "../../../services/store/virtual";
import { isMainCategory } from "../../../services/store/node-info";
import { openVirtual } from "../../../hooks/virtual-nav";
import { isTail } from "../../../services/store/sort";
import { entryType, openEntry, placeEntries, rootEntries, virtualEntries, type VirtualEntry } from "../../molecules/virtual-entry";
import { markNodeItem, onItemClick, openOnDouble } from "../../molecules/node-item";
import type { TreeNode } from "../../../services/ipc/types";
import { openFolder } from "../../../hooks/path-actions";
import { isTemplatesHome, newTemplateItem } from "../../molecules/new-template-item";
import type { ViewMode } from "./view-mode";
import { requestFolderSizes } from "../../../hooks/folder-sizes";
import { focusItem } from "../../../hooks/focus-return";
import { detailCells, detailsHeader, entryCells, detailTemplate, fileKind } from "./details-cells";

// One line under each root tile in the grid says what it is for, so a new Storage explains itself.
const ENTRY_HINTS: Record<string, string> = { favorites: "Pinned folders", downloads: "Your downloads", review: "Files to sort", documents: "System documents", pictures: "System pictures", music: "System music", videos: "System videos", desktop: "System desktop" };
const ROOT_HINTS: Record<string, string> = { REC: "Screen recordings", SS: "Captures", TPL: "Worknode templates" };
const rootHint = (n: TreeNode): string | null => (n.file ? null : (ROOT_HINTS[n.nodeType?.base ?? ""] ?? (isMainCategory(n) ? "Your area" : null)));
// In Details an entry lines up as a row: its icon and label take the Name column and its kind the Type column.
function asDetailRow(entry: HTMLElement): HTMLElement {
  entry.replaceChildren(el("span", { class: "detail-lead" }, ...entry.childNodes), ...entryCells(entryType(entry.dataset.entry as VirtualId)));
  return entry;
}

const hintLine = (text: string) => el("span", { class: "tile-hint" }, text);

function createFolderView(id: string, label: string, rootClass: string, detail?: (n: TreeNode) => string, columns = false): ViewMode {
  const root = el("div", { class: rootClass, role: "listbox", "aria-label": "Folder contents" });
  const loading = new Set<string>();
  // Key of the folder being shown; null is the Storage root.
  let folder: string | null = null;
  let shownFolder: string | null = null;
  let revealed: string | null = null;
  const jump = el("button", { type: "button", class: "jump-selection", "aria-label": "Back to the selected item", title: "Back to the selected item", hidden: "" });
  jump.addEventListener("click", () => {
    const sel = root.querySelector<HTMLElement>(".grid-item.selected");
    sel?.scrollIntoView({ block: "center" });
    sel?.focus({ preventScroll: true });
  });
  // The arrow appears only while the selection is scrolled out of sight, and points the way back to it.
  function updateJump(): void {
    const sel = root.querySelector<HTMLElement>(".grid-item.selected");
    const box = root.getBoundingClientRect();
    const r = sel?.getBoundingClientRect();
    const above = r !== undefined && r.bottom < box.top;
    const below = r !== undefined && r.top > box.bottom;
    jump.hidden = !(above || below);
    jump.classList.toggle("down", below);
    // Fixed to the window, so it is placed at the list's bottom-right corner whatever box holds the list.
    jump.style.left = `${box.right - 54}px`;
    jump.style.top = `${box.bottom - 54}px`;
  }

  // Selecting inside the folder keeps it; any other selection (breadcrumb, back, search) moves the grid there.
  function syncFolder(): void {
    const key = state.selectedKey;
    // An open virtual list is shown until one of its entries is entered; below it the grid moves as in any folder.
    if (key === null || (state.virtual !== null && folder === null && state.virtualKeys.includes(key))) {
      folder = null;
      return;
    }
    const parts = key.split("/");
    const parent = parts.length > 1 ? parts.slice(0, -1).join("/") : null;
    if (key === state.entering) {
      state.entering = null;
      if (selectedNode()?.file === false) {
        folder = key;
        return;
      }
    }
    // A restored tab or launch shows the folder it showed, not the folder selected in it.
    if (state.showing !== undefined && (state.showing === parent || state.showing === key)) {
      folder = state.showing;
      state.showing = undefined;
      return;
    }
    if (key === folder || parent === folder) return;
    const next = selectedNode()?.hasChildren ? key : parent;
    // A native folder itself is no node, so a selection left on a deleted entry falls back to its list.
    folder = next !== null && next === virtualRoot(state.virtual) ? null : next;
  }

  let built: ManageSnapshot | null = null;

  function folderNode(): TreeNode | null {
    return folder === null ? null : findNode(folder.split("/"));
  }

  function shownNodes(): TreeNode[] {
    if (state.virtual !== null && folder === null) return virtualNodes();
    const open = folderNode();
    const base = folder === null ? (state.tree?.nodes ?? []) : (open?.children ?? []);
    return (isManagedFolder(folder ?? "") ? managedNodes() : null) ?? sortedForColumns(listed(base), sortOf(folder ?? ""), state.settings.pinFavorites);
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
    if (n.file) return;
    folder = keyOf(n.relParts);
    select(n.relParts);
  }

  // A file's thumbnail replaces its icon once the shell has drawn it.
  function previewIcon(n: TreeNode): HTMLElement {
    return fileThumb(n, () => folderIcon(n)) as HTMLElement;
  }

  function buildItem(n: TreeNode, tabbable: boolean): HTMLElement {
    const icon = n.file ? previewIcon(n) : folderIcon(n);
    const art = n.hasChildren ? nodeToggle(`Open ${n.name}`, null, folderIcon(n)) : el("span", {}, icon);
    art.classList.add("grid-art");
    if (n.favorite) art.append(favoriteMark());
    if (n.important) art.append(importantMark());
    if (n.status) art.append(statusMark(n.status));
    const lead = [...hideBox(n), art, detail ? el("div", { class: "card-text" }, nameCell(n, false), el("span", { class: "card-detail" }, detail(n))) : nameCell(n, false, false, !columns)];
    // In Details the icon, its marks and the name share the Name column, so every row lines up under the header.
    const leadBox = columns ? el("span", { class: "detail-lead" }, ...lead) : null;
    const item = el("div", { class: "grid-item", role: "option", tabindex: tabbable ? "0" : "-1" }, ...(leadBox ? [leadBox] : lead));
    if (columns) item.append(...detailCells(n));
    markNodeItem(item, n);
    // Card names truncate, so the tooltip leads with the full one.
    if (detail) item.title = `${n.name}
${item.title}`;
    (leadBox ?? item).prepend(...manageBox(n, item));
    onItemClick(item, n, () => shownNodes().map((s) => keyOf(s.relParts)), () => select(n.relParts));
    // As in Explorer, a double click opens; the name renames on a slow second click.
    item.addEventListener("dblclick", () => (n.file ? openOnDouble(n) : enter(n)));
    item.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, root));
    return item;
  }

  // In the root grid every entry leads; an open list and a folder have none.
  const leadEntries = (): VirtualEntry[] => {
    const open = openEntry();
    return folder !== null || open ? [] : rootEntries();
  };

  function perRow(): number {
    const items = [...root.querySelectorAll<HTMLElement>(".grid-item")];
    const top = items[0]?.offsetTop;
    return Math.max(1, items.filter((i) => i.offsetTop === top).length);
  }

  function showMark(): void {
    const tile = state.marked ? root.querySelector<HTMLElement>(`.grid-item[data-entry="${state.marked}"]`) : null;
    if (!tile) return;
    root.querySelectorAll(".grid-item.selected").forEach((t) => t.classList.remove("selected"));
    root.querySelectorAll<HTMLElement>('.grid-item[tabindex="0"]').forEach((t) => (t.tabIndex = -1));
    tile.classList.add("selected");
    tile.tabIndex = 0;
    tile.focus({ preventScroll: true });
    tile.scrollIntoView({ block: "nearest" });
  }
  function markEntry(id: VirtualId): void {
    state.marked = id;
    if (state.selectedKey !== null) return select(null);
    showMark();
    notify();
  }

  function onKey(ev: KeyboardEvent): void {
    // Modified arrows belong to the global shortcuts (Ctrl+Right, Alt+Down, Alt+Up).
    if (ev.ctrlKey || ev.altKey || ev.metaKey) return;
    const n = selectedNode();
    if (isMenuKey(ev)) {
      if (n) nodeMenuFromKey(root, n, root.querySelector(".grid-item.selected"));
      ev.preventDefault();
      return;
    }
    // Entries sit between the folders, so the arrows walk the tiles in the order they are drawn.
    const nodes = shownNodes();
    const tiles = [...root.querySelectorAll<HTMLElement>(".grid-item")];
    const i = tiles.findIndex((t) => t.classList.contains("selected"));
    const go = (j: number) => {
      const at = Math.max(0, Math.min(tiles.length - 1, j));
      if (at === i) return;
      const tile = tiles[at];
      const entry = tile?.dataset.entry as VirtualId | undefined;
      if (entry) return markEntry(entry);
      const target = nodes.find((x) => keyOf(x.relParts) === tile?.dataset.key);
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
        go(tiles.length - 1);
        break;
      case "F2":
        if (n) startEdit(n.relParts);
        break;
      case "Enter":
        if (state.marked && state.selectedKey === null) void openVirtual(state.marked);
        else if (i < 0) go(0);
        else if (n?.file) void openFolder(n);
        else if (n) enter(n);
        break;
      default:
        return;
    }
    ev.preventDefault();
  }

  return {
    id,
    label,
    openFolder() {
      syncFolder();
      return folderNode();
    },
    mount(host) {
      root.addEventListener("keydown", onKey);
      root.addEventListener("contextmenu", (ev) => folderContextMenu(ev, folderNode(), root));
      root.addEventListener("scroll", updateJump, { passive: true });
      jump.replaceChildren(guiIcon("up"));
      host.append(root, jump);
    },
    render() {
      syncFolder();
      const listing = folder === null ? openEntry() : null;
      // The background is a drop target for the folder shown; a virtual list is no folder.
      if (listing) delete root.dataset.key;
      else root.dataset.key = folder ?? "";
      if (state.selectedKey !== null || state.virtual !== null) state.marked = null;
      if (sameManage(built)) return;
      const hadFocus = root.contains(document.activeElement);
      const nodes = shownNodes();
      // With nothing selected the entry tile leads the roving focus.
      const entryTabbable = leadEntries().length > 0 && state.selectedKey === null;
      const tabbable = entryTabbable ? undefined : (nodes.find((n) => keyOf(n.relParts) === state.selectedKey) ?? nodes[0]);
      const open = folderNode();
      const empty = nodes.length === 0 && open !== null && open.children !== null;
      const legend = listing && nodes.length === 0 ? listing.empty : empty ? "Empty" : null;
      const columnKey = folder ?? virtualRoot(state.virtual) ?? "";
      if (columns) root.style.setProperty("--detail-cols", detailTemplate(columnKey));
      const header = [...(isManagedFolder(folder ?? "") ? [manageHeader("manage-bar")] : []), ...(columns ? [detailsHeader(columnKey)] : [])];
      const entryOptions = { tabbable: entryTabbable, mark: markEntry };
      const items = nodes.map((n) => buildItem(n, n === tabbable));
      const tail = nodes.findIndex((n) => !n.file && isTail(n.name));
      const placed = listing ? items : folder === null ? placeEntries(virtualEntries("grid-item", "option", entryOptions).map((e) => (columns ? asDetailRow(e) : e)), items, tail < 0 ? items.length : tail) : items;
      if (id === "grid" && folder === null && !listing) {
        for (const tile of placed) {
          const node = tile.dataset.key ? findNode(tile.dataset.key.split("/")) : null;
          const hint = tile.dataset.entry ? ENTRY_HINTS[tile.dataset.entry] : node ? rootHint(node) : null;
          if (hint) tile.append(hintLine(hint));
        }
      }
      const creator = !listing && isTemplatesHome(open) ? [newTemplateItem(open, "grid-item", "option")] : [];
      // Rebuilding empties the list for a moment, which would drop the scroll back to the top.
      const place = `${state.virtual}|${folder}`;
      const keepScroll = shownFolder === place ? root.scrollTop : 0;
      shownFolder = place;
      root.replaceChildren(...header, ...creator, ...placed, ...(legend && creator.length === 0 ? [el("p", { class: "grid-legend" }, legend)] : []));
      built = manageSnapshot();
      root.scrollTop = keepScroll;
      showMark();
      // Only a new selection is brought into view; any other render leaves the scroll where the user put it.
      const picked = `${place}|${state.selectedKey}`;
      if (picked !== revealed) root.querySelector(".grid-item.selected")?.scrollIntoView({ block: "nearest" });
      revealed = picked;
      updateJump();
      if (hadFocus) root.querySelector<HTMLElement>('.grid-item[tabindex="0"]')?.focus({ preventScroll: true });
      loadChildren();
      if (columns) requestFolderSizes(nodes);
    },
    focus() {
      focusItem(root.querySelector<HTMLElement>('.grid-item[tabindex="0"]'));
    },
    destroy() {
      root.remove();
      jump.remove();
    },
  };
}

export const createGridView = () => createFolderView("grid", "Grid", "grid");
export const createCardsView = () => createFolderView("cards", "Cards", "grid cards", fileKind);
export const createListView = () => createFolderView("list", "List", "grid list");
export const createDetailsView = () => createFolderView("details", "Details", "grid list details", undefined, true);
