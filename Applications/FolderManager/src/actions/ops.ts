// User operations: call the backend, patch or replace the tree, report errors in the status line.
import * as api from "../ipc/api";
import { choiceDialog, warningConfirmDialog } from "../ui/dialogs";
import { el } from "../ui/dom";
import { beginOperationStatus, clearStatus, endOperationStatus, setStatus } from "../ui/status";
import { checkStorage } from "./startup-check";
import { runIntake } from "./intake";
import { applySettings, findNode, hydrate, keyOf, lastSelection, notify, patchLevels, pruneSelection, reveal, select, serializeSetting, setBusy, setTree, state, type Settings } from "../store/state";
import { isFmError } from "../ipc/types";
import type { Copied, EditableType, LevelPatch, MoveDirection, Mutation, NewKind, TreeNode, TreeSnapshot, WkbStatus } from "../ipc/types";

async function reload(snapshot?: TreeSnapshot): Promise<void> {
  const tree = snapshot ?? (await api.refresh());
  setTree(tree);
  await hydrate();
  pruneSelection();
  await askAboutCopies(tree.copies);
}

// Shown once the operation succeeds, because callers clear the status line right after a reload.
let unresolvedCopies: string | null = null;

// The backend marks each copy only once, so a dismissed question leaves the copy marked and is not asked again.
async function askAboutCopies(copies: Copied[]): Promise<void> {
  const dismissed: string[] = [];
  for (const { relParts, original } of copies) {
    const answer = await choiceDialog(
      "Copied folder found",
      `"${relParts.join("/")}" looks like a copy of "${original.join("/")}". Is it a copy, or a new, independent project?`,
      [["copy", "It is a copy"], ["independent", "New project"]],
    );
    if (answer === "copy" || answer === "independent") await api.answerCopy(relParts, answer);
    else dismissed.push(`"${relParts.join("/")}"`);
  }
  if (dismissed.length > 0) unresolvedCopies = `Unresolved copy: ${dismissed.join(", ")} stays marked as a copy of another folder.`;
}

// A level whose parent the UI never loaded cannot be patched, so the tree is read again.
async function patch(levels: LevelPatch[]): Promise<void> {
  const patched = patchLevels(levels);
  if (!patched) return reload();
  for (const nodes of patched) await hydrate(nodes);
  pruneSelection();
}

async function run(fn: () => Promise<void>, progress?: string): Promise<boolean> {
  if (progress) beginOperationStatus(progress);
  setBusy(true);
  try {
    await fn();
    if (unresolvedCopies) {
      setStatus("warning", unresolvedCopies);
      unresolvedCopies = null;
    }
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
  if (m.tree === null) await patch(m.levels);
  // The backend may flag a yaml change and leave the node list empty; the tree then has to be read again.
  else if (m.tree.nodes.length === 0 && m.tree.yamlChanged) await reload();
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
    try {
      const { depthWarning } = await api.getStorage();
      if (depthWarning) setStatus("warning", depthWarning);
    } catch (e) {
      setStatus("warning", `Storage info: ${api.describeError(e)}`);
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

// The tree is read again at once, as the depth decides what it holds.
export const changeTreeDepth = (depth: number) =>
  run(async () => {
    await api.setSetting("treeDepth", String(depth));
    await reload();
    clearStatus();
  });

export const resetPreferences = () =>
  run(async () => {
    const wasLogging = state.settings.debugLogging;
    await api.resetSettings();
    applySettings({});
    if (wasLogging) await api.stopDebugLog();
    await reload();
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
