// Background menu of the open folder: paste into it, switch the view, sort its contents and add folders.
import { addChild, manageHidden, pasteFiles, refresh } from "../../../hooks/ops";
import { openFolder } from "../../../hooks/path-actions";
import { canReorderIds, reorderIds } from "../../../hooks/reorder-ids";
import { keyOf, setColumnSort, sortOf, state, type ColumnSort } from "../../../services/store/state";
import { SORT_KEYS, orderedModes, type ViewPicker } from "../bars/toolbar";
import { VIEW_ICONS } from "../../molecules/view-choice";
import { folderFlagItems } from "./folder-flags";
import { menuIcon, closingItem, menuItem, separator, submenu, type Submenu } from "../../molecules/menu";
import { guiIcon, type GuiIcon } from "../../atoms/gui-icons";
import type { TreeNode } from "../../../services/ipc/types";

export interface MenuParts {
  items: HTMLButtonElement[];
  subs: Submenu[];
  content: HTMLElement[];
}

// The folder's contents are sorted under its own key, not under the column its row stands in.
export function sortSubmenu(relParts: string[], close: (restore?: boolean) => void): Submenu {
  const column = keyOf(relParts);
  const { by, dir } = sortOf(column);
  const sortChoice = (label: string, icon: GuiIcon, checked: boolean, patch: Partial<ColumnSort>, flip = false) => {
    const item = menuItem(label, "menuitemradio", true, () => {
      close();
      setColumnSort(column, patch);
    }, "ctx-item ctx-iconed", checked);
    item.prepend(menuIcon(icon, flip));
    return item;
  };
  const ascending = sortChoice("Ascending", "up", dir === "asc", { dir: "asc" });
  const sort = submenu("Sort by", [...SORT_KEYS.map(([id, label, icon]) => sortChoice(label, icon, by === id, { by: id })), ascending, sortChoice("Descending", "up", dir === "desc", { dir: "desc" }, true)], true);
  // submenu() lays out only its choices, so the divider is slotted into the flyout afterwards.
  ascending.before(separator());
  return sort;
}

export function viewSubmenu(views: ViewPicker, close: (restore?: boolean) => void): Submenu {
  const choices = orderedModes(views).map((m) => {
    const item = menuItem(m.label, "menuitemradio", true, () => {
      views.onSelect(m.id);
      close();
    }, "ctx-item ctx-iconed", m.id === views.active());
    item.prepend(menuIcon(VIEW_ICONS[m.id] ?? "layout"));
    return item;
  });
  return submenu("Change View", choices, true);
}

export function boardMenu(node: TreeNode, views: ViewPicker, close: (restore?: boolean) => void): MenuParts {
  const action = closingItem(close);
  const idle = !state.busy;
  // Enabled even on locked folders: the backend picks the real target and explains a refusal.
  const paste = action("Paste", true, () => void pasteFiles(node.relParts));
  const pasteItems = node.reserved || node.ghost ? [] : [paste];

  const subs: Submenu[] = [viewSubmenu(views, close), sortSubmenu(node.relParts, close)];

  const reload = action("Refresh", idle, () => void refresh());
  const canAdd = node.actions.addChild || node.actions.addFolder;
  const newNode = action(node.actions.addFolder ? "New folder" : "New node", canAdd, () => void addChild(node, "node"));
  const newWorknode = action("New worknode", canAdd, () => void addChild(node, "worknode"));
  // Like the toolbar's Add menu, a main category gets no worknode.
  const addItems = node.relParts.length === 1 ? [newNode] : [newNode, newWorknode];
  paste.prepend(guiIcon("paste"));
  reload.prepend(guiIcon("refresh"));
  newNode.prepend(guiIcon("add"));
  newWorknode.prepend(guiIcon("board"));
  const manage = action("Manage hidden nodes", true, () => void manageHidden(node.relParts));
  const open = action("Open folder location", node.actions.open, () => void openFolder(node));
  const flagItems = folderFlagItems(node, action);
  const reorderItems = canReorderIds(node) ? [action("Reorder by id…", true, () => void reorderIds(node))] : [];

  return {
    items: [open, ...pasteItems, ...subs.map((s) => s.trigger), reload, ...flagItems, manage, ...reorderItems, ...addItems],
    subs,
    content: [open, separator(), ...pasteItems, ...(pasteItems.length > 0 ? [separator()] : []), ...subs.map((s) => s.element), reload, separator(), ...flagItems, manage, ...reorderItems, separator(), ...addItems],
  };
}
