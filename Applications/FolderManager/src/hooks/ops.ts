// User operations: call the backend, patch or replace the tree, report errors in the status line.
import * as api from "../services/ipc/api";
import { choiceDialog, warningConfirmDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { el } from "../components/dom";
import { beginOperationStatus, clearStatus, endOperationStatus, setStatus, reportError } from "../components/organisms/bars/status-line";
import { markRefreshed } from "./auto-refresh";
import { checkStorage } from "./startup-check";
import { runIntake } from "./intake";
import { checkRootFolders } from "./root-check";
import { relist, visibleLevels } from "./relist";
import { dropNotice, type Notice } from "./drop-summary";
import { applySettings, findNode, followExpanded, hydrate, isVirtualShown, keyOf, lastSelection, leaveVirtual, listed, notify, outermost, patchLevels, pruneSelection, reveal, select, selectedNode, serializeSetting, setBusy, setTree, sortOf, state, type Settings } from "../services/store/state";
import { sortedForColumns } from "../services/store/sort";
import { loadVirtual } from "../services/store/virtual";
import { outputOf } from "../services/store/node-info";
import { goToPlace, toggleBoard } from "./virtual-nav";
import { isDelivery, pickInside, todayStamp, type Delivery } from "./outputs";
import { inDownloads, isFmError } from "../services/ipc/types";
import { plural } from "../services/plural";
import type { BoardCard, Copied, EditableType, HiddenChoice, LevelPatch, MoveDirection, Mutation, NewKind, Resolution, TreeNode, TreeSnapshot, WkbStatus } from "../services/ipc/types";

async function reload(snapshot?: TreeSnapshot, full = true): Promise<void> {
  markRefreshed();
  const tree = snapshot ?? (await api.refresh(full));
  setTree(tree);
  if (tree.warnings.length > 0) setStatus("warning", tree.warnings.join(". "));
  followExpanded();
  await hydrate();
  await loadVirtual();
  pruneSelection();
  await askAboutCopies(tree.copies);
}

// Shown once the operation succeeds, because callers clear the status line right after a reload.
let unresolvedCopies: string | null = null;
let doneNotice: Notice | null = null;

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
  await loadVirtual();
  pruneSelection();
}

async function run(fn: () => Promise<void>, progress?: string): Promise<boolean> {
  if (progress) beginOperationStatus(progress);
  setBusy(true);
  try {
    await fn();
    if (doneNotice) {
      setStatus(doneNotice.level, doneNotice.text);
      doneNotice = null;
    }
    if (unresolvedCopies) {
      setStatus("warning", unresolvedCopies);
      unresolvedCopies = null;
    }
    return true;
  } catch (e) {
    // An empty clipboard is a user slip, not a failure.
    reportError(isFmError(e) && e.kind === "clipboardEmpty" ? "warning" : "error", e);
    if (isFmError(e) && (e.kind === "stale" || e.kind === "notFound")) {
      try {
        await reload();
      } catch (reloadError) {
        reportError("error", reloadError);
      }
    }
    return false;
  } finally {
    if (progress) endOperationStatus();
    setBusy(false);
  }
}

// A move or copy passes follow=false, so the view stays on the folder the user was in.
async function applyMutation(m: Mutation, fallback: string[] | null = null, follow = true): Promise<void> {
  const target = follow ? (m.select ?? fallback) : null;
  if (target) {
    for (let i = 1; i < target.length; i++) state.expanded.add(keyOf(target.slice(0, i)));
    select(target);
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
      reportError("warning", e, "Settings");
    }
    try {
      const { depthWarning } = await api.getStorage();
      if (depthWarning) setStatus("warning", depthWarning);
    } catch (e) {
      reportError("warning", e, "Storage info");
    }
    await reload(await api.loadTree());
    const last = state.settings.onLaunch === "continue" ? lastSelection() : null;
    if (!last) return;
    if (last.key) await reveal(last.key.split("/"));
    if (last.virtual === "board") await toggleBoard();
    else if (last.virtual !== null && isVirtualShown(last.virtual)) await goToPlace(last);
  });
}

export const changeSetting = <K extends keyof Settings>(key: K, value: Settings[K]) =>
  run(async () => {
    await api.setSetting(key, serializeSetting(value));
    state.settings[key] = value;
    // An entry hidden while it is open would leave its list on screen, so the view goes back to the root.
    if (state.virtual !== null && !isVirtualShown(state.virtual)) {
      leaveVirtual();
      select(null);
    }
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

// Automatic callers pass full=false so only the manual refresh rebuilds the indexes.
export async function refresh(full = true): Promise<boolean> {
  if (state.busy) return false;
  const ok = await run(async () => {
    await reload(undefined, full);
    clearStatus();
  });
  if (ok) {
    void runIntake(reload);
    void checkRootFolders(resolveInvalid, deleteInvalid);
  }
  return ok;
}

// Folders the backend has no say over are listed from disk; the intake checks only run when one actually appeared.
export async function relistLevels(folders: string[][] | null): Promise<void> {
  if (state.busy) return;
  try {
    const added = await relist(folders ?? visibleLevels());
    if (added && folders !== null) {
      void runIntake(reload);
      void checkRootFolders(resolveInvalid, deleteInvalid);
    }
  } catch (e) {
    reportError("warning", e, "Refresh");
  }
}

export async function expand(node: TreeNode): Promise<void> {
  const key = keyOf(node.relParts);
  state.expanded.add(key);
  notify();
  try {
    await hydrate();
  } catch (e) {
    state.expanded.delete(key);
    reportError("error", e);
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

export const addChild = (node: TreeNode, kind: NewKind) =>
  run(
    async () => applyMutation(await api.addChild(node.relParts, kind)),
    `Creating ${kind === "workboard" ? "workboard" : "node"} in "${node.name}"…`,
  );

export const move = (node: TreeNode, dir: MoveDirection) =>
  run(async () => applyMutation(await api.moveNode(node.relParts, dir), node.relParts), `Moving "${node.name}" ${dir}…`);

export const renameTo = (node: TreeNode, editable: string) =>
  run(async () => {
    const text = editable.trim();
    if (text === node.editableName) return;
    await applyMutation(node.file ? await api.renameFile(node.relParts, text) : await api.renameNode(node.relParts, text));
  });

export async function setType(node: TreeNode, base: EditableType, status: WkbStatus): Promise<void> {
  const ok = await run(async () => {
    const changed =
      node.actions.changeType && (base !== node.nodeType?.base || (base === "WKB" && status !== node.status));
    if (!changed) return;
    await applyMutation(await api.setNodeType(node.relParts, base, base === "WKB" ? status : null), node.relParts);
  });
  // A workboard with a name outside the track format is announced now, since the check only runs on refresh.
  if (ok && base === "WKB") void checkRootFolders(resolveInvalid, deleteInvalid);
}

// The nearest folder that stays listed takes the selection, so the columns do not close.
export const hideNode = (node: TreeNode) =>
  run(async () => {
    const parent = node.relParts.slice(0, -1);
    const sibling = nearestSibling(node);
    const m = await api.setNodeHidden(node.relParts, true);
    await applyMutation({ ...m, select: sibling ?? (parent.length > 0 ? parent : null) }, parent.length > 0 ? parent : null);
  });

// The marked items all leave the view, so the selection falls back to the first one's parent.
export const hideMany = (nodes: TreeNode[]) =>
  run(async () => {
    const parent = nodes[0]?.relParts.slice(0, -1) ?? [];
    for (const node of nodes) {
      const m = await api.setNodeHidden(node.relParts, true);
      await applyMutation({ ...m, select: parent.length > 0 ? parent : null }, parent.length > 0 ? parent : null);
    }
  });

// The side panel lists the folder's children with a checkbox each, and every checkbox applies at once.
export async function manageHidden(parent: string[]): Promise<void> {
  state.managing = parent;
  state.hiddenChoices = [];
  notify();
  await refreshHiddenChoices();
}

export function stopManaging(): void {
  state.managing = null;
  state.hiddenChoices = [];
  notify();
}

async function refreshHiddenChoices(): Promise<void> {
  const parent = state.managing;
  if (!parent) return;
  await run(async () => {
    const choices: HiddenChoice[] = await api.listHiddenChoices(parent);
    if (state.managing !== parent) return;
    state.hiddenChoices = choices;
    notify();
  });
}

// The row already shows the new state, so the list is not rebuilt; the choice is updated to match.
export async function setChildHidden(name: string, on: boolean): Promise<boolean> {
  const parent = state.managing;
  if (!parent) return false;
  const saved = await run(async () => applyMutation(await api.setNodeHidden([...parent, name], on), null, false));
  if (saved) {
    const choice = state.hiddenChoices.find((c) => c.node.name === name);
    if (choice) choice.hidden = on;
  }
  return saved;
}

export const setOutcomeDefault = (node: TreeNode, key: string, value: string | null) =>
  run(async () => applyMutation(await api.setOutcomeDefault(node.relParts, key, value), node.relParts));

export const setOutput = (node: TreeNode, output: string | null) =>
  run(async () => applyMutation(await api.setNodeOutput(node.relParts, output), node.relParts));

// Finishing a workboard delivers its output, asking first when none is set (where nothing is a valid answer); reopening it brings the delivery back.
export async function setDetails(node: TreeNode, status: WkbStatus | null, owner: string | null): Promise<boolean> {
  const current = findNode(node.relParts) ?? node;
  const finishing = status === "done" && current.status !== "done";
  const reopening = status !== null && status !== "done" && current.status === "done";
  let delivery: Delivery | null = null;
  if (finishing && outputOf(current) === null) {
    const choice = await choiceDialog("Hmmm, Where is the final file?", "Output dir is invalid or missing. What do you want to do?", [
      ["file", "Choose a file"],
      ["folder", "Choose a folder"],
      ["all", "Move everything"],
      ["none", "Do nothing"],
    ]);
    if (!isDelivery(choice)) return false;
    delivery = choice;
  }
  if (delivery === "file" || delivery === "folder") {
    const picked = await pickInside(current, delivery === "folder");
    if (!picked) return false;
    if (picked.relative === null) {
      setStatus("warning", "Pick a file or folder inside the workboard");
      return false;
    }
    if (!(await setOutput(current, picked.relative))) return false;
  }
  const saved = await run(async () => applyMutation(await api.setNodeDetails(current.relParts, status, owner), current.relParts));
  if (!saved) return false;
  if (finishing && delivery !== "none") await run(async () => applyMutation(await api.deliverOutput(current.relParts, todayStamp(), delivery === "all"), current.relParts));
  else if (reopening) await run(async () => applyMutation(await api.restoreOutput(current.relParts), current.relParts));
  return true;
}

export const setFavorite = (node: TreeNode, on: boolean) =>
  run(async () => applyMutation(await api.setFavorite(node.relParts, on), node.relParts));

export const setFavorites = (nodes: TreeNode[], on: boolean) =>
  run(async () => {
    for (const node of nodes.filter((n) => n.favorite !== on)) await applyMutation(await api.setFavorite(node.relParts, on), node.relParts, false);
  });

// Each workboard goes through the single-item path, so finishing one still asks for its output.
export async function setManyDetails(nodes: TreeNode[], status: WkbStatus | null, owner: string | null): Promise<void> {
  for (const node of nodes) if (!(await setDetails(node, status, owner))) return;
}

// Several marked workboards are flagged in one go; a lone one is also selected, like a favorite.
export const setImportant = (nodes: TreeNode[], on: boolean) =>
  run(async () => {
    for (const node of nodes.filter((n) => n.important !== on)) await applyMutation(await api.setImportant(node.relParts, on), node.relParts, nodes.length === 1);
  });

// Placing cards never moves the selection, so the board keeps the card the user is on.
export const placeOnBoard = (cards: BoardCard[]) => run(async () => applyMutation(await api.setBoard(cards)));

export const setLinkHidden = (node: TreeNode, on: boolean) =>
  run(async () => applyMutation(await api.setLinkHidden(node.relParts, on), node.relParts));

const REVIEW_PATH = ["998.Review"];

// The review list in the order its column shows it.
function reviewListing(): string[] {
  const children = findNode(REVIEW_PATH)?.children ?? [];
  return sortedForColumns(listed(children), sortOf(keyOf(REVIEW_PATH)), state.settings.pinFavorites).map((n) => keyOf(n.relParts));
}

export const setReviewFilter = (hidden: string[], favorites: boolean) =>
  run(async () => {
    const before = reviewListing();
    const selected = state.selectedKey;
    const index = selected === null ? -1 : before.indexOf(selected);
    await applyMutation(await api.setReviewFilter(hidden, favorites));
    if (selected === null || selectedNode() !== null) return;
    // A hidden selection moves to the nearest link still listed, or to the review folder when none is left.
    const remaining = new Set(reviewListing());
    const nearest = before.slice(index + 1).find((k) => remaining.has(k)) ?? before.slice(0, Math.max(index, 0)).reverse().find((k) => remaining.has(k));
    select(nearest ? nearest.split("/") : REVIEW_PATH);
  });

export const setUserFlag = (node: TreeNode, on: boolean) =>
  run(async () => applyMutation(await api.setUserFlag(node.relParts, on), node.relParts));

// What comes in from outside may break the storage's rules, so the refresh checks run right after it lands, keeping the drop notice.
async function revalidate(): Promise<void> {
  const ok = await run(reload);
  if (!ok) return;
  void runIntake(reload);
  void checkRootFolders(resolveInvalid, deleteInvalid);
}

export const dropFiles = async (target: string[], paths: string[]): Promise<void> => {
  if (inDownloads(target)) return setStatus("warning", "Downloads takes no files from here");
  const ok = await run(
    async () => {
      const { mutation, placed, conflicts } = await api.dropFiles(target, paths);
      await applyMutation(mutation);
      doneNotice = dropNotice(placed, mutation.warnings);
      // Each workboard already in the storage gets its own question; only the ones answered "Replace" come back.
      const replace: string[] = [];
      for (const conflict of conflicts) {
        const location = el("a", { href: "#" }, conflict.existing.join("/"));
        location.addEventListener("click", (ev) => {
          ev.preventDefault();
          void api.openNode(conflict.existing).catch((e) => reportError("warning", e));
        });
        // Replacing lands the new one where it was dropped, which is a new path unless that is the folder it already holds.
        const newPath = conflict.existing.slice(0, -1).join("/") !== target.join("/");
        const confirmed = await warningConfirmDialog(
          "Workboard already exists",
          [
            el("p", {}, `"${conflict.name}" is already in the storage at "`, location, `".`),
            el("p", {}, "If you replace it, you send that one (current in your Storage) to the Recycle Bin.", newPath ? " It will also end up at a new path." : ""),
            el("p", {}, "Make sure the new one is a recent version and the old one has no files you could lose."),
          ],
          "Replace",
          "danger",
        );
        if (confirmed) replace.push(conflict.source);
      }
      if (replace.length > 0) {
        const swapped = await api.dropFiles(target, replace, replace);
        await applyMutation(swapped.mutation);
        doneNotice = dropNotice([...placed, ...swapped.placed], swapped.mutation.warnings);
      }
    },
    `Placing ${plural(paths.length, "file")} in "${target[target.length - 1]}"…`,
  );
  if (ok) await revalidate();
};

export const pasteFiles = (target: string[]) =>
  inDownloads(target) ? Promise.resolve(setStatus("warning", "Downloads takes no files from here")) : run(
    async () => {
      const { mutation, placed } = await api.pasteFiles(target);
      await applyMutation(mutation);
      doneNotice = dropNotice(placed, mutation.warnings);
    },
    `Pasting files in "${target[target.length - 1]}"…`,
  ).then(async (ok) => {
    if (ok) await revalidate();
  });

// Files already in the storage move (or copy) into a folder, with the same placement as files dropped from Explorer.
export const moveFiles = (sources: string[][], target: string[], copy: boolean) =>
  run(
    async () => {
      if (!copy) selectionAfterMoving(sources);
      const { mutation, placed } = await api.moveFilesIn(sources, target, copy);
      await applyMutation(mutation);
      doneNotice = dropNotice(placed, mutation.warnings);
    },
    `${copy ? "Copying" : "Moving"} files to "${target[target.length - 1]}"…`,
  ).then(async (ok) => {
    if (ok) await revalidate();
  });

// A folder moved out of the storage on purpose loses its record, instead of staying listed as missing.
export const forgetMoved = (sources: string[][]) =>
  run(async () => {
    for (const parts of sources) {
      if (!findNode(parts)?.ghost) continue;
      await applyMutation(await api.deleteNode(parts, true, false), null, false);
    }
  });

// Folders move one at a time, each on its own mutation; a failed one is reported without stopping the rest.
export const moveFolders = (sources: string[][], destination: string[], copy: boolean) =>
  run(
    async () => {
      if (!copy) selectionAfterMoving(sources);
      const failures: string[] = [];
      for (const source of sources) {
        try {
          await applyMutation(await api.moveFolder(source, destination, copy), null, false);
        } catch (e) {
          failures.push(api.describeError(e));
        }
      }
      if (failures.length > 0) setStatus("error", failures.join(". "));
    },
    `${copy ? "Copying" : "Moving"} ${sources.length === 1 ? `"${sources[0][sources[0].length - 1]}"` : `${sources.length} folders`} to "${destination[destination.length - 1]}"…`,
  ).then(async (ok) => {
    if (ok) await revalidate();
  });

export const resolveInvalid = (relParts: string[], resolution: Resolution) =>
  run(
    async () => applyMutation(await api.resolveInvalid(relParts, resolution)),
    `Fixing "${relParts[relParts.length - 1]}"…`,
  );

// The invalid folder may not be in the loaded tree, so the backend inspects it and the dialog asks like a tree delete does.
export async function deleteInvalid(relParts: string[]): Promise<boolean> {
  const name = relParts[relParts.length - 1] ?? "";
  let deleted = false;
  const ok = await run(async () => {
    const preview = await api.inspectDelete(relParts);
    const nonEmpty = preview.files + preview.dirs > 0;
    if (state.settings.confirmDelete && !preview.ghost) {
      const what = nonEmpty ? `contains ${preview.files} files and ${preview.dirs} folders. They go` : "goes";
      if (!(await warningConfirmDialog("Delete folder", [`"${name}" ${what} to the Recycle Bin`], "Delete", "danger"))) return;
    }
    beginOperationStatus(`Deleting "${name}"…`);
    await applyMutation(await api.deleteNode(relParts, preview.ghost, nonEmpty));
    deleted = true;
  }, "Waiting for user decision…");
  return ok && deleted;
}

// The folder after it in its column as the user sees it, else the one before; null when it is the only one shown.
// The folder after it in its column as the user sees it, else the one before; items that are leaving are skipped.
function nearestSibling(node: TreeNode, leaving: string[][] = []): string[] | null {
  const parent = node.relParts.slice(0, -1);
  const level = parent.length > 0 ? (findNode(parent)?.children ?? []) : (state.tree?.nodes ?? []);
  const shown = sortedForColumns(listed(level), sortOf(keyOf(parent)), state.settings.pinFavorites);
  const at = shown.findIndex((n) => n.name === node.name);
  if (at < 0) return null;
  const gone = new Set(leaving.map(keyOf));
  for (let distance = 1; distance < shown.length; distance++) {
    const after = shown[at + distance];
    const before = shown[at - distance];
    if (after && !gone.has(keyOf(after.relParts))) return after.relParts;
    if (before && !gone.has(keyOf(before.relParts))) return before.relParts;
  }
  return null;
}

// A moved selection leaves nothing selected, so the nearest item that stays takes it and the columns stay open.
function selectionAfterMoving(moving: string[][]): void {
  const key = state.selectedKey;
  if (key === null || !moving.some((parts) => keyOf(parts) === key)) return;
  const node = selectedNode();
  if (!node) return;
  const sibling = nearestSibling(node, moving);
  const parent = node.relParts.slice(0, -1);
  select(sibling ?? (parent.length > 0 ? parent : null));
}

// Files have no contents to count, so they only need the confirmation and the sibling choice.
async function removeFile(node: TreeNode): Promise<void> {
  if (state.settings.confirmDelete && !(await warningConfirmDialog("Delete file", [`"${node.name}" goes to the Recycle Bin`], "Delete", "danger"))) return;
  beginOperationStatus(`Deleting "${node.name}"…`);
  const sibling = nearestSibling(node);
  await applyMutation(await api.deleteFile(node.relParts), sibling ?? node.relParts.slice(0, -1));
}

export const remove =(node: TreeNode) =>
  run(async () => {
    if (node.file) return removeFile(node);
    const preview = await api.inspectDelete(node.relParts);
    const nonEmpty = preview.files + preview.dirs > 0;
    if (state.settings.confirmDelete && !preview.ghost) {
      const what = nonEmpty ? `contains ${preview.files} files and ${preview.dirs} folders. They go` : "goes";
      if (!(await warningConfirmDialog("Delete folder", [`"${node.name}" ${what} to the Recycle Bin`], "Delete", "danger"))) return;
    }
    beginOperationStatus(`Deleting "${node.name}"…`);
    const sibling = nearestSibling(node);
    const m = await api.deleteNode(node.relParts, preview.ghost, nonEmpty);
    const parent = node.relParts.slice(0, -1);
    await applyMutation(m, sibling ?? (parent.length > 0 ? parent : null));
  }, "Waiting for user decision…");

// One confirmation for the batch; each item is then deleted on its own, so one refusal does not stop the rest.
export const removeMany = (nodes: TreeNode[]) =>
  run(async () => {
    const keep = new Set(outermost(nodes.map((n) => n.relParts)));
    const picked = nodes.filter((n) => keep.has(n.relParts));
    if (state.settings.confirmDelete && !(await warningConfirmDialog("Delete items", [`${picked.length} items go to the Recycle Bin. Folders go with their contents.`], "Delete", "danger"))) return;
    beginOperationStatus(`Deleting ${picked.length} items…`);
    const failures: string[] = [];
    for (const node of picked) {
      try {
        if (node.file) {
          await applyMutation(await api.deleteFile(node.relParts));
        } else {
          const preview = await api.inspectDelete(node.relParts);
          await applyMutation(await api.deleteNode(node.relParts, preview.ghost, preview.files + preview.dirs > 0));
        }
      } catch (e) {
        failures.push(api.describeError(e));
      }
    }
    if (failures.length > 0) setStatus("error", failures.join(". "));
  }, "Waiting for user decision…");