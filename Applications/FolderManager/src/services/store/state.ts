// Single in-memory store for the tree snapshot, selection and expansion.
import { listChildren } from "../ipc/api";
import { favoritesFirst, foldersFirst, SORT_BYS, type ColumnSort, type SortBy } from "./sort";
import { readPref, savePref } from "./prefs";
import { DOWNLOADS_ROOT, inDownloads } from "../ipc/types";
import type { HiddenChoice, LevelPatch, TreeNode, TreeSnapshot } from "../ipc/types";

interface State {
  tree: TreeSnapshot | null;
  // Set by an explicit open, so a view that lists a folder enters this one instead of showing it among its siblings.
  entering: string | null;
  selectedKey: string | null;
  // Every selected key, the primary one included; a plain click leaves just that one.
  selectedKeys: string[];
  // Folder the last click in the views landed on, "" for the storage root; Ctrl+V pastes there.
  pasteKey: string | null;
  expanded: Set<string>;
  busy: boolean;
  // Lists every review link so the hidden ones can be selected again, instead of only the shown ones.
  hidingLinks: boolean;
  // Folders deeper than this are shown but not indexed in the database.
  depthLimit: number;
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
  // The system Downloads folder read so far; its paths start with DOWNLOADS_ROOT and live outside `tree`.
  downloads: TreeNode[];
  // The listed entry the selection went through; inside FAVORITES the path stays symbolic below it.
  via: string | null;
}

export type VirtualId = "favorites" | "downloads" | "review" | "board";

const START_VIEWS: StartView[] = ["tree", "columns", "grid", "list", "details", "cards"];

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
export type TabView = "last" | "default";
export type StartView = "continue" | "tree" | "columns" | "grid" | "list" | "details" | "cards";

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
  showFavorites: boolean;
  showDownloads: boolean;
  showReview: boolean;
  theme: Theme;
  accent: AccentId;
  iconColor: AccentId;
  textSize: TextSize;
  backupFrequency: BackupFrequency;
  // Empty means the default folder inside the Storage.
  backupFolder: string;
  onLaunch: OnLaunch;
  startView: StartView;
  tabView: TabView;
  // Only the shortcuts the user changed; the rest keep their defaults.
  shortcuts: Record<string, KeyCombo>;
}

export type { ColumnSort, SortBy };

const SORTS_PREF = "fm.sorts";
const DEFAULT_SORT: ColumnSort = { by: "track", dir: "asc" };

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
  entering: null,
  selectedKey: null,
  selectedKeys: [],
  pasteKey: null,
  expanded: new Set(),
  busy: false,
  hidingLinks: false,
  depthLimit: 3,
  managing: null,
  hiddenChoices: [],
  editing: null,
  sorts: readSorts(),
  settings: { pinFavorites: true, hideDone: false, hideDirectoryIdentifiers: false, confirmDelete: true, autoUpdate: true, debugLogging: false, backupAuto: false, watchExternal: true, autoOpenColumns: false, showFavorites: true, showDownloads: true, showReview: true, theme: "dark", accent: "green", iconColor: "purple", textSize: "small", backupFrequency: "day", backupFolder: "", onLaunch: "continue", startView: "continue", tabView: "last", shortcuts: {} },
  virtual: null,
  virtualKeys: [],
  downloads: [],
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

// The nodes at the top of a path: the storage's, or Downloads' when the path starts with its marker.
const topLevel = (parts: string[]): TreeNode[] => (inDownloads(parts) ? state.downloads : (state.tree?.nodes ?? []));

export function findNode(parts: string[]): TreeNode | null {
  let level: TreeNode[] = topLevel(parts.slice(0, 1));
  let found: TreeNode | null = null;
  // The Downloads marker names the list itself, not a node in it.
  for (const part of inDownloads(parts) ? parts.slice(1) : parts) {
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
  state.settings.watchExternal = saved.watchExternal !== "false";
  state.settings.autoOpenColumns = saved.autoOpenColumns === "true";
  state.settings.showFavorites = saved.showFavorites !== "false";
  state.settings.showDownloads = saved.showDownloads !== "false";
  state.settings.showReview = saved.showReview !== "false";
  state.settings.theme = saved.theme === "light" || saved.theme === "system" ? saved.theme : "dark";
  state.settings.accent = ["blue", "purple", "coral", "gold"].includes(saved.accent ?? "") ? (saved.accent as AccentId) : "green";
  state.settings.iconColor = ["green", "blue", "coral", "gold"].includes(saved.iconColor ?? "") ? (saved.iconColor as AccentId) : "purple";
  state.settings.textSize = saved.textSize === "medium" || saved.textSize === "large" ? saved.textSize : "small";
  state.settings.backupFrequency = saved.backupFrequency === "week" || saved.backupFrequency === "month" ? saved.backupFrequency : "day";
  state.settings.backupFolder = saved.backupFolder ?? "";
  state.settings.onLaunch = saved.onLaunch === "root" ? "root" : "continue";
  state.settings.tabView = saved.tabView === "default" ? "default" : "last";
  state.settings.startView = START_VIEWS.includes(saved.startView as StartView) ? (saved.startView as StartView) : "continue";
  state.settings.shortcuts = readShortcuts(saved.shortcuts);
}

const LAST_PREF = "fm.last";

export interface SavedPlace {
  key: string | null;
  virtual: VirtualId | null;
  via: string | null;
}

function readSaved(): (SavedPlace & { root?: string }) | null {
  const raw = readPref(LAST_PREF);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as SavedPlace & { root?: string };
  } catch (e) {
    console.warn("Saved selection is unreadable", e);
    return null;
  }
}

// The board selects nothing, so the place it was opened from is kept beside it.
export function rememberSelection(): void {
  if (!state.tree) return;
  const before = state.virtual === "board" ? readSaved() : null;
  const key = state.virtual === "board" ? (before?.root === state.tree.root ? (before.key ?? null) : null) : state.selectedKey;
  savePref(LAST_PREF, JSON.stringify({ root: state.tree.root, key, virtual: state.virtual, via: state.virtual === "board" ? null : state.via }));
}

// Where the user stood when the app closed, if it belongs to the Storage now open.
export function lastSelection(): SavedPlace | null {
  const saved = readSaved();
  if (!saved || !state.tree || saved.root !== state.tree.root) return null;
  return { key: saved.key ?? null, virtual: saved.virtual ?? null, via: saved.via ?? null };
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

// Favorites and Downloads can be switched off in the settings; the board has no switch.
export const isVirtualShown = (id: VirtualId): boolean => (id === "favorites" ? state.settings.showFavorites : id === "downloads" ? state.settings.showDownloads : id === "review" ? state.settings.showReview : true);

export function leaveVirtual(): void {
  state.virtual = null;
  state.virtualKeys = [];
  state.via = null;
}

// A virtual list stays open while one of its entries, or in FAVORITES something below the one entered, is selected.
function keepVirtual(key: string | null): boolean {
  if (key === null) return false;
  // Everything under Downloads belongs to its list; the entry it went through is the top-level one.
  if (state.virtual === "downloads" && key.startsWith(`${DOWNLOADS_ROOT}/`)) {
    state.via = key.split("/").slice(0, 2).join("/");
    return true;
  }
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
  rememberPlace();
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
// Walks down loading each level on the way, past the depth limit too; a folder renamed outside keeps its place through its id, and a gone one leaves its nearest parent.
export async function reveal(parts: string[]): Promise<void> {
  await hydrate();
  const resolved: string[] = [];
  let level: TreeNode[] = topLevel(parts.slice(0, 1));
  for (const part of inDownloads(parts) ? parts.slice(1) : parts) {
    const id = inDownloads(parts) ? null : idOf(part);
    const sameId = id === null ? [] : level.filter((n) => idOf(n.name) === id);
    const found = level.find((n) => n.name === part) ?? (sameId.length === 1 ? sameId[0] : undefined);
    if (!found) break;
    if (found.children === null && found.hasChildren && resolved.length + 1 < parts.length) found.children = await listChildren(found.relParts);
    resolved.push(found.name);
    level = found.children ?? [];
  }
  const path = inDownloads(parts) && resolved.length > 0 ? [parts[0] ?? "", ...resolved] : resolved;
  for (let i = 1; i < path.length; i++) state.expanded.add(keyOf(path.slice(0, i)));
  select(path.length > 0 && findNode(path) ? path : null);
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
  let level: TreeNode[] = topLevel(parts.slice(0, 1));
  for (const part of parts) {
    // File names in Downloads are not identifiers, so only an exact name is followed there.
    const id = inDownloads(parts) ? null : idOf(part);
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

// Where the selected item sat among its siblings, so a rename or delete can land on whatever now holds that place.
let lastPlace: { key: string; index: number } | null = null;

const siblingsOf = (parts: string[]): TreeNode[] => {
  const parent = parts.length > 1 ? findNode(parts.slice(0, -1)) : null;
  return listed(parent ? (parent.children ?? []) : topLevel(parts.slice(0, 1)));
};

function rememberPlace(): void {
  const key = state.selectedKey;
  const index = key === null ? -1 : siblingsOf(key.split("/")).findIndex((n) => keyOf(n.relParts) === key);
  lastPlace = key !== null && index >= 0 ? { key, index } : null;
}

// The item itself when it can be followed, else what took its place among its siblings, else the nearest folder still there.
function nearestKey(key: string): string | null {
  const followed = followKey(key);
  if (followed) return followed;
  const parts = key.split("/");
  if (lastPlace?.key === key) {
    const siblings = siblingsOf(parts);
    const there = siblings[Math.min(lastPlace.index, siblings.length - 1)];
    if (there) return keyOf(there.relParts);
  }
  for (let i = parts.length - 1; i > 0; i--) if (findNode(parts.slice(0, i))) return keyOf(parts.slice(0, i));
  return null;
}

export function pruneSelection(): void {
  if (state.editing && !findNode(state.editing.key.split("/"))) state.editing = null;
  state.selectedKeys = [...new Set(state.selectedKeys.map(followKey).filter((key): key is string => key !== null))];
  if (state.selectedKey !== null) state.selectedKey = followKey(state.selectedKey) ?? (state.selectedKeys.at(-1) || nearestKey(state.selectedKey));
  if (state.selectedKeys.length === 0 && state.selectedKey !== null) state.selectedKeys = [state.selectedKey];
  if (state.via !== null) state.via = followKey(state.via);
  rememberPlace();
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
export async function hydrate(nodes: TreeNode[] = [...(state.tree?.nodes ?? []), ...state.downloads]): Promise<void> {
  const walk = async (level: TreeNode[]): Promise<void> => {
    for (const n of level) {
      // Preloaded levels are walked even when collapsed, so a deeper folder opened from the columns view still loads.
      if (n.children === null && n.hasChildren && state.expanded.has(keyOf(n.relParts))) n.children = await listChildren(n.relParts);
      await walk(n.children ?? []);
    }
  };
  await walk(nodes);
}
