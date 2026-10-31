// Background menu of the open folder: paste into it, switch the view, sort its contents and add folders.
import { addChild, pasteFiles, refresh } from "../actions/ops";
import { keyOf, setColumnSort, sortOf, state, type ColumnSort } from "../store/state";
import { SORT_KEYS, orderedModes, type ViewPicker } from "../shell/toolbar";
import { menuItem, separator, submenu, type Submenu } from "../ui/menu";
import type { TreeNode } from "../ipc/types";

export interface MenuParts {
  items: HTMLButtonElement[];
  subs: Submenu[];
  content: HTMLElement[];
}

export function boardMenu(node: TreeNode, views: ViewPicker, close: (restore?: boolean) => void): MenuParts {
  const idle = !state.busy;
  // Enabled even on locked folders: the backend picks the real target and explains a refusal.
  const paste = menuItem("Paste", "menuitem", true, () => {
    close();
    void pasteFiles(node.relParts);
  });
  const pasteItems = node.reserved || node.ghost ? [] : [paste];

  const viewChoices = orderedModes(views).map((m) =>
    menuItem(m.label, "menuitemradio", true, () => {
      views.onSelect(m.id);
      close();
    }, "ctx-item", m.id === views.active()),
  );
  const subs: Submenu[] = [submenu("Change View", viewChoices, true)];

  // The open folder's contents are sorted under its own key, not under the column its row stands in.
  const column = keyOf(node.relParts);
  const { by, dir } = sortOf(column);
  const sortChoice = (label: string, checked: boolean, patch: Partial<ColumnSort>) =>
    menuItem(label, "menuitemradio", true, () => {
      close();
      setColumnSort(column, patch);
    }, "ctx-item", checked);
  const ascending = sortChoice("Ascending", dir === "asc", { dir: "asc" });
  const sort = submenu("Sort by", [...SORT_KEYS.map(([id, label]) => sortChoice(label, by === id, { by: id })), ascending, sortChoice("Descending", dir === "desc", { dir: "desc" })], true);
  // submenu() lays out only its choices, so the divider is slotted into the flyout afterwards.
  ascending.before(separator());
  if (views.active() !== "tree") subs.push(sort);

  const reload = menuItem("Refresh", "menuitem", idle, () => {
    close();
    void refresh();
  });
  const canAdd = node.actions.addChild && idle;
  const newNode = menuItem("New node", "menuitem", canAdd, () => {
    close();
    void addChild(node, "node");
  });
  const newWorkboard = menuItem("New workboard", "menuitem", canAdd, () => {
    close();
    void addChild(node, "workboard");
  });
  // Like the toolbar's Add menu, a main category gets no workboard.
  const addItems = node.relParts.length === 1 ? [newNode] : [newNode, newWorkboard];

  return {
    items: [...pasteItems, ...subs.map((s) => s.trigger), reload, ...addItems],
    subs,
    content: [...pasteItems, ...(pasteItems.length > 0 ? [separator()] : []), ...subs.map((s) => s.element), reload, separator(), ...addItems],
  };
}
