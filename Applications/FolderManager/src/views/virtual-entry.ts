// Rows for the virtual root entries; without a data-key no move, drop or paste ever lands on them.
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";
import { openVirtual } from "../actions/virtual-nav";
import { select, state, type VirtualId } from "../store/state";

export interface VirtualEntry {
  id: VirtualId;
  label: string;
  icon: GuiIcon;
  hint: string;
  empty: string;
}

export const VIRTUAL_ENTRIES: VirtualEntry[] = [
  { id: "favorites", label: "FAVORITES", icon: "starred", hint: "Every favorite folder in the Storage", empty: "No favorites yet" },
  { id: "board", label: "BOARD", icon: "layout", hint: "Task board of every workboard in the Storage", empty: "No workboards yet" },
];

// The board opens from the side panel, so only FAVORITES heads the root listing.
export const ROOT_ENTRIES = VIRTUAL_ENTRIES.filter((e) => e.id !== "board");

export const openEntry = (): VirtualEntry | null => VIRTUAL_ENTRIES.find((e) => e.id === state.virtual) ?? null;

// `toggle` lets a second activation close the entry, as a tree row folds.
export function virtualEntry(entry: VirtualEntry, cls: string, role: string, toggle = false): HTMLElement {
  const open = state.virtual === entry.id;
  const item = el(
    "div",
    { class: `${cls} virtual-entry`, role, tabindex: "0", title: entry.hint, "aria-selected": String(open && state.selectedKey === null) },
    el("span", { class: "virtual-icon", "aria-hidden": "true" }, guiIcon(entry.icon)),
    el("span", { class: "name" }, el("span", { class: "name-text" }, entry.label)),
  );
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

export const virtualEntries = (cls: string, role: string, toggle = false): HTMLElement[] => ROOT_ENTRIES.map((e) => virtualEntry(e, cls, role, toggle));
