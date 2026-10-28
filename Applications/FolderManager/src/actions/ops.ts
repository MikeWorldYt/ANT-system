// User operations: call the backend, replace the snapshot, report errors in the status line.
import * as api from "../ipc/api";
import { warningConfirmDialog } from "../ui/dialogs";
import { el } from "../ui/dom";
import { beginOperationStatus, clearStatus, endOperationStatus, setStatus } from "../ui/status";
import { checkStorage } from "./startup-check";
import { runIntake } from "./intake";
import { applySettings, findNode, hydrate, keyOf, lastSelection, notify, pruneSelection, reveal, select, serializeSetting, setBusy, setTree, state, type Settings } from "../store/state";
import { isFmError } from "../ipc/types";
import type { EditableType, MoveDirection, Mutation, NewKind, TreeNode, TreeSnapshot, WkbStatus } from "../ipc/types";

async function reload(snapshot?: TreeSnapshot): Promise<void> {
  setTree(snapshot ?? (await api.refresh()));
  await hydrate();
  pruneSelection();
}

async function run(fn: () => Promise<void>, progress?: string): Promise<boolean> {
  if (progress) beginOperationStatus(progress);
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
    if (progress) endOperationStatus();
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

export async function refresh(): Promise<boolean> {
  if (state.busy) return false;
  const ok = await run(async () => {
    await reload();
    clearStatus();
  });
  if (ok) void runIntake();
  return ok;
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

export const addNext = async (node: TreeNode, kind: NewKind) => {
  const atRoot = node.relParts.length === 1;
  if (atRoot) {
    if (kind === "workboard") return;
    const confirmed = await warningConfirmDialog(
      "You are about to create a main category!",
      [
        el("p", {},
          "Choose a broad, stable name because Level 1 categories should not be renamed later; linked paths and shortcuts may depend on them. ",
          el("a", { href: "https://ant-system.vercel.app/en/foundationals/folder-structure#level-1", target: "_blank", rel: "noopener noreferrer" }, "Learn more"),
        ),
        "If you proceed, keep more specific folders inside this category.",
      ],
      "Create category",
    );
    if (!confirmed) return;
  }
  const parentName = node.relParts.length > 1 ? node.relParts[node.relParts.length - 2] : "Storage root";
  const progress = atRoot
    ? "Creating a main category in Storage root…"
    : `Creating ${kind === "workboard" ? "workboard" : "node"} in "${parentName}"…`;
  await run(async () => applyMutation(await api.addNode(node.relParts, kind)), progress);
};

export const move = (node: TreeNode, dir: MoveDirection) =>
  run(async () => applyMutation(await api.moveNode(node.relParts, dir), node.relParts), `Moving "${node.name}" ${dir}…`);

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
      if (!(await warningConfirmDialog("Delete folder", [`"${node.name}" ${what} to the Recycle Bin`], "Delete", "danger"))) return;
    }
    beginOperationStatus(`Deleting "${node.name}"…`);
    const m = await api.deleteNode(node.relParts, preview.ghost, nonEmpty);
    const parent = node.relParts.slice(0, -1);
    await applyMutation(m, parent.length > 0 ? parent : null);
  }, "Waiting for user decision…");
