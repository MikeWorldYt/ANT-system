// Footer of the navigation pane: names the open Storage and opens a menu to switch to another or manage the list.
import { el, keepClicks } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { iconButton } from "../../atoms/icon-button";
import { openSettings } from "../../windows/settings";
import { changeStoragePath, knownStorages } from "../../../services/ipc/api";
import type { KnownStorage } from "../../../services/ipc/api";
import { state } from "../../../services/store/state";
import { afterPick } from "../../../hooks/follow-storage";
import { EDGE, focusInitial, menuIcon, menuItem, openPopup, popupTrigger, rove, separator } from "../../molecules/menu";
import { sameFolder, storagePicker } from "../dialogs/storage-picker/storage-picker";
import { reportError } from "./status-line";

const folderName = (root: string | undefined): string => root?.split(/[\\/]/).filter(Boolean).pop() ?? "Storage";

export interface StorageFooter {
  element: HTMLElement;
  update(): void;
}

export function createStorageFooter(): StorageFooter {
  const name = el("strong", { class: "nav-store-name" });
  const arrows = el("span", { class: "nav-store-icon", "aria-hidden": "true" }, guiIcon("pageStorage"));
  const button = el("button", { type: "button", class: "nav-store", "aria-label": "Switch Storage" }, arrows, name);
  const gear = iconButton("Settings", "settings", () => openSettings());
  gear.classList.add("nav-store-settings");
  const element = el("div", { class: "nav-footer" }, button, gear);
  // The pane's own press handlers would pull focus back to the tree while the menu opens.
  keepClicks(element);
  element.addEventListener("pointerup", (ev) => ev.stopPropagation());
  let closeMenu: ((restore?: boolean) => void) | null = null;
  let opening = false;

  async function switchTo(path: string): Promise<void> {
    try {
      await changeStoragePath(path);
      await afterPick("opened");
    } catch (e) {
      reportError("error", e);
    }
  }

  async function manage(): Promise<void> {
    try {
      const picked = await storagePicker(state.tree?.root ?? null);
      if (picked) await afterPick(picked);
    } catch (e) {
      reportError("error", e);
    }
  }

  function choice(storage: KnownStorage, current: string | null, close: () => void): HTMLButtonElement {
    const isCurrent = current !== null && sameFolder(storage.path, current);
    const item = menuItem(storage.name, "menuitemradio", storage.exists, () => {
      close();
      if (!isCurrent) void switchTo(storage.path);
    }, `ctx-item ctx-iconed${isCurrent ? " ctx-current" : ""}`, isCurrent);
    item.prepend(menuIcon("pageStorage"));
    item.title = storage.exists ? storage.path : "Not found";
    if (!storage.exists) item.append(el("span", { class: "ctx-trail" }, "Not found"));
    else if (isCurrent) item.append(el("span", { class: "ctx-trail picker-tag" }, "Current"));
    return item;
  }

  async function open(): Promise<void> {
    if (opening) return;
    opening = true;
    let storages: KnownStorage[];
    try {
      storages = await knownStorages();
    } catch (e) {
      reportError("warning", e, "Storages");
      return;
    } finally {
      opening = false;
    }
    const close = (restore = true) => closeMenu?.(restore);
    const current = state.tree?.root ?? null;
    const manageItem = menuItem("Manage storages…", "menuitem", true, () => {
      close();
      void manage();
    }, "ctx-item ctx-iconed");
    manageItem.prepend(menuIcon("settings"));
    const choices = storages.map((s) => choice(s, current, close));
    const items = [...choices, manageItem];
    const menu = el("div", { class: "ctx-menu nav-store-menu", role: "menu", "aria-label": "Storages" }, el("div", { class: "ctx-group", role: "group" }, ...choices, separator(), manageItem));
    menu.addEventListener("keydown", (ev) => {
      if (ev.key === "Tab" || ev.key === "Escape") {
        close();
        ev.preventDefault();
      } else rove(items, ev);
    });
    menu.addEventListener("focusout", (ev) => {
      if (!menu.contains(ev.relatedTarget as Node | null)) close(false);
    });
    // Keeps a press on the menu padding from blurring the focused item, which would close the menu.
    menu.addEventListener("mousedown", (ev) => ev.preventDefault());
    const r = button.getBoundingClientRect();
    menu.style.minWidth = `${r.width}px`;
    closeMenu = openPopup(button, menu, () => {
      const { height } = menu.getBoundingClientRect();
      menu.style.left = `${Math.max(EDGE, r.left)}px`;
      menu.style.top = `${Math.max(EDGE, r.top - EDGE - height)}px`;
    }, () => (closeMenu = null));
    focusInitial(items);
  }

  button.setAttribute("aria-haspopup", "menu");
  button.setAttribute("aria-expanded", "false");
  popupTrigger(button, ["ArrowUp", "ArrowDown"], () => closeMenu, () => void open());

  return {
    element,
    update() {
      const text = folderName(state.tree?.root);
      name.textContent = text;
      button.title = `${text}: switch Storage`;
    },
  };
}
