// Background menu of the open folder: paste into it, switch the view, sort its contents and add folders.
import { addChild, manageHidden, pasteFiles, refresh } from "../../../hooks/ops";
import { keyOf, setColumnSort, sortOf, state, type ColumnSort } from "../../../services/store/state";
import { SORT_KEYS, orderedModes, type ViewPicker } from "../bars/toolbar";
import { menuIcon, closingItem, menuItem, separator, submenu, type Submenu } from "../../molecules/menu";
import type { GuiIcon } from "../../atoms/gui-icons";
import type { TreeNode } from "../../../services/ipc/types";

export interface MenuParts {
  items: HTMLButtonElement[];
  subs: Submenu[];
  content: HTMLElement[];
}

// The folder's contents are sorted under its own key, not under the column its row stands in.
export function sortSubmenu(node: TreeNode, close: (restore?: boolean) => void): Submenu {
  const column = keyOf(node.relParts);
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

export function boardMenu(node: TreeNode, views: ViewPicker, close: (restore?: boolean) => void): MenuParts {
  const action = closingItem(close);
  const idle = !state.busy;
  // Enabled even on locked folders: the backend picks the real target and explains a refusal.
  const paste = action("Paste", true, () => void pasteFiles(node.relParts));
  const pasteItems = node.reserved || node.ghost ? [] : [paste];

  const viewChoices = orderedModes(views).map((m) =>
    menuItem(m.label, "menuitemradio", true, () => {
      views.onSelect(m.id);
      close();
    }, "ctx-item", m.id === views.active()),
  );
  const subs: Submenu[] = [submenu("Change View", viewChoices, true)];

  if (views.active() !== "tree") subs.push(sortSubmenu(node, close));

  const reload = action("Refresh", idle, () => void refresh());
  const canAdd = node.actions.addChild || node.actions.addFolder;
  const newNode = action(node.actions.addFolder ? "New folder" : "New node", canAdd, () => void addChild(node, "node"));
  const newWorknode = action("New worknode", canAdd, () => void addChild(node, "worknode"));
  // Like the toolbar's Add menu, a main category gets no worknode.
  const addItems = node.relParts.length === 1 ? [newNode] : [newNode, newWorknode];
  const manage = action("Manage hidden nodes", true, () => void manageHidden(node.relParts));

  return {
    items: [...pasteItems, ...subs.map((s) => s.trigger), reload, ...addItems, manage],
    subs,
    content: [...pasteItems, ...(pasteItems.length > 0 ? [separator()] : []), ...subs.map((s) => s.element), reload, separator(), ...addItems, separator(), manage],
  };
}
