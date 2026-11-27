// Allow files and Activate automations, shared by a node's own menu and the menu of the folder it opens.
import { setUserFlag } from "../../../hooks/ops";
import type { TreeNode } from "../../../services/ipc/types";

type Action = (label: string, enabled: boolean, run: () => void) => HTMLButtonElement;

export function folderFlagItems(node: TreeNode, action: Action): HTMLButtonElement[] {
  const allowsFiles = node.flags.includes("data");
  const files = action(allowsFiles ? "Deny files" : "Allow files", node.actions.editFlags, () => void setUserFlag(node, !allowsFiles));
  const automated = node.flags.includes("autom");
  const autom = action(automated ? "Deactivate automations" : "Activate automations", !node.ghost && !node.locked && !node.reserved && node.relParts.length > 1, () => void setUserFlag(node, !automated, "autom"));
  const base = node.nodeType?.base ?? "";
  return [...(base === "NODE" ? [files] : []), ...(["REV", "SS", "REC", "RES"].includes(base) ? [] : [autom])];
}
