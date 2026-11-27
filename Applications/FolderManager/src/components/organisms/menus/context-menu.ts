// Context menu shared by every view; item enablement comes only from node.actions.
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { kindIcon } from "../../atoms/icons";
import { menuKeys } from "../tooltip/tooltip";
import { statusIcon } from "../../atoms/status-mark";
import { importantLabel, isWorknode, ownerOf, STATUSES, TYPES } from "../../../services/store/node-info";
import { hideMany, hideNode, remove, removeDownloads, removeMany, setDetails, setFavorite, setFavorites, setImportant, setLinkHidden, setManyDetails, setType } from "../../../hooks/ops";
import { revealNode, suggestSearch } from "../../../services/ipc/api";
import { reportError } from "../bars/status-line";
import { createShortcut, putOnClipboard } from "../../../hooks/folder-share";
import { openTab } from "../../../hooks/tabs";
import { copyPath, openFolder, openInApp, openInSplit } from "../../../hooks/path-actions";
import { isSelected, keyOf, selectedNodes, startEdit, state, virtualRoot } from "../../../services/store/state";
import { menuItems, runMenuItem } from "../../../extensions/host";
import { autoDismiss, menuIcon, focusInitial, closingItem, menuItem, place, rove, separator, submenu, type Submenu } from "../../molecules/menu";
import { boardMenu, type MenuParts } from "./board-menu";
import { nativeBackgroundMenu } from "./native-menu";
import { folderFlagItems } from "./folder-flags";
import type { ViewPicker } from "../bars/toolbar";
import { allInSystem } from "../../../services/store/system-folders";
import { inSystem, type TreeNode } from "../../../services/ipc/types";

interface MenuRequest {
  node: TreeNode;
  // Set when the menu acts on a marked group; node is then the one that was clicked.
  nodes?: TreeNode[];
  x: number;
  y: number;
  folderOnly?: boolean;
}

interface NativeMenuRequest {
  parts: string[];
  x: number;
  y: number;
}

declare global {
  interface HTMLElementEventMap {
    "fm:node-menu": CustomEvent<MenuRequest>;
    "fm:native-menu": CustomEvent<NativeMenuRequest>;
  }
}

let dismissOpen: (() => void) | null = null;

function dispatch(host: Element, detail: MenuRequest): void {
  host.dispatchEvent(new CustomEvent<MenuRequest>("fm:node-menu", { bubbles: true, detail }));
}

export function nodeContextMenu(ev: MouseEvent, node: TreeNode, host: Element): void {
  ev.preventDefault();
  // The view background listens too; an item's menu must not reach it.
  ev.stopPropagation();
  // Keyboard-originated contextmenu events carry button 0 and are handled on keydown.
  if (ev.button !== 2) return;
  // Right-clicking a marked item keeps the marks, so the menu acts on all of them.
  if (isSelected(keyOf(node.relParts)) && state.selectedKeys.length > 1) {
    dispatch(host, { node, nodes: selectedNodes(), x: ev.clientX, y: ev.clientY });
    return;
  }
  // The clicked item only shows its menu; the selection and the open columns stay where they are.
  if (ev.target instanceof Element) ev.target.closest("[data-key]")?.classList.add("menu-target");
  dispatch(host, { node, x: ev.clientX, y: ev.clientY });
}

// Background menu of the open folder; null (the storage root) has no menu.
export function folderContextMenu(ev: MouseEvent, folder: TreeNode | null, host: Element): void {
  ev.preventDefault();
  ev.stopPropagation();
  if (ev.button !== 2) return;
  const native = virtualRoot(state.virtual);
  // A native folder's own list has no node, so its menu is built from the root marker alone.
  if (!folder && native !== null) {
    host.dispatchEvent(new CustomEvent<NativeMenuRequest>("fm:native-menu", { bubbles: true, detail: { parts: [native], x: ev.clientX, y: ev.clientY } }));
    return;
  }
  if (!folder) return;
  // The background menu acts on its folder without selecting it, so the open columns stay put.
  dispatch(host, { node: folder, x: ev.clientX, y: ev.clientY, folderOnly: true });
}

export const isMenuKey = (ev: KeyboardEvent) => ev.key === "ContextMenu" || (ev.key === "F10" && ev.shiftKey);

export function nodeMenuFromKey(host: Element, node: TreeNode, anchor: Element | null): void {
  const r = (anchor ?? host).getBoundingClientRect();
  const nodes = isSelected(keyOf(node.relParts)) && state.selectedKeys.length > 1 ? selectedNodes() : undefined;
  dispatch(host, { node, nodes, x: r.left + 24, y: r.bottom });
}

// Items in Downloads only open and copy: nothing there is a storage node, and the files are not ours to change.
function downloadsMenu(nodes: TreeNode[], close: (restore?: boolean) => void): MenuParts {
  const action = closingItem(close);
  const [one] = nodes.length === 1 ? nodes : [];
  const copy = action("Copy", true, () => void putOnClipboard(nodes, false));
  const cut = action("Cut", true, () => void putOnClipboard(nodes, true));
  const del = menuItem("Delete", "menuitem", true, () => {
    close();
    void removeDownloads(nodes);
  }, "ctx-item danger");
  del.prepend(guiIcon("trash"));
  if (!one) return { items: [copy, cut, del], subs: [], content: [copy, cut, separator(), del] };
  const open = action(one.file ? "Open" : "Open folder location", one.actions.open, () => void openFolder(one));
  const copyPathItem = action("Copy path", one.actions.copyPath, () => void copyPath(one));
  const rename = menuItem("Rename", "menuitem", one.actions.rename, () => {
    close(false);
    startEdit(one.relParts);
  });
  const locate = one.file ? [revealItem(one, action)] : [];
  return { items: [open, ...locate, copyPathItem, copy, cut, rename, del], subs: [], content: [open, ...locate, copyPathItem, separator(), copy, cut, rename, separator(), del] };
}

// Shows the file in the system explorer with it selected inside its folder.
function revealItem(file: TreeNode, action: ReturnType<typeof closingItem>): HTMLButtonElement {
  return action("Open folder location", true, () => void revealNode(file.relParts).catch((e: unknown) => reportError("error", e)));
}

function nodeMenu(node: TreeNode, close: (restore?: boolean) => void): MenuParts {
  const action = closingItem(close);
  if (inSystem(node.relParts)) return downloadsMenu([node], close);
  const a = node.actions;

  const open = action(node.file ? "Open" : "Open folder location", a.open, () => void openFolder(node));
  const copyPathItem = action("Copy path", a.copyPath, () => void copyPath(node));
  const favorite = action(node.favorite ? "Remove from favorites" : "Add to favorites", a.favorite, () => void setFavorite(node, !node.favorite));
  favorite.prepend(guiIcon("starred"));
  favorite.classList.toggle("ctx-favorite-add", !node.favorite);
  const important = action(importantLabel(node.important), a.important, () => void setImportant([node], !node.important));
  important.prepend(guiIcon("important"));
  important.classList.toggle("ctx-important-add", !node.important);
  // Only worknodes are placed on the board, so the item is left out for other folders.
  const importantItems = isWorknode(node) ? [important] : [];
  const hide = action(node.hidden ? "Show element" : "Hide element", a.hideLink, () => void setLinkHidden(node, !node.hidden));
  // Only review links carry the action, so the item is left out everywhere else.
  const hideItems = a.hideLink ? [hide] : [];
  const rename = menuItem("Rename", "menuitem", a.rename, () => {
    close(false);
    startEdit(node.relParts);
  });
  // Ungated like Ctrl+C: the backend explains a refusal.
  const copy = action("Copy", true, () => void putOnClipboard([node], false));
  // The backend refuses a cut that is not allowed, such as a folder that holds no loose data.
  const cut = action("Cut", true, () => void putOnClipboard([node], true));
  // A review link only opens its owner folder, so the actions that change the folder do not apply.
  if (node.linked) {
    return { items: [open, copyPathItem, favorite, ...hideItems, copy], subs: [], content: [open, copyPathItem, favorite, ...hideItems, separator(), copy] };
  }
  const shortcut = action("Create shortcut", true, () => void createShortcut(node));
  for (const [item, icon] of [[rename, "pencil"], [copy, "duplicate"], [cut, "cut"], [shortcut, "link"]] as const) item.prepend(guiIcon(icon));
  const openHere = action("Open", true, () => void openInApp(node));
  const newTab = action("Open in new tab", true, () => openTab(node.relParts));
  const splitTab = action("Open in split tab", true, () => void openInSplit(node));
  for (const [item, icon] of [[newTab, "add"], [splitTab, "split"]] as const) {
    item.classList.add("ctx-iconed");
    item.prepend(menuIcon(icon));
  }
  const shareItems = node.reserved || node.ghost ? [] : [shortcut];
  const fileItems = folderFlagItems(node, action);
  const typeChoices = TYPES.map(([base, label]) => {
    const item = menuItem(label, "menuitemradio", a.changeType, () => {
      close();
      void setType(node, base, node.status ?? "in-progress");
    }, "ctx-item ctx-iconed", node.nodeType?.base === base);
    const icon = kindIcon(base === "WKB" ? "wkb" : "folder");
    icon.classList.add("ctx-icon");
    item.prepend(icon);
    return item;
  });
  const statusChoices = STATUSES.map(([value, label]) => {
    const item = menuItem(label, "menuitemradio", a.editDetails, () => {
      close();
      void setDetails(node, value, null);
    }, `ctx-item ctx-status st-${value}`, node.status === value);
    item.replaceChildren(guiIcon(statusIcon(value)), label);
    return item;
  });
  // Rarely used actions sit one level down, so the everyday ones stay short to scan.
  const openIn = submenu("Open in", [newTab, splitTab], true);
  const subs: Submenu[] = [submenu("Change type", typeChoices, a.changeType)];
  if (node.nodeType?.base === "WKB") subs.push(submenu("Change status", statusChoices, a.editDetails));
  const del = menuItem("Delete", "menuitem", a.delete, () => {
    close();
    void remove(node);
  }, "ctx-item danger");
  del.prepend(guiIcon("trash"));
  const hideElement = action("Hide element", a.hide, () => void hideNode(node));
  hideElement.prepend(guiIcon("hide"));

  // A file offers only the actions that apply to one file.
  if (node.file) {
    const revealFile = revealItem(node, action);
    return {
      items: [open, revealFile, copyPathItem, rename, copy, cut, shortcut, del],
      subs: [],
      content: [open, revealFile, copyPathItem, separator(), rename, copy, cut, shortcut, separator(), del],
    };
  }

  const extras = menuItems().map((m) =>
    action(m.label, true, () => runMenuItem(m.extId, m.id, node)),
  );

  return {
    items: [openHere, open, openIn.trigger, copyPathItem, favorite, ...importantItems, ...hideItems, rename, copy, cut, ...shareItems, ...subs.map((s) => s.trigger), ...fileItems, hideElement, del, ...extras],
    subs: [openIn, ...subs],
    content: [
      openHere, open, openIn.element, copyPathItem, favorite, ...importantItems, ...hideItems, separator(),
      rename, copy, cut, ...shareItems, separator(),
      ...subs.map((s) => s.element), ...fileItems, separator(),
      hideElement, del, ...(extras.length > 0 ? [separator(), ...extras] : []),
    ],
  };
}

// Owners come from the index asynchronously, so the flyout fills in after the menu is shown.
function ownerSubmenu(nodes: TreeNode[], close: () => void, enabled: boolean): Submenu {
  const sub = submenu("Change owner", [], enabled);
  const flyout = sub.element.querySelector(".ctx-flyout");
  suggestSearch("owner", "")
    .then((owners) => {
      const items = owners.map((o) =>
        menuItem(o.value, "menuitemradio", true, () => {
          close();
          void setManyDetails(nodes, null, o.value);
        }, "ctx-item", nodes.every((n) => ownerOf(n) === o.value)),
      );
      sub.choices.push(...items);
      flyout?.append(...items);
    })
    .catch((e: unknown) => reportError("warning", e));
  return sub;
}

// Worknode-only actions show only when every marked item is a worknode.
function worknodeItems(nodes: TreeNode[], close: () => void): { items: HTMLButtonElement[]; subs: Submenu[] } {
  const action = closingItem(close);
  if (!nodes.every(isWorknode)) return { items: [], subs: [] };
  const statusChoices = STATUSES.map(([value, label]) => {
    const item = menuItem(label, "menuitemradio", true, () => {
      close();
      void setManyDetails(nodes, value, null);
    }, `ctx-item ctx-status st-${value}`, nodes.every((n) => n.status === value));
    item.replaceChildren(guiIcon(statusIcon(value)), label);
    return item;
  });
  const editable = nodes.every((n) => n.actions.editDetails);
  const subs = [submenu("Change status", statusChoices, editable), ownerSubmenu(nodes, close, editable)];
  const on = !nodes.every((n) => n.important);
  const important = action(importantLabel(!on), nodes.every((n) => n.actions.important), () => void setImportant(nodes, on));
  important.prepend(guiIcon("important"));
  important.classList.toggle("ctx-important-add", on);
  return { items: [important], subs };
}

// Several marked items share Copy, Cut and Delete; Delete needs every one of them deletable.
function selectionMenu(nodes: TreeNode[], close: (restore?: boolean) => void): MenuParts {
  const action = closingItem(close);
  const favOn = !nodes.every((n) => n.favorite);
  const favorite = action(favOn ? "Add to favorites" : "Remove from favorites", nodes.every((n) => n.actions.favorite), () => void setFavorites(nodes, favOn));
  favorite.prepend(guiIcon("starred"));
  favorite.classList.toggle("ctx-favorite-add", favOn);
  const wkb = worknodeItems(nodes, close);
  const hideElement = action("Hide elements", nodes.every((n) => n.actions.hide), () => void hideMany(nodes));
  hideElement.prepend(guiIcon("hide"));
  const copy = action("Copy", true, () => void putOnClipboard(nodes, false));
  const cut = action("Cut", true, () => void putOnClipboard(nodes, true));
  const del = menuItem("Delete", "menuitem", nodes.every((n) => n.actions.delete), () => {
    close();
    void removeMany(nodes);
  }, "ctx-item danger");
  del.prepend(guiIcon("trash"));
  const subItems = wkb.subs.map((s) => s.trigger);
  return {
    items: [favorite, ...wkb.items, copy, cut, ...subItems, hideElement, del],
    subs: wkb.subs,
    content: [
      favorite, ...wkb.items, separator(),
      copy, cut, separator(),
      ...(wkb.subs.length > 0 ? [...wkb.subs.map((s) => s.element), separator()] : []),
      hideElement, del,
    ],
  };
}

function openMenu({ node, nodes, x, y, folderOnly }: MenuRequest, restoreFocus: () => void, views: ViewPicker): void {
  const group = nodes !== undefined && nodes.length > 1 ? nodes : null;
  const label = group ? `Actions for ${group.length} items` : `Actions for ${node.name}`;
  const downloads = allInSystem(group ?? [node]);
  openMenuWith(
    (close) => (group && downloads ? downloadsMenu(group, close) : folderOnly && downloads ? nativeBackgroundMenu(node.relParts, views, close) : folderOnly && !downloads ? boardMenu(node, views, close) : group ? selectionMenu(group, close) : nodeMenu(node, close)),
    label,
    x,
    y,
    restoreFocus,
  );
}

export function openMenuWith(build: (close: (restore?: boolean) => void) => MenuParts, label: string, x: number, y: number, restoreFocus: () => void): void {
  dismissOpen?.();
  const { items, subs, content } = build(close);
  for (const item of [...items, ...subs.flatMap((s) => s.choices)]) {
    const keys = subs.some((s) => s.trigger === item) ? [] : menuKeys(item.textContent?.trim() ?? "");
    if (keys.length > 0) item.append(el("span", { class: "app-tooltip-keys ctx-keys", "aria-hidden": "true" }, ...keys.map((k) => el("kbd", {}, k))));
  }
  const closeSubs = () => subs.filter((s) => !s.element.contains(document.activeElement)).forEach((s) => s.close());
  items.forEach((item) => item.addEventListener("mouseenter", closeSubs));
  subs.forEach((s) => s.trigger.addEventListener("mouseenter", s.open));
  const menu = el("div", { class: "ctx-menu ctx-node", role: "menu", "aria-label": label }, ...content);
  menu.addEventListener("mouseleave", closeSubs);

  function close(restore = true): void {
    menu.remove();
    stopWatching();
    dismissOpen = null;
    if (restore) restoreFocus();
  }

  menu.addEventListener("keydown", (ev) => {
    if (ev.key !== "Tab") {
      const inside = subs.find((s) => s.element.contains(document.activeElement) && document.activeElement !== s.trigger);
      if (inside) {
        inside.onKey(ev);
        return;
      }
      const current = subs.find((s) => s.trigger === document.activeElement);
      if (current && (ev.key === "ArrowRight" || ev.key === "Enter" || ev.key === " ")) {
        current.open();
        focusInitial(current.choices);
      } else if (ev.key !== "Escape" && !rove(items, ev)) return;
    }
    if (ev.key === "Tab" || ev.key === "Escape") close();
    ev.preventDefault();
    ev.stopPropagation();
  });

  document.body.append(menu);
  place(menu, x, y);
  const stopWatching = autoDismiss((t) => menu.contains(t), close, true);
  dismissOpen = () => close(false);
  items[0]?.focus();
}

export function installContextMenu(host: HTMLElement, restoreFocus: () => void, views: ViewPicker): void {
  // Suppresses the WebView2 default menu everywhere except text fields, which keep copy and paste.
  document.addEventListener("contextmenu", (ev) => {
    const t = ev.target;
    if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || (t instanceof HTMLElement && t.isContentEditable)) return;
    ev.preventDefault();
  });
  host.addEventListener("fm:node-menu", (ev) => openMenu(ev.detail, restoreFocus, views));
  host.addEventListener("fm:native-menu", ({ detail }) => openMenuWith((close) => nativeBackgroundMenu(detail.parts, views, close), "Folder", detail.x, detail.y, restoreFocus));
}
