// Single in-memory store for the tree snapshot, selection and expansion.
import { listChildren } from "./api";
import type { TreeNode, TreeSnapshot } from "./types";

interface State {
  tree: TreeSnapshot | null;
  selectedKey: string | null;
  expanded: Set<string>;
  busy: boolean;
}

export const state: State = { tree: null, selectedKey: null, expanded: new Set(), busy: false };

const listeners = new Set<() => void>();

export const keyOf = (parts: string[]) => parts.join("/");

export function subscribe(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function notify(): void {
  listeners.forEach((fn) => fn());
}

export function findNode(parts: string[]): TreeNode | null {
  let level: TreeNode[] = state.tree?.nodes ?? [];
  let found: TreeNode | null = null;
  for (const part of parts) {
    found = level.find((n) => n.name === part) ?? null;
    if (!found) return null;
    level = found.children ?? [];
  }
  return found;
}

export function selectedNode(): TreeNode | null {
  return state.selectedKey === null ? null : findNode(state.selectedKey.split("/"));
}

export function visibleRows(): TreeNode[] {
  const rows: TreeNode[] = [];
  const walk = (nodes: TreeNode[]) => {
    for (const n of nodes) {
      rows.push(n);
      if (state.expanded.has(keyOf(n.relParts)) && n.children) walk(n.children);
    }
  };
  walk(state.tree?.nodes ?? []);
  return rows;
}

export function select(parts: string[] | null): void {
  state.selectedKey = parts ? keyOf(parts) : null;
  notify();
}

export function setBusy(busy: boolean): void {
  state.busy = busy;
  notify();
}

export function setTree(tree: TreeSnapshot): void {
  state.tree = tree;
}

export function pruneSelection(): void {
  if (state.selectedKey !== null && !selectedNode()) state.selectedKey = null;
}

// Loads children of every expanded node whose children are not fetched yet.
export async function hydrate(): Promise<void> {
  const walk = async (nodes: TreeNode[]): Promise<void> => {
    for (const n of nodes) {
      if (!state.expanded.has(keyOf(n.relParts))) continue;
      if (n.children === null && n.hasChildren) n.children = await listChildren(n.relParts);
      await walk(n.children ?? []);
    }
  };
  await walk(state.tree?.nodes ?? []);
}
