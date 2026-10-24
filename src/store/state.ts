// Single in-memory store for the tree snapshot, selection and expansion.
import { listChildren } from "../ipc/api";
import { favoritesFirst, SORT_BYS, type ColumnSort, type SortBy } from "./sort";
import { readPref, savePref } from "./prefs";
import type { TreeNode, TreeSnapshot } from "../ipc/types";

interface State {
  tree: TreeSnapshot | null;
  selectedKey: string | null;
  expanded: Set<string>;
  busy: boolean;
  editing: { key: string; draft: string; focused: boolean } | null;
  // Order of each column, keyed by its parent's path ("" for the first one); the tree view has its own.
  sorts: Record<string, ColumnSort>;
  settings: Settings;
}

export interface Settings {
  pinFavorites: boolean;
  hideDone: boolean;
}

export type { ColumnSort, SortBy };

const SORTS_PREF = "fm.sorts";
const DEFAULT_SORT: ColumnSort = { by: "name", dir: "asc" };

function readSorts(): Record<string, ColumnSort> {
  try {
    const parsed: unknown = JSON.parse(readPref(SORTS_PREF) ?? "{}");
    if (typeof parsed !== "object" || parsed === null) return {};
    const sorts: Record<string, ColumnSort> = {};
    for (const [key, value] of Object.entries(parsed)) {
      const { by, dir } = (value ?? {}) as Partial<ColumnSort>;
      if (SORT_BYS.includes(by as SortBy) && (dir === "asc" || dir === "desc")) sorts[key] = { by: by as SortBy, dir };
    }
    return sorts;
  } catch (e) {
    console.warn("Saved column order is unreadable", e);
    return {};
  }
}

export const state: State = {
  tree: null,
  selectedKey: null,
  expanded: new Set(),
  busy: false,
  editing: null,
  sorts: readSorts(),
  settings: { pinFavorites: true, hideDone: false },
};

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

export const listed = (nodes: TreeNode[]): TreeNode[] =>
  state.settings.hideDone ? nodes.filter((n) => n.status !== "done") : nodes;

export function visibleRows(): TreeNode[] {
  const rows: TreeNode[] = [];
  const walk = (nodes: TreeNode[]) => {
    for (const n of favoritesFirst(listed(nodes), state.settings.pinFavorites)) {
      rows.push(n);
      if (state.expanded.has(keyOf(n.relParts)) && n.children) walk(n.children);
    }
  };
  walk(state.tree?.nodes ?? []);
  return rows;
}

export function applySettings(saved: Record<string, string>): void {
  state.settings.pinFavorites = saved.pinFavorites !== "false";
  state.settings.hideDone = saved.hideDone === "true";
}

export const sortOf = (columnKey: string): ColumnSort => state.sorts[columnKey] ?? DEFAULT_SORT;

// The column the selection stands in, or the first one when nothing is selected.
export const activeColumnKey = (): string => state.selectedKey?.split("/").slice(0, -1).join("/") ?? "";

export function setColumnSort(columnKey: string, patch: Partial<ColumnSort>): void {
  state.sorts[columnKey] = { ...sortOf(columnKey), ...patch };
  savePref(SORTS_PREF, JSON.stringify(state.sorts));
  notify();
}

export function select(parts: string[] | null): void {
  state.selectedKey = parts ? keyOf(parts) : null;
  notify();
}

export function startEdit(parts: string[]): void {
  const node = findNode(parts);
  if (!node?.actions.rename || state.busy) return;
  state.editing = { key: keyOf(parts), draft: node.editableName, focused: false };
  select(parts);
}

export function stopEdit(): void {
  state.editing = null;
  notify();
}

// Expands every ancestor, loads what is missing and selects the folder.
export async function reveal(parts: string[]): Promise<void> {
  for (let i = 1; i < parts.length; i++) state.expanded.add(keyOf(parts.slice(0, i)));
  await hydrate();
  select(parts.length > 0 ? parts : null);
}

export function setBusy(busy: boolean): void {
  state.busy = busy;
  notify();
}

export function setTree(tree: TreeSnapshot): void {
  state.tree = tree;
}

export function pruneSelection(): void {
  if (state.editing && !findNode(state.editing.key.split("/"))) state.editing = null;
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
