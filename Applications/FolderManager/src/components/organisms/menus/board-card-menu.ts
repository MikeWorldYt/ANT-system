// Right-click menu of a board card; its Move to submenu is the keyboard way to change a card's column.
import { hideNode, remove, setImportant } from "../../../hooks/ops";
import { goToReal } from "../../../hooks/virtual-nav";
import { BOARD_COLUMNS } from "../../../services/store/board";
import { importantLabel } from "../../../services/store/node-info";
import { guiIcon } from "../../atoms/gui-icons";
import { menuItem, separator, submenu } from "../../molecules/menu";
import { copyPath } from "../../../hooks/path-actions";
import type { MenuParts } from "./board-menu";
import type { BoardColumn, TreeNode } from "../../../services/ipc/types";

export interface CardActions {
  edit(): void;
  moveTo(column: BoardColumn): void;
}

export function boardCardMenu(node: TreeNode, at: BoardColumn, act: CardActions, close: (restore?: boolean) => void): MenuParts {
  const a = node.actions;
  const open = menuItem("Open", "menuitem", a.open, () => {
    close(false);
    void goToReal(node.relParts);
  });
  const copy = menuItem("Copy path", "menuitem", a.copyPath, () => {
    close();
    void copyPath(node);
  });
  const edit = menuItem("Edit", "menuitem", a.editDetails || a.rename, () => {
    close(false);
    act.edit();
  });
  const important = menuItem(importantLabel(node.important), "menuitem", a.important, () => {
    close();
    void setImportant([node], !node.important);
  });
  important.prepend(guiIcon("important"));
  important.classList.toggle("ctx-important-add", !node.important);
  const columns = BOARD_COLUMNS.map(([id, label]) =>
    menuItem(label, "menuitemradio", id !== at, () => {
      close();
      act.moveTo(id);
    }, "ctx-item", id === at),
  );
  const move = submenu("Move to", columns, true);
  const hide = menuItem("Hide element", "menuitem", a.hide, () => {
    close();
    void hideNode(node);
  });
  hide.prepend(guiIcon("hide"));
  const del = menuItem("Delete", "menuitem", a.delete, () => {
    close();
    void remove(node);
  }, "ctx-item danger");
  del.prepend(guiIcon("trash"));
  return {
    items: [open, copy, edit, important, move.trigger, hide, del],
    subs: [move],
    content: [open, copy, edit, important, move.element, separator(), hide, del],
  };
}
