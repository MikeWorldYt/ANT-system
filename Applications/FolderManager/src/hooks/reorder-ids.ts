// Reorder by id: a warning that the paths change, the order dialog, then one backend call for every rename.
import * as api from "../services/ipc/api";
import { warningConfirmDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { reorderDialog } from "../components/organisms/dialogs/action-dialog/reorder-dialog";
import { reportError, setStatus } from "../components/organisms/bars/status-line";
import { reorderChildren } from "./ops";
import { findNode, state } from "../services/store/state";
import { isTracked } from "../services/store/id-order";
import { inSystem, type ReorderPlan, type TreeNode } from "../services/ipc/types";

const WARNING = [
  "Changing the ids renames these folders, so their paths change.",
  "Any program, shortcut or link that uses an old path may stop working and will need to be linked again to the new one.",
];

// The backend marks the slot nodes a move may swap; those are the nodes a reorder renumbers.
const numbered = (nodes: TreeNode[]): number => nodes.filter((n) => n.actions.moveUp || n.actions.moveDown).length;

const worknodes = (nodes: TreeNode[]): number =>
  nodes.filter((n) => !n.file && !n.ghost && !n.linked && !n.locked && n.nodeType?.base === "WKB" && isTracked(n.name)).length;

// A folder whose children are not loaded yet is offered when it has any; the plan counts them once read.
export function canReorderIds(folder: TreeNode): boolean {
  const real = !folder.file && !folder.ghost && !folder.linked && !folder.reserved && !folder.locked && folder.relParts.length > 0 && !inSystem(folder.relParts);
  if (!state.settings.adminIdentifiers || !real || state.busy || state.virtual !== null) return false;
  const children = findNode(folder.relParts)?.children ?? folder.children;
  return children === null ? folder.hasChildren : numbered(children) >= 2 || worknodes(children) >= 2;
}

export async function reorderIds(folder: TreeNode): Promise<void> {
  let plan: ReorderPlan;
  let icons: Map<string, TreeNode>;
  try {
    plan = await api.reorderPlan(folder.relParts);
    icons = new Map((await api.listAllChildren(folder.relParts)).map((n) => [n.name, n]));
  } catch (e) {
    reportError("error", e);
    return;
  }
  if (plan.nodes.length < 2 && plan.worknodes.length < 2) return setStatus("info", `"${folder.name}" has fewer than two numbered folders or worknodes to reorder.`);
  if (!(await warningConfirmDialog("Reorder by id", WARNING, "Continue"))) return;
  const answer = await reorderDialog(folder.name, plan, icons, folder.relParts.length === 0);
  if (answer) await reorderChildren(folder, answer.targets, answer.renames);
}
