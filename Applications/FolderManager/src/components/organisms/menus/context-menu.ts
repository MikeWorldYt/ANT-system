// Context menu shared by every view; item enablement comes only from node.actions.
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { statusIcon } from "../../atoms/status-mark";
import { importantLabel, isWorkboard, ownerOf, STATUSES, TYPES } from "../../../services/store/node-info";
import { addChild, hideMany, hideNode, pasteFiles, remove, removeMany, setDetails, setFavorite, setFavorites, setImportant, setLinkHidden, setManyDetails, setType, setUserFlag } from "../../../hooks/ops";
import { suggestSearch } from "../../../services/ipc/api";
import { reportError } from "../bars/status-line";
import { createShortcut, putOnClipboard } from "../../../hooks/folder-share";
import { openTab } from "../../../hooks/tabs";
import { copyPath, openFolder, openInApp, openInSplit } from "../../../hooks/path-actions";
import { isSelected, keyOf, selectedNodes, startEdit, state } from "../../../services/store/state";
import { menuItems, runMenuItem } from "../../../extensions/host";
import { autoDismiss, focusInitial, closingItem, menuItem, place, rove, separator, submenu, type Submenu } from "../../molecules/menu";
import { boardMenu, type MenuParts } from "./board-menu";
import type { ViewPicker } from "../bars/toolbar";
import { inDownloads, type TreeNode } from "../../../services/ipc/types";

interface MenuRequest {
  node: TreeNode;
  // Set when the menu acts on a marked group; node is then the one that was clicked.
  nodes?: TreeNode[];
  x: number;
  y: number;
  folderOnly?: boolean;
}

declare global {
  interface HTMLElementEventMap {
    "fm:node-menu": CustomEvent<MenuRequest>;
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
  if (ev.button !== 2 || !folder) return;
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
  if (!one) return { items: [copy], subs: [], content: [copy] };
  const open = action(one.file ? "Open" : "Open folder location", one.actions.open, () => void openFolder(one));
  const copyPathItem = action("Copy path", one.actions.copyPath, () => void copyPath(one));
  return { items: [open, copyPathItem, copy], subs: [], content: [open, copyPathItem, separator(), copy] };
}

function nodeMenu(node: TreeNode, close: (restore?: boolean) => void): MenuParts {
  const action = closingItem(close);
  if (inDownloads(node.relParts)) return downloadsMenu([node], close);
  const a = node.actions;

  const open = action(node.file ? "Open" : "Open folder location", a.open, () => void openFolder(node));
  const copyPathItem = action("Copy path", a.copyPath, () => void copyPath(node));
  const favorite = action(node.favorite ? "Remove from favorites" : "Add to favorites", a.favorite, () => void setFavorite(node, !node.favorite));
  favorite.prepend(guiIcon("starred"));
  favorite.classList.toggle("ctx-favorite-add", !node.favorite);
  const important = action(importantLabel(node.important), a.important, () => void setImportant([node], !node.important));
  important.prepend(guiIcon("important"));
  important.classList.toggle("ctx-important-add", !node.important);
  // Only workboards are placed on the board, so the item is left out for other folders.
  const importantItems = isWorkboard(node) ? [important] : [];
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
  // Enabled even on locked folders: the backend picks the real target and explains a refusal.
  const paste = action("Paste", true, () => void pasteFiles(node.relParts));
  // A review link only opens its owner folder, so the actions that change the folder do not apply.
  if (node.linked) {
    return { items: [open, copyPathItem, favorite, ...hideItems, copy, paste], subs: [], content: [open, copyPathItem, favorite, ...hideItems, separator(), copy, paste] };
  }
  const shortcut = action("Create shortcut", true, () => void createShortcut(node));
  const openHere = action("Open", true, () => void openInApp(node));
  const newTab = action("Open in new tab", true, () => openTab(node.relParts));
  const splitTab = action("Open in split tab", true, () => void openInSplit(node));
  const shareItems = node.reserved || node.ghost ? [] : [paste, shortcut];
  const allowsFiles = node.flags.includes("data");
  const files = action(allowsFiles ? "Deny files" : "Allow files", a.editFlags, () => void setUserFlag(node, !allowsFiles));
  const fileItems = node.nodeType?.base === "NODE" ? [files] : [];
  const typeChoices = TYPES.map(([base, label]) =>
    menuItem(label, "menuitemradio", a.changeType, () => {
      close();
      void setType(node, base, node.status ?? "in-progress");
    }, "ctx-item", node.nodeType?.base === base),
  );
  const statusChoices = STATUSES.map(([value, label]) => {
    const item = menuItem(label, "menuitemradio", a.editDetails, () => {
      close();
      void setDetails(node, value, null);
    }, `ctx-item ctx-status st-${value}`, node.status === value);
    item.replaceChildren(guiIcon(statusIcon(value)), label);
    return item;
  });
  const subs: Submenu[] = [submenu("Change type", typeChoices, a.changeType)];
  if (node.nodeType?.base === "WKB") subs.push(submenu("Change status", statusChoices, a.editDetails));
  const newNode = action(a.addFolder ? "New folder" : "New node", a.addChild || a.addFolder, () => void addChild(node, "node"));
  const newWorkboard = action("New workboard", (a.addChild || a.addFolder) && node.relParts.length > 1, () => void addChild(node, "workboard"));
  const del = menuItem("Delete", "menuitem", a.delete, () => {
    close();
    void remove(node);
  }, "ctx-item danger");
  del.prepend(guiIcon("trash"));
  const hideElement = action("Hide element", a.hide, () => void hideNode(node));
  hideElement.prepend(guiIcon("hide"));

  // A file offers only the actions that apply to one file.
  if (node.file) {
    return {
      items: [open, copyPathItem, rename, copy, cut, shortcut, del],
      subs: [],
      content: [open, copyPathItem, separator(), rename, copy, cut, shortcut, separator(), del],
    };
  }

  const extras = menuItems().map((m) =>
    action(m.label, true, () => runMenuItem(m.extId, m.id, node)),
  );

  return {
    items: [openHere, open, newTab, splitTab, copyPathItem, favorite, ...importantItems, ...hideItems, rename, copy, cut, ...shareItems, ...subs.map((s) => s.trigger), ...fileItems, newNode, newWorkboard, hideElement, del, ...extras],
    subs,
    content: [
      openHere, open, newTab, splitTab, copyPathItem, favorite, ...importantItems, ...hideItems, separator(),
      rename, copy, cut, ...shareItems, separator(),
      ...subs.map((s) => s.element), ...fileItems, separator(),
      newNode, newWorkboard, separator(),
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

// Workboard-only actions show only when every marked item is a workboard.
function workboardItems(nodes: TreeNode[], close: () => void): { items: HTMLButtonElement[]; subs: Submenu[] } {
  const action = closingItem(close);
  if (!nodes.every(isWorkboard)) return { items: [], subs: [] };
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
  const wkb = workboardItems(nodes, close);
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
  const downloads = (group ?? [node]).every((n) => inDownloads(n.relParts));
  openMenuWith(
    (close) => (group && downloads ? downloadsMenu(group, close) : folderOnly && !downloads ? boardMenu(node, views, close) : group ? selectionMenu(group, close) : nodeMenu(node, close)),
    label,
    x,
    y,
    restoreFocus,
  );
}

export function openMenuWith(build: (close: (restore?: boolean) => void) => MenuParts, label: string, x: number, y: number, restoreFocus: () => void): void {
  dismissOpen?.();
  const { items, subs, content } = build(close);
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
}
