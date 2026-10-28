// User operations: call the backend, replace the snapshot, report errors in the status line.
import * as api from "../ipc/api";
import { confirmDialog } from "../ui/dialogs";
import { setStatus, clearStatus } from "../ui/status";
import { checkStorage } from "./startup-check";
import { applySettings, findNode, hydrate, keyOf, lastSelection, notify, pruneSelection, reveal, select, serializeSetting, setBusy, setTree, state, type Settings } from "../store/state";
import { isFmError } from "../ipc/types";
import type { EditableType, MoveDirection, Mutation, NewKind, TreeNode, TreeSnapshot, WkbStatus } from "../ipc/types";

async function reload(snapshot?: TreeSnapshot): Promise<void> {
  setTree(snapshot ?? (await api.refresh()));
  await hydrate();
  pruneSelection();
}

async function run(fn: () => Promise<void>): Promise<boolean> {
  setBusy(true);
  try {
    await fn();
    return true;
  } catch (e) {
    setStatus("error", api.describeError(e));
    if (isFmError(e) && (e.kind === "stale" || e.kind === "notFound")) {
      try {
        await reload();
      } catch (reloadError) {
        setStatus("error", api.describeError(reloadError));
      }
    }
    return false;
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
  // The backend may flag a yaml change and leave the node list empty; the tree then has to be read again.
  if (m.tree.nodes.length === 0 && m.tree.yamlChanged) await reload();
  else await reload(m.tree);
  if (m.warnings.length > 0) setStatus("warning", m.warnings.join(". "));
  else clearStatus();
}

export function init(): Promise<boolean> {
  return run(async () => {
    if (!(await checkStorage())) return;
    try {
      applySettings(await api.getSettings());
    } catch (e) {
      setStatus("warning", `Settings: ${api.describeError(e)}`);
    }
    await reload(await api.loadTree());
    const last = state.settings.onLaunch === "continue" ? lastSelection() : null;
    if (!last) return;
    await reveal(last);
    if (!findNode(last)) select(null);
  });
}

export const changeSetting = <K extends keyof Settings>(key: K, value: Settings[K]) =>
  run(async () => {
    await api.setSetting(key, serializeSetting(value));
    state.settings[key] = value;
    notify();
  });

export const resetPreferences = () =>
  run(async () => {
    const wasLogging = state.settings.debugLogging;
    await api.resetSettings();
    applySettings({});
    if (wasLogging) await api.stopDebugLog();
    notify();
  });

export const restoreBackupNamed = (name: string, stamp: string) =>
  run(async () => {
    await api.restoreBackup(name, stamp);
    applySettings(await api.getSettings());
    await reload();
    clearStatus();
  });

export function refresh(): Promise<boolean> {
  if (state.busy) return Promise.resolve(false);
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

export const addNext = (node: TreeNode, kind: NewKind) =>
  run(async () => applyMutation(await api.addNode(node.relParts, kind)));

export const move = (node: TreeNode, dir: MoveDirection) =>
  run(async () => applyMutation(await api.moveNode(node.relParts, dir), node.relParts));

export const renameTo = (node: TreeNode, editable: string) =>
  run(async () => {
    const text = editable.trim();
    if (text === node.editableName) return;
    await applyMutation(await api.renameNode(node.relParts, text));
  });

export const setType = (node: TreeNode, base: EditableType, status: WkbStatus) =>
  run(async () => {
    const changed =
      node.actions.changeType && (base !== node.nodeType?.base || (base === "WKB" && status !== node.status));
    if (!changed) return;
    await applyMutation(await api.setNodeType(node.relParts, base, base === "WKB" ? status : null), node.relParts);
  });

export const setDetails = (node: TreeNode, status: WkbStatus | null, owner: string | null) =>
  run(async () => applyMutation(await api.setNodeDetails(node.relParts, status, owner), node.relParts));

export const setFavorite = (node: TreeNode, on: boolean) =>
  run(async () => applyMutation(await api.setFavorite(node.relParts, on), node.relParts));

export const setUserFlag = (node: TreeNode, on: boolean) =>
  run(async () => applyMutation(await api.setUserFlag(node.relParts, on), node.relParts));

export const remove = (node: TreeNode) =>
  run(async () => {
    const preview = await api.inspectDelete(node.relParts);
    const nonEmpty = preview.files + preview.dirs > 0;
    if (state.settings.confirmDelete && !preview.ghost) {
      const what = nonEmpty ? `contains ${preview.files} files and ${preview.dirs} folders. They go` : "goes";
      if (!(await confirmDialog("Delete folder", `"${node.name}" ${what} to the Recycle Bin`, "Delete"))) return;
    }
    const m = await api.deleteNode(node.relParts, preview.ghost, nonEmpty);
    const parent = node.relParts.slice(0, -1);
    await applyMutation(m, parent.length > 0 ? parent : null);
  });
