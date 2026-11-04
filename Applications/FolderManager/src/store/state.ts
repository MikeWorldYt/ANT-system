// Single in-memory store for the tree snapshot, selection and expansion.
import { listChildren } from "../ipc/api";
import { favoritesFirst, foldersFirst, SORT_BYS, type ColumnSort, type SortBy } from "./sort";
import { readPref, savePref } from "./prefs";
import type { HiddenChoice, LevelPatch, TreeNode, TreeSnapshot } from "../ipc/types";

interface State {
  tree: TreeSnapshot | null;
  selectedKey: string | null;
  // Every selected key, the primary one included; a plain click leaves just that one.
  selectedKeys: string[];
  // Folder the last click in the views landed on, "" for the storage root; Ctrl+V pastes there.
  pasteKey: string | null;
  expanded: Set<string>;
  busy: boolean;
  // Lists every review link so the hidden ones can be selected again, instead of only the shown ones.
  hidingLinks: boolean;
  // The folder whose children the side panel lists for hiding, with each child's hidden state.
  managing: string[] | null;
  hiddenChoices: HiddenChoice[];
  editing: { key: string; draft: string; focused: boolean } | null;
  // Order of each column, keyed by its parent's path ("" for the first one); the tree view has its own.
  sorts: Record<string, ColumnSort>;
  settings: Settings;
  // The open virtual root entry and the real folders it lists, gathered from the whole Storage.
  virtual: VirtualId | null;
  virtualKeys: string[];
  // The listed entry the selection went through; inside FAVORITES the path stays symbolic below it.
  via: string | null;
}

export type VirtualId = "favorites" | "board";

export interface KeyCombo {
  code: string;
  mod: boolean;
  alt: boolean;
}

export type OnLaunch = "continue" | "root";
export type BackupFrequency = "day" | "week" | "month";
export type Theme = "system" | "light" | "dark";
export type AccentId = "green" | "blue" | "purple" | "coral" | "gold";
export type TextSize = "small" | "medium" | "large";
export type StartView = "continue" | "tree" | "columns";

export interface Settings {
  pinFavorites: boolean;
  hideDone: boolean;
  hideDirectoryIdentifiers: boolean;
  confirmDelete: boolean;
  autoUpdate: boolean;
  debugLogging: boolean;
  backupAuto: boolean;
  watchExternal: boolean;
  autoOpenColumns: boolean;
  theme: Theme;
  accent: AccentId;
  iconColor: AccentId;
  textSize: TextSize;
  backupFrequency: BackupFrequency;
  // Empty means the default folder inside the Storage.
  backupFolder: string;
  onLaunch: OnLaunch;
  startView: StartView;
  // Only the shortcuts the user changed; the rest keep their defaults.
  shortcuts: Record<string, KeyCombo>;
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
  selectedKeys: [],
  pasteKey: null,
  expanded: new Set(),
  busy: false,
  hidingLinks: false,
  managing: null,
  hiddenChoices: [],
  editing: null,
  sorts: readSorts(),
  settings: { pinFavorites: true, hideDone: false, hideDirectoryIdentifiers: false, confirmDelete: true, autoUpdate: true, debugLogging: false, backupAuto: false, watchExternal: false, autoOpenColumns: false, theme: "dark", accent: "green", iconColor: "purple", textSize: "small", backupFrequency: "day", backupFolder: "", onLaunch: "continue", startView: "continue", shortcuts: {} },
  virtual: null,
  virtualKeys: [],
  via: null,
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

export const isSelected = (key: string): boolean => state.selectedKeys.includes(key);

export function selectedNodes(): TreeNode[] {
  return state.selectedKeys.flatMap((key) => findNode(key.split("/")) ?? []);
}

// What a command acts on: every marked item when several are marked, else the item on the path.
export function actedNodes(): TreeNode[] {
  if (state.selectedKeys.length > 1) return selectedNodes();
  const n = selectedNode();
  return n ? [n] : [];
}

// A folder's selected contents travel with it, so only the outermost picks count.
export const outermost = (paths: string[][]): string[][] =>
  paths.filter((p) => !paths.some((o) => o.length < p.length && o.every((part, i) => part === p[i])));

export const listed = (nodes: TreeNode[]): TreeNode[] => {
  const shown = state.hidingLinks ? nodes : nodes.filter((n) => !n.hidden);
  return state.settings.hideDone ? shown.filter((n) => n.status !== "done" || inSelectedPath(n)) : shown;
};

export function toggleHidingLinks(): void {
  state.hidingLinks = !state.hidingLinks;
  notify();
}

const inSelectedPath = (n: TreeNode): boolean => {
  const key = keyOf(n.relParts);
  return state.selectedKey === key || (state.selectedKey?.startsWith(`${key}/`) ?? false);
};

export function visibleRows(nodes: TreeNode[] = state.tree?.nodes ?? []): TreeNode[] {
  const rows: TreeNode[] = [];
  const walk = (level: TreeNode[]) => {
    for (const n of foldersFirst(favoritesFirst(listed(level), state.settings.pinFavorites))) {
      rows.push(n);
      if (state.expanded.has(keyOf(n.relParts)) && n.children) walk(n.children);
    }
  };
  walk(nodes);
  return rows;
}

export const serializeSetting = (value: Settings[keyof Settings]): string =>
  typeof value === "object" ? JSON.stringify(value) : String(value);

function readShortcuts(raw: string | undefined): Record<string, KeyCombo> {
  if (!raw) return {};
  try {
    return JSON.parse(raw) as Record<string, KeyCombo>;
  } catch (e) {
    console.warn("Saved shortcuts are unreadable", e);
    return {};
  }
}

export function applySettings(saved: Record<string, string>): void {
  state.settings.pinFavorites = saved.pinFavorites !== "false";
  state.settings.hideDone = saved.hideDone === "true";
  state.settings.hideDirectoryIdentifiers = saved.hideDirectoryIdentifiers === "true";
  state.settings.confirmDelete = saved.confirmDelete !== "false";
  state.settings.autoUpdate = saved.autoUpdate !== "false";
  state.settings.debugLogging = saved.debugLogging === "true";
  state.settings.backupAuto = saved.backupAuto === "true";
  state.settings.watchExternal = saved.watchExternal === "true";
  state.settings.autoOpenColumns = saved.autoOpenColumns === "true";
  state.settings.theme = saved.theme === "light" || saved.theme === "system" ? saved.theme : "dark";
  state.settings.accent = ["blue", "purple", "coral", "gold"].includes(saved.accent ?? "") ? (saved.accent as AccentId) : "green";
  state.settings.iconColor = ["green", "blue", "coral", "gold"].includes(saved.iconColor ?? "") ? (saved.iconColor as AccentId) : "purple";
  state.settings.textSize = saved.textSize === "medium" || saved.textSize === "large" ? saved.textSize : "small";
  state.settings.backupFrequency = saved.backupFrequency === "week" || saved.backupFrequency === "month" ? saved.backupFrequency : "day";
  state.settings.backupFolder = saved.backupFolder ?? "";
  state.settings.onLaunch = saved.onLaunch === "root" ? "root" : "continue";
  state.settings.startView = saved.startView === "tree" || saved.startView === "columns" ? saved.startView : "continue";
  state.settings.shortcuts = readShortcuts(saved.shortcuts);
}

const LAST_PREF = "fm.last";

export function rememberSelection(): void {
  if (!state.tree) return;
  savePref(LAST_PREF, JSON.stringify({ root: state.tree.root, key: state.selectedKey }));
}

// The folder that was selected when the app closed, if it belongs to the Storage now open.
export function lastSelection(): string[] | null {
  const raw = readPref(LAST_PREF);
  if (!raw || !state.tree) return null;
  try {
    const saved = JSON.parse(raw) as { root?: string; key?: string | null };
    return saved.root === state.tree.root && saved.key ? saved.key.split("/") : null;
  } catch (e) {
    console.warn("Saved selection is unreadable", e);
    return null;
  }
}

export const sortOf = (columnKey: string): ColumnSort => state.sorts[columnKey] ?? DEFAULT_SORT;

// The column the selection stands in, or the first one when nothing is selected.
export const activeColumnKey = (): string => state.selectedKey?.split("/").slice(0, -1).join("/") ?? "";

export function setColumnSort(columnKey: string, patch: Partial<ColumnSort>): void {
  state.sorts[columnKey] = { ...sortOf(columnKey), ...patch };
  savePref(SORTS_PREF, JSON.stringify(state.sorts));
  notify();
}

// Managing lasts while the managed folder or one of its direct children is selected; any other folder ends it.
function inManagedScope(key: string | null): boolean {
  if (key === null || state.managing === null) return true;
  const managed = keyOf(state.managing);
  const at = key.lastIndexOf("/");
  return key === managed || (at < 0 ? "" : key.slice(0, at)) === managed;
}

export function leaveVirtual(): void {
  state.virtual = null;
  state.virtualKeys = [];
  state.via = null;
}

// A virtual list stays open while one of its entries, or in FAVORITES something below the one entered, is selected.
function keepVirtual(key: string | null): boolean {
  if (key === null) return false;
  if (state.virtual === "favorites" && state.via !== null && key.startsWith(`${state.via}/`)) return true;
  if (!state.virtualKeys.includes(key)) return false;
  state.via = key;
  return true;
}

// Opens a virtual root entry with nothing selected, in one step so history sees a single move.
export function selectVirtual(id: VirtualId): void {
  if (state.virtual !== id) state.virtualKeys = [];
  state.virtual = id;
  state.via = null;
  state.selectedKey = null;
  state.selectedKeys = [];
  notify();
}

export function select(parts: string[] | null): void {
  state.selectedKey = parts ? keyOf(parts) : null;
  state.selectedKeys = state.selectedKey === null ? [] : [state.selectedKey];
  if (state.virtual !== null && !keepVirtual(state.selectedKey)) leaveVirtual();
  if (!inManagedScope(state.selectedKey)) {
    state.managing = null;
    state.hiddenChoices = [];
  }
  notify();
}

// Ctrl toggles one item; Shift selects the run from the anchor to it among the listed siblings.
export function clickSelect(parts: string[], siblings: string[], mods: { ctrl: boolean; shift: boolean }): void {
  const key = keyOf(parts);
  const anchor = state.selectedKey;
  if (mods.shift && anchor !== null && siblings.includes(anchor) && siblings.includes(key)) {
    const a = siblings.indexOf(anchor);
    const b = siblings.indexOf(key);
    state.selectedKeys = siblings.slice(Math.min(a, b), Math.max(a, b) + 1);
  } else if (mods.ctrl) {
    if (state.selectedKeys.includes(key)) {
      state.selectedKeys = state.selectedKeys.filter((k) => k !== key);
    } else {
      state.selectedKeys = [...state.selectedKeys, key];
      state.selectedKey = key;
    }
  } else {
    select(parts);
    return;
  }
  notify();
}

// A rubber band only marks items; the path (selectedKey) stays, so the open columns do not close.
export function selectKeys(keys: string[]): void {
  state.selectedKeys = keys;
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

// A rename outside the app keeps the id, so a stale path follows the one sibling that still has it.
const ID_PREFIX = /^(?:(?:PJ|PR|HW|CR|TK)\.\d+|\d+\.)/;
const idOf = (name: string): string | null => ID_PREFIX.exec(name)?.[0] ?? null;

function followPath(parts: string[]): string[] | null {
  const resolved: string[] = [];
  let level: TreeNode[] = state.tree?.nodes ?? [];
  for (const part of parts) {
    const id = idOf(part);
    const sameId = id === null ? [] : level.filter((n) => idOf(n.name) === id);
    const found = level.find((n) => n.name === part) ?? (sameId.length === 1 ? sameId[0] : undefined);
    if (!found) return null;
    resolved.push(found.name);
    level = found.children ?? [];
  }
  return resolved;
}

const followKey = (key: string): string | null => {
  const resolved = followPath(key.split("/"));
  return resolved ? keyOf(resolved) : null;
};

// Open folders are kept by path, so one renamed outside the app stays open under its new name.
export function followExpanded(): void {
  state.expanded = new Set([...state.expanded].map((key) => followKey(key) ?? key));
}

export function pruneSelection(): void {
  if (state.editing && !findNode(state.editing.key.split("/"))) state.editing = null;
  state.selectedKeys = [...new Set(state.selectedKeys.map(followKey).filter((key): key is string => key !== null))];
  if (state.selectedKey !== null) state.selectedKey = followKey(state.selectedKey) ?? state.selectedKeys.at(-1) ?? null;
  if (state.via !== null) state.via = followKey(state.via);
}

// Swaps in each parent's new children, keeping the loaded subtrees of untouched siblings; null when a parent is not loaded.
export function patchLevels(levels: LevelPatch[]): TreeNode[][] | null {
  if (!state.tree) return null;
  const patched: TreeNode[][] = [];
  for (const level of levels) {
    const parent = level.parent.length > 0 ? findNode(level.parent) : null;
    if (level.parent.length > 0 && !parent) return null;
    const old = parent ? (parent.children ?? []) : state.tree.nodes;
    const children = level.children.map((n) => {
      if (n.children !== null || !n.hasChildren) return n;
      return { ...n, children: old.find((o) => o.name === n.name)?.children ?? null };
    });
    if (parent) {
      parent.hasChildren = level.hasChildren;
      parent.children = level.hasChildren ? children : null;
    } else {
      state.tree.nodes = children;
    }
    patched.push(children);
  }
  return patched;
}

// Loads children of every expanded node under `nodes` whose children are not fetched yet.
export async function hydrate(nodes: TreeNode[] = state.tree?.nodes ?? []): Promise<void> {
  const walk = async (level: TreeNode[]): Promise<void> => {
    for (const n of level) {
      if (!state.expanded.has(keyOf(n.relParts))) continue;
      if (n.children === null && n.hasChildren) n.children = await listChildren(n.relParts);
      await walk(n.children ?? []);
    }
  };
  await walk(nodes);
}
