// Background menu of a native folder: the storage board menu's pieces, minus what only a storage can do.
import { newNativeFolder, pasteFiles, refreshNative } from "../../../hooks/ops";
import { openNode } from "../../../services/ipc/api";
import { showFolderProperties } from "../../../hooks/properties";
import { state } from "../../../services/store/state";
import { reportError } from "../bars/status-line";
import { guiIcon } from "../../atoms/gui-icons";
import { closingItem, menuItem, separator } from "../../molecules/menu";
import { sortSubmenu, viewSubmenu, type MenuParts } from "./board-menu";
import type { ViewPicker } from "../bars/toolbar";

export function nativeBackgroundMenu(parts: string[], views: ViewPicker, close: (restore?: boolean) => void): MenuParts {
  const action = closingItem(close);
  const open = action("Open folder location", true, () => void openNode(parts).catch((e: unknown) => reportError("error", e)));
  const paste = action("Paste", true, () => void pasteFiles(parts));
  const subs = [viewSubmenu(views, close), sortSubmenu(parts, close)];
  const reload = action("Refresh", !state.busy, () => void refreshNative(parts));
  const properties = action("Properties", true, () => showFolderProperties(parts));
  // The rename that follows takes the focus, so the menu does not hand it back to the view.
  const add = menuItem("New folder", "menuitem", !state.busy, () => {
    close(false);
    void newNativeFolder(parts);
  });
  paste.prepend(guiIcon("paste"));
  reload.prepend(guiIcon("refresh"));
  add.prepend(guiIcon("add"));
  return {
    items: [open, paste, ...subs.map((s) => s.trigger), reload, add, properties],
    subs,
    content: [open, separator(), paste, separator(), ...subs.map((s) => s.element), reload, separator(), add, separator(), properties],
  };
}
