// The Details view's column set: which columns show and how wide each is, kept per viewer as in Explorer.
import { readPref, savePref } from "./prefs";
import type { SortBy } from "./state";

export type ColumnId = "name" | "modified" | "type" | "size" | "created" | "status" | "owner";

export interface ColumnDef {
  id: ColumnId;
  label: string;
  sort: SortBy | null;
  width: number;
  shown: boolean;
}

export const COLUMNS: ColumnDef[] = [
  { id: "name", label: "Name", sort: "name", width: 360, shown: true },
  { id: "modified", label: "Date modified", sort: "modified", width: 150, shown: true },
  { id: "type", label: "Type", sort: null, width: 110, shown: true },
  { id: "size", label: "Size", sort: "size", width: 70, shown: true },
  { id: "created", label: "Date created", sort: "created", width: 150, shown: true },
  { id: "status", label: "Status", sort: "status", width: 100, shown: false },
  { id: "owner", label: "Owner", sort: null, width: 110, shown: false },
];

const PREF = "fm.detailColumns";
export const MIN_WIDTH = 40;

type Widths = Partial<Record<ColumnId, number>>;

// Widths are kept per folder, as Explorer remembers each folder's columns.
interface Saved {
  shown: ColumnId[];
  widths: Record<string, Widths>;
}

function load(): Saved {
  const fallback: Saved = { shown: COLUMNS.filter((c) => c.shown).map((c) => c.id), widths: {} };
  const raw = readPref(PREF);
  if (!raw) return fallback;
  try {
    const saved = JSON.parse(raw) as Partial<Saved>;
    const shown = (saved.shown ?? fallback.shown).filter((id) => COLUMNS.some((c) => c.id === id));
    // Widths saved before they were kept per folder are keyed by column, so they are dropped.
    const perFolder = Object.values(saved.widths ?? {}).every((w) => typeof w === "object" && w !== null);
    return { shown: shown.includes("name") ? shown : ["name", ...shown], widths: perFolder ? (saved.widths ?? {}) : {} };
  } catch (e) {
    console.warn("Details columns preference unreadable", e);
    return fallback;
  }
}

let saved = load();
const listeners = new Set<() => void>();

const persist = (): void => {
  savePref(PREF, JSON.stringify(saved));
  listeners.forEach((l) => l());
};

export const onColumnsChange = (l: () => void): void => void listeners.add(l);
export const shownColumns = (): ColumnDef[] => COLUMNS.filter((c) => saved.shown.includes(c.id));
export const isShown = (id: ColumnId): boolean => saved.shown.includes(id);
export const widthOf = (folder: string, id: ColumnId): number => saved.widths[folder]?.[id] ?? COLUMNS.find((c) => c.id === id)?.width ?? MIN_WIDTH;

export function toggleColumn(id: ColumnId): void {
  if (id === "name") return;
  saved.shown = isShown(id) ? saved.shown.filter((c) => c !== id) : COLUMNS.filter((c) => c.id === id || saved.shown.includes(c.id)).map((c) => c.id);
  persist();
}

export function setWidths(folder: string, widths: Widths): void {
  const own = (saved.widths[folder] ??= {});
  for (const [id, w] of Object.entries(widths)) own[id as ColumnId] = Math.max(MIN_WIDTH, Math.round(w));
  persist();
}
