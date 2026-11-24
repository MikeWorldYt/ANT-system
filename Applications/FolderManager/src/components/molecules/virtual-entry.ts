// Rows for the virtual root entries; without a data-key no move, drop or paste ever lands on them.
import { el } from "../dom";
import { guiIcon, type GuiIcon } from "../atoms/gui-icons";
import { fromToggle, nodeToggle } from "../atoms/node-toggle";
import { openVirtual } from "../../hooks/virtual-nav";
import { isVirtualShown, state, virtualRoot, type VirtualId } from "../../services/store/state";

export interface VirtualEntry {
  id: VirtualId;
  label: string;
  icon: GuiIcon;
  hint: string;
  empty: string;
}

export const VIRTUAL_ENTRIES: VirtualEntry[] = [
  { id: "favorites", label: "FAVORITES", icon: "starred", hint: "Favorites folder", empty: "No favorites yet" },
  { id: "downloads", label: "DOWNLOADS", icon: "download", hint: "Native Downloads folder", empty: "Downloads is empty" },
  { id: "review", label: "REVIEW", icon: "statusReview", hint: "Review folders Index", empty: "No review folders yet" },
  { id: "documents", label: "DOCUMENTS", icon: "documents", hint: "Native Documents folder", empty: "Documents is empty" },
  { id: "pictures", label: "PICTURES", icon: "pictures", hint: "Native Pictures folder", empty: "Pictures is empty" },
  { id: "music", label: "MUSIC", icon: "music", hint: "Native Music folder", empty: "Music is empty" },
  { id: "videos", label: "VIDEOS", icon: "videos", hint: "Native Videos folder", empty: "Videos is empty" },
  { id: "desktop", label: "DESKTOP", icon: "desktop", hint: "Native Desktop folder", empty: "Desktop is empty" },
  { id: "board", label: "BOARD", icon: "layout", hint: "Task board of every worknode in the Storage", empty: "No worknodes yet" },
];

// The board opens from the side panel, so only the entries the settings leave on head the root listing.
export const rootEntries = (): VirtualEntry[] => VIRTUAL_ENTRIES.filter((e) => e.id !== "board" && isVirtualShown(e.id));

// The entry the arrow keys start from when nothing is selected: the open one, else the first.
export const headEntry = (): VirtualEntry | undefined => {
  const entries = rootEntries();
  return entries.find((e) => e.id === state.virtual) ?? entries[0];
};

// The native folders besides Downloads close every list, after the folders, instead of heading it.
export const isTailEntry = (id: string | undefined): boolean => id !== undefined && id !== "downloads" && virtualRoot(id as VirtualId) !== null;

export const leadEntries = (): VirtualEntry[] => rootEntries().filter((e) => !isTailEntry(e.id));

export const tailEntries = (): VirtualEntry[] => rootEntries().filter((e) => isTailEntry(e.id));

// The root entry just after (1) or before (-1) another in its own group, head or tail; the first one when there is no other.
export const entryBeside = (from: VirtualEntry | undefined, step: 1 | -1): VirtualEntry | undefined => {
  const entries = isTailEntry(from?.id) ? tailEntries() : leadEntries();
  return entries[entries.findIndex((e) => e.id === from?.id) + step];
};

// The entry the side panel describes: the one stood on, else the open one.
export const shownEntry = (): VirtualEntry | null => VIRTUAL_ENTRIES.find((e) => e.id === (state.marked ?? state.virtual) && e.id !== "board") ?? null;

export const openEntry = (): VirtualEntry | null => VIRTUAL_ENTRIES.find((e) => e.id === state.virtual) ?? null;

interface EntryOptions {
  tabbable?: boolean;
  marker?: "caret" | "chevron";
  mark?: (id: VirtualId) => void;
  // The caret unfolds the entry where it stands, as a tree row folds, without opening it in the views.
  unfold?: { open: boolean; set: (on: boolean) => void };
}

// Without `tabbable` the view's own container holds the focus.
export function virtualEntry(entry: VirtualEntry, cls: string, role: string, { tabbable = false, marker, mark, unfold }: EntryOptions = {}): HTMLElement {
  const open = state.virtual === entry.id;
  const folded = unfold?.open ?? open;
  const icon = el("span", { class: "virtual-icon", "aria-hidden": "true" }, guiIcon(entry.icon));
  const lead = marker === "caret" ? [nodeToggle(`${folded ? "Collapse" : "Expand"} ${entry.label}`, folded, el("span", { class: "caret", "aria-hidden": "true" }, guiIcon(folded ? "chevronDown" : "chevronRight")), icon)] : [icon];
  const item = el(
    "div",
    { class: `${cls} virtual-entry`, "data-entry": entry.id, role, title: entry.hint, "aria-selected": String(open && state.selectedKey === null) },
    ...lead,
    el("span", { class: "name" }, el("span", { class: "name-text" }, entry.label)),
  );
  if (tabbable !== undefined) item.tabIndex = tabbable ? 0 : -1;
  if (marker === "chevron") {
    const chevron = nodeToggle(`Show contents of ${entry.label}`, null, guiIcon("chevronRight"));
    chevron.classList.add("chevron");
    item.append(chevron);
  }
  if (open) item.classList.add(state.selectedKey === null ? "selected" : "on-path");
  const activate = () => void openVirtual(entry.id);
  // Where folders open on a double click, one click only marks the entry, as it marks a folder.
  if (mark) {
    item.addEventListener("click", (ev) => (unfold && fromToggle(ev) ? unfold.set(!unfold.open) : mark(entry.id)));
    // Its two clicks already toggled twice, so a double click on the caret ends unfolded.
    item.addEventListener("dblclick", (ev) => (unfold && fromToggle(ev) ? unfold.set(true) : activate()));
  } else item.addEventListener("click", activate);
  // The view's own keys would act on the selected folder instead.
  item.addEventListener("keydown", (ev) => {
    if (ev.key !== "Enter" && ev.key !== " ") return;
    ev.preventDefault();
    ev.stopPropagation();
    activate();
  });
  // Not a folder on disk, so neither its own menu nor the background one applies.
  item.addEventListener("contextmenu", (ev) => {
    ev.preventDefault();
    ev.stopPropagation();
  });
  return item;
}

// Favorites heads the root; Downloads opens the tail group at `cut` and Review closes it, beside the folders they gather; the other native folders come last.
export function placeEntries(entries: HTMLElement[], items: HTMLElement[], cut: number): HTMLElement[] {
  const pick = (id: string) => entries.filter((e) => e.dataset.entry === id);
  return [...pick("favorites"), ...items.slice(0, cut), ...pick("downloads"), ...items.slice(cut), ...pick("review"), ...entries.filter((e) => isTailEntry(e.dataset.entry))];
}

// Only the head entry can hold the roving focus.
export const virtualEntries = (cls: string, role: string, options: EntryOptions = {}): HTMLElement[] =>
  rootEntries().map((e) => virtualEntry(e, cls, role, { ...options, tabbable: options.tabbable === true && e.id === headEntry()?.id }));
