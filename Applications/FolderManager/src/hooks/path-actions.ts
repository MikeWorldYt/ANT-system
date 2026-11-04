// Copying a node's path and opening it in Explorer, shared by the node menus and the board cards.
import { describeError, openNode } from "../services/ipc/api";
import { setStatus } from "../components/organisms/bars/status-line";
import type { TreeNode } from "../services/ipc/types";

export async function copyPath(node: TreeNode): Promise<void> {
  try {
    await navigator.clipboard.writeText(node.absPath);
    setStatus("info", `Path copied: ${node.absPath}`);
  } catch {
    setStatus("warning", `Could not copy to the clipboard. Path: ${node.absPath}`);
  }
}

export async function openFolder(node: TreeNode): Promise<void> {
  try {
    await openNode(node.relParts);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}
