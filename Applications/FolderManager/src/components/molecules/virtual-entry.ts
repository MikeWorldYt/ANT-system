// Rows for the virtual root entries; without a data-key no move, drop or paste ever lands on them.
import { el } from "../dom";
import { guiIcon, type GuiIcon } from "../atoms/gui-icons";
import { nodeToggle } from "../atoms/node-toggle";
import { openVirtual } from "../../hooks/virtual-nav";
import { isVirtualShown, select, state, type VirtualId } from "../../services/store/state";

export interface VirtualEntry {
  id: VirtualId;
  label: string;
  icon: GuiIcon;
  hint: string;
  empty: string;
}

export const VIRTUAL_ENTRIES: VirtualEntry[] = [
  { id: "favorites", label: "FAVORITES", icon: "starred", hint: "Every favorite folder in the Storage", empty: "No favorites yet" },
  { id: "downloads", label: "DOWNLOADS", icon: "download", hint: "The Downloads folder of this computer", empty: "Downloads is empty" },
  { id: "board", label: "BOARD", icon: "layout", hint: "Task board of every workboard in the Storage", empty: "No workboards yet" },
];

// The board opens from the side panel, so only the entries the settings leave on head the root listing.
export const rootEntries = (): VirtualEntry[] => VIRTUAL_ENTRIES.filter((e) => e.id !== "board" && isVirtualShown(e.id));

// The entry the arrow keys start from when nothing is selected: the open one, else the first.
export const headEntry = (): VirtualEntry | undefined => {
  const entries = rootEntries();
  return entries.find((e) => e.id === state.virtual) ?? entries[0];
};

// The root entry just after (1) or before (-1) another; the first one when there is no other.
export const entryBeside = (from: VirtualEntry | undefined, step: 1 | -1): VirtualEntry | undefined => {
  const entries = rootEntries();
  return entries[entries.findIndex((e) => e.id === from?.id) + step];
};

export const openEntry = (): VirtualEntry | null => VIRTUAL_ENTRIES.find((e) => e.id === state.virtual) ?? null;

interface EntryOptions {
  toggle?: boolean;
  tabbable?: boolean;
  marker?: "caret" | "chevron";
}

// `toggle` lets a second activation close the entry, as a tree row folds; without `tabbable` the view's own container holds the focus.
export function virtualEntry(entry: VirtualEntry, cls: string, role: string, { toggle = false, tabbable = false, marker }: EntryOptions = {}): HTMLElement {
  const open = state.virtual === entry.id;
  const icon = el("span", { class: "virtual-icon", "aria-hidden": "true" }, guiIcon(entry.icon));
  const lead = marker === "caret" ? [nodeToggle(`${open ? "Collapse" : "Expand"} ${entry.label}`, open, el("span", { class: "caret", "aria-hidden": "true" }, guiIcon(open ? "chevronDown" : "chevronRight")), icon)] : [icon];
  const item = el(
    "div",
    { class: `${cls} virtual-entry`, role, title: entry.hint, "aria-selected": String(open && state.selectedKey === null) },
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
  const activate = () => (toggle && open ? select(null) : void openVirtual(entry.id));
  item.addEventListener("click", activate);
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

// Only the head entry can hold the roving focus.
export const virtualEntries = (cls: string, role: string, options: EntryOptions = {}): HTMLElement[] =>
  rootEntries().map((e) => virtualEntry(e, cls, role, { ...options, tabbable: options.tabbable === true && e.id === headEntry()?.id }));
