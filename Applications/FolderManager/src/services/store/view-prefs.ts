// Per-device choices on what the views show, as in Explorer's Show/hide menu: hidden items and file name extensions.
import { readPref, savePref } from "./prefs";
import { notify } from "./state";

const HIDDEN_PREF = "fm.hiddenItems";
const EXT_PREF = "fm.extensions";

// Kept in memory too, so the choice still holds for this session when storage is blocked.
let hiddenItems = readPref(HIDDEN_PREF) === "shown";
let extensions = readPref(EXT_PREF) !== "hidden";

export const hiddenItemsShown = (): boolean => hiddenItems;
export const extensionsShown = (): boolean => extensions;

export function setHiddenItemsShown(on: boolean): void {
  hiddenItems = on;
  savePref(HIDDEN_PREF, on ? "shown" : "hidden");
}

export function setExtensionsShown(on: boolean): void {
  extensions = on;
  savePref(EXT_PREF, on ? "shown" : "hidden");
  notify();
}
