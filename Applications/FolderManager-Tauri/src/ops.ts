// User operations: call the backend, replace the snapshot, report errors in the status line.
import * as api from "./api";
import { confirmDialog } from "./dialogs";
import { setStatus, clearStatus } from "./status";
import { hydrate, keyOf, notify, pruneSelection, setBusy, setTree, state } from "./state";
import { isFmError } from "./types";
import type { EditableType, MoveDirection, Mutation, TreeNode, TreeSnapshot, WkbStatus } from "./types";

export interface Edit {
  editable: string;
  type: string;
  status: WkbStatus;
}

const EDITABLE: readonly string[] = ["NODE", "WKB", "PORTF"];

// The first dot separates the numeric id from the editable part.
export const editablePart = (name: string) => (name.includes(".") ? name.slice(name.indexOf(".") + 1) : name);

async function reload(snapshot?: TreeSnapshot): Promise<void> {
  setTree(snapshot ?? (await api.refresh()));
  await hydrate();
  pruneSelection();
}

async function run(fn: () => Promise<void>): Promise<void> {
  setBusy(true);
  try {
    await fn();
  } catch (e) {
    setStatus("error", api.describeError(e));
    if (isFmError(e) && (e.kind === "stale" || e.kind === "notFound")) {
      try {
        await reload();
      } catch (reloadError) {
        setStatus("error", api.describeError(reloadError));
      }
    }
  } finally {
    setBusy(false);
  }
}

async function applyMutation(m: Mutation, fallback: string[] | null = null): Promise<void> {
  const target = m.select ?? fallback;
  if (target) {
    for (let i = 1; i < target.length; i++) state.expanded.add(keyOf(target.slice(0, i)));
    state.selectedKey = keyOf(target);
  }
  await reload(m.tree);
  if (m.warnings.length > 0) setStatus("warning", m.warnings.join(". "));
  else clearStatus();
}

export function init(): Promise<void> {
  return run(async () => {
    await reload(await api.loadTree());
  });
}

export function refresh(): Promise<void> {
  if (state.busy) return Promise.resolve();
  return run(async () => {
    await reload();
    clearStatus();
  });
}

export async function expand(node: TreeNode): Promise<void> {
  const key = keyOf(node.relParts);
  state.expanded.add(key);
  notify();
  try {
    await hydrate();
  } catch (e) {
    state.expanded.delete(key);
    setStatus("error", api.describeError(e));
  }
  notify();
}

export async function toggleExpand(node: TreeNode): Promise<void> {
  const key = keyOf(node.relParts);
  if (!node.hasChildren) return;
  if (state.expanded.has(key)) {
    state.expanded.delete(key);
    notify();
    return;
  }
  await expand(node);
}

export const addChild = (node: TreeNode) =>
  run(async () => {
    state.expanded.add(keyOf(node.relParts));
    node.children = null;
    await applyMutation(await api.addChild(node.relParts));
  });

export const move = (node: TreeNode, dir: MoveDirection) =>
  run(async () => applyMutation(await api.moveNode(node.relParts, dir), node.relParts));

export const apply = (node: TreeNode, edit: Edit) =>
  run(async () => {
    const renamed = node.actions.rename && edit.editable.trim() !== editablePart(node.name);
    const base = node.nodeType?.base;
    const typeChanged =
      node.actions.changeType &&
      EDITABLE.includes(edit.type) &&
      (edit.type !== base || (edit.type === "WKB" && edit.status !== node.status));
    let parts = node.relParts;
    if (renamed) {
      const m = await api.renameNode(parts, edit.editable.trim());
      parts = m.select ?? parts;
      if (!typeChanged) return applyMutation(m);
    }
    if (typeChanged) {
      const status = edit.type === "WKB" ? edit.status : null;
      await applyMutation(await api.setNodeType(parts, edit.type as EditableType, status), parts);
    }
  });

export const remove = (node: TreeNode) =>
  run(async () => {
    const preview = await api.inspectDelete(node.relParts);
    const nonEmpty = preview.files + preview.dirs > 0;
    if (nonEmpty) {
      const ok = await confirmDialog(
        "Delete folder",
        `"${node.name}" contains ${preview.files} files and ${preview.dirs} folders. They go to the Recycle Bin`,
        "Delete",
      );
      if (!ok) return;
    }
    const m = await api.deleteNode(node.relParts, preview.ghost, nonEmpty);
    const parent = node.relParts.slice(0, -1);
    await applyMutation(m, parent.length > 0 ? parent : null);
  });
