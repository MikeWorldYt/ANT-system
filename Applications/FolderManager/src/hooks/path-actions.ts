// Copying a node's path and opening it here or in Explorer, shared by the node menus and the board cards.
import { openNode } from "../services/ipc/api";
import { keyOf, state } from "../services/store/state";
import { focusPane, openSplit } from "./split";
import { goToReal } from "./virtual-nav";
import { setStatus, reportError } from "../components/organisms/bars/status-line";
import type { TreeNode } from "../services/ipc/types";

export async function copyPath(node: TreeNode): Promise<void> {
  try {
    await navigator.clipboard.writeText(node.absPath);
    setStatus("info", `Path copied: ${node.absPath}`);
  } catch {
    setStatus("warning", `Could not copy to the clipboard. Path: ${node.absPath}`);
  }
}

export async function openInApp(node: Pick<TreeNode, "relParts">): Promise<void> {
  state.entering = keyOf(node.relParts);
  state.expanded.add(state.entering);
  await goToReal(node.relParts);
}

export async function openInSplit(node: Pick<TreeNode, "relParts">): Promise<void> {
  openSplit();
  focusPane(true);
  await openInApp(node);
}

export async function openFolder(node: TreeNode): Promise<void> {
  try {
    await openNode(node.relParts);
  } catch (e) {
    reportError("warning", e);
  }
}
