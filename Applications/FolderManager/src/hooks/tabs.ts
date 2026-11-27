// Tabs share the one tree; each keeps only its place, expansion, history and view, swapped into the store on a change of tab.
import { VIRTUAL_ENTRIES } from "../components/molecules/virtual-entry";
import { rootName } from "../components/organisms/bars/path-input";
import { reportError } from "../components/organisms/bars/status-line";
import { currentPlace, type Place } from "../services/store/place";
import { isWorknode } from "../services/store/node-info";
import { systemName, systemRootOf } from "../services/ipc/types";
import { readPref, savePref } from "../services/store/prefs";
import { currentSplit, fillOther, parkedSplit, restoreSplit, savedSplit, type SavedSplit, type Split } from "./split";
import { parkBatch, unparkBatch, type ParkedBatch } from "./automations/batch";
import { parkReplay, unparkReplay, type ParkedReplay } from "./automations/batch-replay";
import type { ViewSwitcher } from "./view-switch";
import { findNode, isVirtualShown, keyOf, leaveVirtual, notify, state } from "../services/store/state";

export interface Trail {
  history: Place[];
  at: number;
}

export interface TabNavigator {
  trail(): Trail;
  setTrail(trail: Trail): void;
  enter(trail: Trail, place: Place): Promise<void>;
}

interface Tab {
  place: Place;
  expanded: Set<string>;
  trail: Trail;
  view: string;
  split: Split | null;
  batch?: ParkedBatch | null;
  replay?: ParkedReplay | null;
  // The folder the view shows, which differs from the selection; undefined when the view has no single folder.
  shown?: string | null;
}

export interface TabView {
  label: string;
  title: string;
  active: boolean;
  split: "left" | "right" | null;
}

const PREF = "fm.tabs";
let navigator: TabNavigator | null = null;
let views: Pick<ViewSwitcher, "current" | "switchTo"> | null = null;
const currentView = () => views?.current().id ?? "";
const shownFolder = (): string | null | undefined => {
  const open = views?.current().openFolder?.();
  return open === undefined ? undefined : open && keyOf(open.relParts);
};
const newTab = (place: Place, expanded: Set<string>, view = currentView()): Tab => ({ place, expanded, trail: { history: [], at: -1 }, view, split: null });

let tabs: Tab[] = [newTab(currentPlace(), state.expanded)];
let active = 0;
// Most recently used first, so closing a tab returns to the one used before it.
let recent: Tab[] = [];
let root: string | null = null;
let moving = false;

export function bindTabs(nav: TabNavigator, switcher: Pick<ViewSwitcher, "current" | "switchTo">): void {
  navigator = nav;
  views = switcher;
}

interface SavedTabs {
  places: Place[];
  views: string[];
  splits: (SavedSplit | null)[];
  shown: (string | null | undefined)[];
  active: number;
}

const NO_TABS: SavedTabs = { places: [], views: [], splits: [], shown: [], active: 0 };

function readSaved(): SavedTabs {
  const raw = readPref(`${PREF}:${root}`);
  if (raw === null) return NO_TABS;
  try {
    const saved = JSON.parse(raw) as Partial<SavedTabs>;
    const list = <T>(v: T[] | undefined): T[] => (Array.isArray(v) ? v : []);
    return { places: list(saved.places), views: list(saved.views), splits: list(saved.splits), shown: list(saved.shown), active: saved.active ?? 0 };
  } catch (e) {
    console.warn("Saved tabs are unreadable", e);
    return NO_TABS;
  }
}

// The current tab stays as it is, because the launch restores its place on its own.
function loadTabs(): void {
  const saved = state.settings.onLaunch === "continue" ? readSaved() : NO_TABS;
  const usable = saved.places.map((place, i) => ({ place, i })).filter(({ place }) => place.virtual === null || isVirtualShown(place.virtual));
  const current = tabs[active] ?? newTab(currentPlace(), state.expanded);
  const at = usable.findIndex(({ i }) => i === saved.active);
  tabs = usable.length > 0 ? usable.map(({ place }, i) => (i === Math.max(at, 0) ? current : newTab(place, new Set(), saved.views[usable[i]?.i ?? -1]))) : [current];
  active = usable.length > 0 ? Math.max(at, 0) : 0;
  tabs.forEach((tab, i) => {
    const split = saved.splits[usable[i]?.i ?? -1];
    if (split) tab.split = parkedSplit(split);
    if (tab !== current) tab.shown = saved.shown[usable[i]?.i ?? -1] ?? undefined;
  });
  restoreSplit(tabs[active]?.split ?? null);
}

const saveTabs = () => savePref(`${PREF}:${root}`, JSON.stringify({ places: tabs.map((t) => t.place), views: tabs.map((t) => t.view), splits: tabs.map((t) => savedSplit(t.split)), shown: tabs.map((t) => t.shown ?? null), active }));

// Follows the store after every change, so the labels and what the next launch restores stay current.
export function syncTabs(): void {
  const open = state.tree?.root ?? null;
  if (open === null || moving) return;
  if (open !== root) {
    root = open;
    loadTabs();
  }
  const tab = tabs[active];
  if (tab) {
    tab.place = currentPlace();
    tab.view = currentView();
    tab.split = currentSplit();
    tab.shown = shownFolder();
    state.shownNow = tab.shown;
  }
  saveTabs();
}

function capture(): void {
  const tab = tabs[active];
  if (!tab || !navigator) return;
  tab.place = currentPlace();
  tab.expanded = state.expanded;
  tab.trail = navigator.trail();
  tab.view = currentView();
  tab.split = currentSplit();
  tab.batch = parkBatch();
  tab.replay = parkReplay();
}

async function activate(index: number): Promise<void> {
  const tab = tabs[index];
  if (!tab || !navigator) return;
  moving = true;
  recent = [...new Set([tab, ...tabs.slice(active, active + 1), ...recent])];
  active = index;
  state.expanded = tab.expanded;
  state.editing = null;
  state.pasteKey = null;
  state.managing = null;
  state.hiddenChoices = [];
  restoreSplit(tab.split);
  unparkBatch(tab.batch ?? null);
  tab.batch = null;
  views?.switchTo(tab.view);
  unparkReplay(tab.replay ?? null);
  tab.replay = null;
  // A tab from an older save has no shown folder, so it shows the one around its selection.
  state.showing = tab.shown !== undefined ? tab.shown : tab.place.virtual === null && tab.place.key !== null ? (tab.place.key.split("/").slice(0, -1).join("/") || null) : undefined;
  state.entering = null;
  try {
    await navigator.enter(tab.trail, tab.place);
  } catch (e) {
    reportError("error", e);
  } finally {
    moving = false;
  }
  syncTabs();
  notify();
}

export function selectTab(index: number): void {
  if (moving || index === active) return;
  capture();
  void activate(index);
}

export function nextTab(): void {
  selectTab((active + 1) % tabs.length);
}

// A tab opens on the folder it was asked for, else on the storage root.
export function openTab(parts?: string[]): Promise<void> {
  if (moving) return Promise.resolve();
  capture();
  const place: Place = { virtual: null, via: null, key: parts ? keyOf(parts) : null };
  const { tabView, startView } = state.settings;
  const view = tabView === "default" && startView !== "continue" ? startView : currentView();
  tabs.push(newTab(place, new Set(parts ? state.expanded : []), view));
  if (parts) {
    state.entering = place.key;
    leaveVirtual();
  }
  return activate(tabs.length - 1);
}

export function duplicateTab(index = active): void {
  if (moving) return;
  capture();
  const tab = tabs[index];
  if (!tab) return;
  const copy = newTab(tab.place, new Set(tab.expanded), tab.view);
  copy.trail = { history: [...tab.trail.history], at: tab.trail.at };
  tabs.push(copy);
  void activate(tabs.length - 1);
}

export function moveTab(from: number, to: number): void {
  const current = tabs[active];
  const tab = tabs[from];
  if (moving || !tab || !current) return;
  tabs.splice(from, 1);
  tabs.splice(to, 0, tab);
  active = tabs.indexOf(current);
  saveTabs();
  notify();
}

export function closeTab(index = active): void {
  if (moving || tabs.length === 1) return;
  if (index !== active) {
    tabs.splice(index, 1);
    if (index < active) active--;
    saveTabs();
    return notify();
  }
  tabs.splice(index, 1);
  const previous = recent.find((t) => tabs.includes(t));
  active = -1;
  void activate(previous ? tabs.indexOf(previous) : Math.min(index, tabs.length - 1));
}

// The picked tab leaves the bar: its place becomes the split's empty half.
export function joinTab(index: number): void {
  const tab = tabs[index];
  if (moving || !tab || index === active || tab.split || !currentSplit()?.empty) return;
  tabs.splice(index, 1);
  if (index < active) active--;
  recent = recent.filter((t) => t !== tab);
  fillOther(tab.place, tab.expanded, tab.trail, tab.view);
}

// Tabs are narrow, so the id prefix ("01.", "PR.051_") gives way to the title; the tooltip keeps the full name.
const withoutId = (name: string): string => name.replace(/^(?:[A-Za-z]{2,4}\.)?\d+[._]/, "") || name;

// A tab names where it stands, not what is selected: the project around the selection, else the folder holding it.
export function labelOf(place: Place, shown?: string | null): string {
  const key = shown === undefined ? place.key : shown;
  if (key !== null) {
    const parts = key.split("/");
    const project = parts.findIndex((_, i) => {
      const n = findNode(parts.slice(0, i + 1));
      return n !== null && n !== undefined && isWorknode(n);
    });
    if (project >= 0) return parts[project] ?? "";
    const folder = findNode(parts)?.file ? parts.slice(0, -1) : parts;
    // A native folder's own root is keyed "::Name"; it shows the entry label the breadcrumb uses.
    if (folder.length === 1 && systemRootOf(folder) !== null) return systemName(folder).toUpperCase();
    if (folder.length > 0) return folder[folder.length - 1] ?? "";
    return rootName(state.tree?.root);
  }
  return VIRTUAL_ENTRIES.find((e) => e.id === place.virtual)?.label ?? rootName(state.tree?.root);
}

export const tabViews = (): TabView[] =>
  tabs.map((t, i) => ({ label: withoutId(labelOf(t.place, t.shown)), title: (t.shown === undefined ? t.place.key : t.shown) ?? labelOf(t.place, t.shown), active: i === active, split: t.split ? (t.split.right ? "right" : "left") : null }));

export const tabStates = () =>
  tabs.map((t, i) => ({ label: labelOf(t.place), relParts: t.place.key?.split("/") ?? [], virtual: t.place.virtual, view: t.view, active: i === active }));
