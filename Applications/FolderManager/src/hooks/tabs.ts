// Tabs share the one tree; each keeps only its place, expansion and history, swapped into the store on a change of tab.
import { VIRTUAL_ENTRIES } from "../components/molecules/virtual-entry";
import { rootName } from "../components/organisms/bars/path-input";
import { reportError } from "../components/organisms/bars/status-line";
import { currentPlace, type Place } from "../services/store/place";
import { readPref, savePref } from "../services/store/prefs";
import { isVirtualShown, keyOf, notify, state } from "../services/store/state";

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
}

export interface TabView {
  label: string;
  title: string;
  active: boolean;
}

const PREF = "fm.tabs";
const newTab = (place: Place, expanded: Set<string>): Tab => ({ place, expanded, trail: { history: [], at: -1 } });

let navigator: TabNavigator | null = null;
let tabs: Tab[] = [newTab(currentPlace(), state.expanded)];
let active = 0;
let root: string | null = null;
let moving = false;

export function bindTabs(nav: TabNavigator): void {
  navigator = nav;
}

function readSaved(): { places: Place[]; active: number } {
  const raw = readPref(`${PREF}:${root}`);
  if (raw === null) return { places: [], active: 0 };
  try {
    const saved = JSON.parse(raw) as { places?: Place[]; active?: number };
    return { places: Array.isArray(saved.places) ? saved.places : [], active: saved.active ?? 0 };
  } catch (e) {
    console.warn("Saved tabs are unreadable", e);
    return { places: [], active: 0 };
  }
}

// The current tab stays as it is, because the launch restores its place on its own.
function loadTabs(): void {
  const saved = state.settings.onLaunch === "continue" ? readSaved() : { places: [], active: 0 };
  const usable = saved.places.map((place, i) => ({ place, i })).filter(({ place }) => place.virtual === null || isVirtualShown(place.virtual));
  const current = tabs[active] ?? newTab(currentPlace(), state.expanded);
  const at = usable.findIndex(({ i }) => i === saved.active);
  tabs = usable.length > 0 ? usable.map(({ place }, i) => (i === Math.max(at, 0) ? current : newTab(place, new Set()))) : [current];
  active = usable.length > 0 ? Math.max(at, 0) : 0;
}

const saveTabs = () => savePref(`${PREF}:${root}`, JSON.stringify({ places: tabs.map((t) => t.place), active }));

// Follows the store after every change, so the labels and what the next launch restores stay current.
export function syncTabs(): void {
  const open = state.tree?.root ?? null;
  if (open === null || moving) return;
  if (open !== root) {
    root = open;
    loadTabs();
  }
  const tab = tabs[active];
  if (tab) tab.place = currentPlace();
  saveTabs();
}

function capture(): void {
  const tab = tabs[active];
  if (!tab || !navigator) return;
  tab.place = currentPlace();
  tab.expanded = state.expanded;
  tab.trail = navigator.trail();
}

async function activate(index: number): Promise<void> {
  const tab = tabs[index];
  if (!tab || !navigator) return;
  moving = true;
  active = index;
  state.expanded = tab.expanded;
  state.editing = null;
  state.pasteKey = null;
  state.managing = null;
  state.hiddenChoices = [];
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

// A tab opens on the folder it was asked for, else on a copy of the current place.
export function openTab(parts?: string[]): void {
  if (moving) return;
  capture();
  const place: Place = parts ? { virtual: null, via: null, key: keyOf(parts) } : currentPlace();
  tabs.splice(active + 1, 0, newTab(place, new Set(state.expanded)));
  void activate(active + 1);
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
  void activate(Math.min(index, tabs.length - 1));
}

function labelOf(place: Place): string {
  if (place.key !== null) return place.key.split("/").pop() ?? "";
  return VIRTUAL_ENTRIES.find((e) => e.id === place.virtual)?.label ?? rootName(state.tree?.root);
}

export const tabViews = (): TabView[] =>
  tabs.map((t, i) => ({ label: labelOf(t.place), title: t.place.key ?? labelOf(t.place), active: i === active }));
