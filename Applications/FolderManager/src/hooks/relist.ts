// Re-reads single levels from disk so a focus or a watcher batch never needs the full backend sync.
import * as api from "../services/ipc/api";
import { findNode, hydrate, keyOf, notify, patchLevels, pruneSelection, state, virtualRoot } from "../services/store/state";
import { keepChildren, loadUnfoldedLists, loadVirtual, unfoldedRoots } from "../services/store/virtual";
import { invalidateSizes } from "../services/store/folder-size";
import { loadOtherVirtual } from "./split";
import { noteListing } from "./file-trips";
import { DOWNLOADS_ROOT, SYSTEM_ROOTS, type TreeNode } from "../services/ipc/types";

// Levels on screen: the root, the open folders and the path down to the selection.
export function visibleLevels(): string[][] {
  const levels: string[][] = [[]];
  const selected = state.selectedKey?.split("/") ?? [];
  const walk = (nodes: TreeNode[]): void => {
    for (const n of nodes) {
      const onSelection = n.relParts.length <= selected.length && n.relParts.every((part, i) => part === selected[i]);
      if (n.children === null || !(state.expanded.has(keyOf(n.relParts)) || onSelection)) continue;
      levels.push(n.relParts);
      walk(n.children);
    }
  };
  walk(state.tree?.nodes ?? []);
  return levels;
}

// A level the UI never loaded is skipped, as it reads fresh when opened.
async function relistLevel(parent: string[]): Promise<{ added: boolean; changed: boolean }> {
  const loaded = parent.length > 0 ? findNode(parent)?.children : state.tree?.nodes;
  if (!loaded) return { added: false, changed: false };
  const children = await api.listChildren(parent);
  noteListing(parent, loaded, children);
  const known = new Set(loaded.map((n) => n.name));
  const added = children.some((n) => !known.has(n.name));
  const changed = added || children.length !== loaded.length;
  const patched = patchLevels([{ parent, hasChildren: children.length > 0, children }]);
  if (patched) await hydrate(patched[0]);
  return { added, changed };
}

let queue: Promise<unknown> = Promise.resolve();

// A native folder nobody has open needs no read when its watcher fires.
const inUse = (root: string): boolean => virtualRoot(state.virtual) === root || unfoldedRoots().includes(root) || [...state.expanded].some((k) => k === root || k.startsWith(`${root}/`));

// Native folders are not in the tree, so their open folders are read from the expanded keys; Downloads' list is read again in any case.
export async function relistSystem(only?: string): Promise<boolean> {
  const roots = (only !== undefined ? [only] : [DOWNLOADS_ROOT, ...SYSTEM_ROOTS]).filter((r) => r === DOWNLOADS_ROOT || inUse(r));
  if (roots.length === 0) return false;
  const open = [...state.expanded].filter((k) => roots.some((r) => k.startsWith(`${r}/`))).map((k) => k.split("/"));
  // The graph lists a native folder itself when its card is open.
  for (const r of roots) if (state.expanded.has(r)) state.system[r] = keepChildren(r, await api.listChildren([r]));
  return relist(open, true);
}

// Resolves to whether any level gained a folder; levels are read one after another so batches never overlap.
export function relist(parents: string[][], reloadLists = false): Promise<boolean> {
  const run = async (): Promise<boolean> => {
    invalidateSizes();
    let added = false;
    let changed = false;
    for (const parent of parents) {
      const result = await relistLevel(parent);
      added ||= result.added;
      changed ||= result.changed;
    }
    pruneSelection();
    // A change that may be metadata only leaves the listing the same size, so the entry lists are read again on request.
    if (changed || reloadLists) {
      await loadVirtual();
      await loadOtherVirtual();
      await loadUnfoldedLists();
    }
    notify();
    return added;
  };
  const next = queue.then(run, run);
  queue = next.catch(() => undefined);
  return next;
}
