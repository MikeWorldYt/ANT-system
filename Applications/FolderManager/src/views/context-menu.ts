// Context menu shared by every view; item enablement comes only from node.actions.
import { describeError, openNode } from "../ipc/api";
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { statusIcon } from "../ui/status-mark";
import { STATUSES, TYPES } from "../store/node-info";
import { addNext, pasteFiles, remove, setDetails, setFavorite, setType, setUserFlag } from "../actions/ops";
import { createShortcut, putOnClipboard } from "../actions/folder-share";
import { select, startEdit } from "../store/state";
import { setStatus } from "../ui/status";
import { menuItems, runMenuItem } from "../extensions/host";
import { autoDismiss, focusInitial, menuItem, place, rove, separator, submenu, type Submenu } from "../ui/menu";
import { boardMenu, type MenuParts } from "./board-menu";
import type { ViewPicker } from "../shell/toolbar";
import type { TreeNode } from "../ipc/types";

interface MenuRequest {
  node: TreeNode;
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

function dispatch(host: Element, node: TreeNode, x: number, y: number, folderOnly = false): void {
  host.dispatchEvent(new CustomEvent<MenuRequest>("fm:node-menu", { bubbles: true, detail: { node, x, y, folderOnly } }));
}

export function nodeContextMenu(ev: MouseEvent, node: TreeNode, host: Element): void {
  ev.preventDefault();
  // The view background listens too; an item's menu must not reach it.
  ev.stopPropagation();
  // Keyboard-originated contextmenu events carry button 0 and are handled on keydown.
  if (ev.button !== 2) return;
  select(node.relParts);
  dispatch(host, node, ev.clientX, ev.clientY);
}

// Background menu of the open folder; null (the storage root) has no menu.
export function folderContextMenu(ev: MouseEvent, folder: TreeNode | null, host: Element): void {
  ev.preventDefault();
  ev.stopPropagation();
  if (ev.button !== 2 || !folder) return;
  select(folder.relParts);
  dispatch(host, folder, ev.clientX, ev.clientY, true);
}

export const isMenuKey = (ev: KeyboardEvent) => ev.key === "ContextMenu" || (ev.key === "F10" && ev.shiftKey);

export function nodeMenuFromKey(host: Element, node: TreeNode, anchor: Element | null): void {
  const r = (anchor ?? host).getBoundingClientRect();
  dispatch(host, node, r.left + 24, r.bottom);
}

async function copyPath(node: TreeNode): Promise<void> {
  try {
    await navigator.clipboard.writeText(node.absPath);
    setStatus("info", `Path copied: ${node.absPath}`);
  } catch {
    setStatus("warning", `Could not copy to the clipboard. Path: ${node.absPath}`);
  }
}

async function openFolder(node: TreeNode): Promise<void> {
  try {
    await openNode(node.relParts);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

function nodeMenu(node: TreeNode, close: (restore?: boolean) => void): MenuParts {
  const a = node.actions;

  const open = menuItem("Open", "menuitem", a.open, () => {
    close();
    void openFolder(node);
  });
  const copyPathItem = menuItem("Copy path", "menuitem", a.copyPath, () => {
    close();
    void copyPath(node);
  });
  const favorite = menuItem(node.favorite ? "Remove from favorites" : "Add to favorites", "menuitem", a.favorite, () => {
    close();
    void setFavorite(node, !node.favorite);
  });
  favorite.prepend(guiIcon("starred"));
  favorite.classList.toggle("ctx-favorite-add", !node.favorite);
  const rename = menuItem("Rename", "menuitem", a.rename, () => {
    close(false);
    startEdit(node.relParts);
  });
  // Ungated like Ctrl+C: the backend explains a refusal.
  const copy = menuItem("Copy", "menuitem", true, () => {
    close();
    void putOnClipboard(node, false);
  });
  // Enabled even on locked folders: the backend picks the real target and explains a refusal.
  const paste = menuItem("Paste", "menuitem", true, () => {
    close();
    void pasteFiles(node.relParts);
  });
  const shortcut = menuItem("Create shortcut", "menuitem", true, () => {
    close();
    void createShortcut(node);
  });
  const shareItems = node.reserved || node.ghost ? [] : [paste, shortcut];
  const allowsFiles = node.flags.includes("data");
  const files = menuItem(allowsFiles ? "Deny files" : "Allow files", "menuitem", a.editFlags, () => {
    close();
    void setUserFlag(node, !allowsFiles);
  });
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
  const newNode = menuItem("New node", "menuitem", a.addNext, () => {
    close();
    void addNext(node, "node");
  });
  const newWorkboard = menuItem("New workboard", "menuitem", a.addNext && node.relParts.length > 1, () => {
    close();
    void addNext(node, "workboard");
  });
  const del = menuItem("Delete", "menuitem", a.delete, () => {
    close();
    void remove(node);
  }, "ctx-item danger");
  del.prepend(guiIcon("trash"));

  const extras = menuItems().map((m) =>
    menuItem(m.label, "menuitem", true, () => {
      close();
      runMenuItem(m.extId, m.id, node);
    }),
  );

  return {
    items: [open, copyPathItem, favorite, rename, copy, ...shareItems, ...subs.map((s) => s.trigger), ...fileItems, newNode, newWorkboard, del, ...extras],
    subs,
    content: [
      open, copyPathItem, favorite, separator(),
      rename, copy, ...shareItems, separator(),
      ...subs.map((s) => s.element), ...fileItems, separator(),
      newNode, newWorkboard, separator(),
      del, ...(extras.length > 0 ? [separator(), ...extras] : []),
    ],
  };
}

function openMenu({ node, x, y, folderOnly }: MenuRequest, restoreFocus: () => void, views: ViewPicker): void {
  dismissOpen?.();
  const { items, subs, content } = folderOnly ? boardMenu(node, views, close) : nodeMenu(node, close);
  const closeSubs = () => subs.filter((s) => !s.element.contains(document.activeElement)).forEach((s) => s.close());
  items.forEach((item) => item.addEventListener("mouseenter", closeSubs));
  subs.forEach((s) => s.trigger.addEventListener("mouseenter", s.open));
  const menu = el("div", { class: "ctx-menu ctx-node", role: "menu", "aria-label": `Actions for ${node.name}` }, ...content);
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
