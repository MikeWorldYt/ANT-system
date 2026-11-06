// Re-reads single levels from disk so a focus or a watcher batch never needs the full backend sync.
import * as api from "../services/ipc/api";
import { findNode, hydrate, keyOf, notify, patchLevels, pruneSelection, state } from "../services/store/state";
import { loadVirtual } from "../services/store/virtual";
import type { TreeNode } from "../services/ipc/types";

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
  const known = new Set(loaded.map((n) => n.name));
  const added = children.some((n) => !known.has(n.name));
  const changed = added || children.length !== loaded.length;
  const patched = patchLevels([{ parent, hasChildren: children.length > 0, children }]);
  if (patched) await hydrate(patched[0]);
  return { added, changed };
}

let queue: Promise<unknown> = Promise.resolve();

// Resolves to whether any level gained a folder; levels are read one after another so batches never overlap.
export function relist(parents: string[][]): Promise<boolean> {
  const run = async (): Promise<boolean> => {
    let added = false;
    let changed = false;
    for (const parent of parents) {
      const result = await relistLevel(parent);
      added ||= result.added;
      changed ||= result.changed;
    }
    pruneSelection();
    if (changed) await loadVirtual();
    notify();
    return added;
  };
  const next = queue.then(run, run);
  queue = next.catch(() => undefined);
  return next;
}
