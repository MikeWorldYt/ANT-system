// Tabs share the one tree; each keeps only its place, expansion, history and view, swapped into the store on a change of tab.
import { VIRTUAL_ENTRIES } from "../components/molecules/virtual-entry";
import { rootName } from "../components/organisms/bars/path-input";
import { reportError } from "../components/organisms/bars/status-line";
import { currentPlace, type Place } from "../services/store/place";
import { isWorkboard } from "../services/store/node-info";
import { readPref, savePref } from "../services/store/prefs";
import type { ViewSwitcher } from "./view-switch";
import { findNode, isVirtualShown, keyOf, leaveVirtual, notify, state } from "../services/store/state";

export interface Trail {
  history: Place[];
  at: number;
}

export interface TabNavigator {
  trail(): Trail;
  enter(trail: Trail, place: Place): Promise<void>;
}

interface Tab {
  place: Place;
  expanded: Set<string>;
  trail: Trail;
  view: string;
}

export interface TabView {
  label: string;
  title: string;
  active: boolean;
}

const PREF = "fm.tabs";
let navigator: TabNavigator | null = null;
let views: Pick<ViewSwitcher, "current" | "switchTo"> | null = null;
const currentView = () => views?.current().id ?? "";
const newTab = (place: Place, expanded: Set<string>, view = currentView()): Tab => ({ place, expanded, trail: { history: [], at: -1 }, view });

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

function readSaved(): { places: Place[]; views: string[]; active: number } {
  const raw = readPref(`${PREF}:${root}`);
  if (raw === null) return { places: [], views: [], active: 0 };
  try {
    const saved = JSON.parse(raw) as { places?: Place[]; views?: string[]; active?: number };
    return { places: Array.isArray(saved.places) ? saved.places : [], views: Array.isArray(saved.views) ? saved.views : [], active: saved.active ?? 0 };
  } catch (e) {
    console.warn("Saved tabs are unreadable", e);
    return { places: [], views: [], active: 0 };
  }
}

// The current tab stays as it is, because the launch restores its place on its own.
function loadTabs(): void {
  const saved = state.settings.onLaunch === "continue" ? readSaved() : { places: [], views: [], active: 0 };
  const usable = saved.places.map((place, i) => ({ place, i })).filter(({ place }) => place.virtual === null || isVirtualShown(place.virtual));
  const current = tabs[active] ?? newTab(currentPlace(), state.expanded);
  const at = usable.findIndex(({ i }) => i === saved.active);
  tabs = usable.length > 0 ? usable.map(({ place }, i) => (i === Math.max(at, 0) ? current : newTab(place, new Set(), saved.views[usable[i]?.i ?? -1]))) : [current];
  active = usable.length > 0 ? Math.max(at, 0) : 0;
}

const saveTabs = () => savePref(`${PREF}:${root}`, JSON.stringify({ places: tabs.map((t) => t.place), views: tabs.map((t) => t.view), active }));

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
  views?.switchTo(tab.view);
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
export function openTab(parts?: string[]): void {
  if (moving) return;
  capture();
  const place: Place = { virtual: null, via: null, key: parts ? keyOf(parts) : null };
  tabs.push(newTab(place, new Set(parts ? state.expanded : [])));
  if (parts) {
    state.entering = place.key;
    leaveVirtual();
  }
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

// A tab names where it stands, not what is selected: the project around the selection, else the folder holding it.
function labelOf(place: Place): string {
  if (place.key !== null) {
    const parts = place.key.split("/");
    const project = parts.findIndex((_, i) => {
      const n = findNode(parts.slice(0, i + 1));
      return n !== null && n !== undefined && isWorkboard(n);
    });
    if (project >= 0) return parts[project] ?? "";
    const folder = findNode(parts)?.file ? parts.slice(0, -1) : parts;
    if (folder.length > 0) return folder[folder.length - 1] ?? "";
    return rootName(state.tree?.root);
  }
  return VIRTUAL_ENTRIES.find((e) => e.id === place.virtual)?.label ?? rootName(state.tree?.root);
}

export const tabViews = (): TabView[] =>
  tabs.map((t, i) => ({ label: labelOf(t.place), title: t.place.key ?? labelOf(t.place), active: i === active }));
