// Per-device layout of the side panel, shared by the panel and the Appearance settings.
import { readPref, savePref } from "./prefs";
import { notify } from "./state";

export const RAIL_IDS = ["settings", "favorite", "edit", "moveUp", "moveDown", "delete"] as const;
export type RailId = (typeof RAIL_IDS)[number];

export interface RailItem {
  readonly id: RailId;
  readonly shown: boolean;
}

const QUICK_PREF = "fm.quickActions";
const RAIL_PREF = "fm.rail";

const defaultRail = (): RailItem[] => RAIL_IDS.map((id) => ({ id, shown: true }));

function readRail(): RailItem[] {
  const raw = readPref(RAIL_PREF);
  if (raw === null) return defaultRail();
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    console.warn("Saved side panel layout is unreadable", e);
    return defaultRail();
  }
  if (!Array.isArray(parsed)) {
    console.warn("Saved side panel layout is not a list", parsed);
    return defaultRail();
  }
  const items: RailItem[] = [];
  for (const entry of parsed) {
    const { id, shown } = (entry ?? {}) as Partial<RailItem>;
    if (RAIL_IDS.includes(id as RailId) && !items.some((i) => i.id === id)) items.push({ id: id as RailId, shown: shown !== false });
  }
  RAIL_IDS.forEach((id, index) => {
    if (!items.some((i) => i.id === id)) items.splice(Math.min(index, items.length), 0, { id, shown: true });
  });
  return items;
}

// Kept in memory too, so the layout still changes for this session when storage is blocked.
let quickActions = readPref(QUICK_PREF) !== "hidden";
let rail: readonly RailItem[] = readRail();

export const quickActionsShown = (): boolean => quickActions;

export function setQuickActionsShown(on: boolean): void {
  quickActions = on;
  savePref(QUICK_PREF, on ? "shown" : "hidden");
  notify();
}

export const railLayout = (): readonly RailItem[] => rail;

export function setRailLayout(items: readonly RailItem[]): void {
  rail = items;
  savePref(RAIL_PREF, JSON.stringify(rail));
  notify();
}
