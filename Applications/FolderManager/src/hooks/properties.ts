// Opens the Properties window of an item, handing it the icon its row shows so the new window needs no tree of its own.
import { openPropertiesWindow } from "../services/ipc/api";
import { folderIcon, kindIcon } from "../components/atoms/icons";
import { reportError } from "../components/organisms/bars/status-line";
import type { TreeNode } from "../services/ipc/types";

export const showProperties = (node: TreeNode): void => open(node.relParts, folderIcon(node).src);

// A native folder's own list has no node, so only its root marker names it.
export const showFolderProperties = (relParts: string[]): void => open(relParts, kindIcon("folder").src);

function open(relParts: string[], icon: string): void {
  openPropertiesWindow(relParts, icon).catch((e: unknown) => reportError("error", e, "Properties window"));
}
