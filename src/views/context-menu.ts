// Context menu shared by every view; item enablement comes only from node.actions.
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { statusIcon } from "../ui/status-mark";
import { STATUSES, TYPES } from "../store/node-info";
import { remove, setDetails, setFavorite, setType } from "../actions/ops";
import { select, startEdit } from "../store/state";
import { setStatus } from "../ui/status";
import { autoDismiss, focusInitial, menuItem, place, rove, separator, submenu, type Submenu } from "../ui/menu";
import type { TreeNode } from "../ipc/types";

interface MenuRequest {
  node: TreeNode;
  x: number;
  y: number;
}

declare global {
  interface HTMLElementEventMap {
    "fm:node-menu": CustomEvent<MenuRequest>;
  }
}

let dismissOpen: (() => void) | null = null;

function dispatch(host: Element, node: TreeNode, x: number, y: number): void {
  host.dispatchEvent(new CustomEvent<MenuRequest>("fm:node-menu", { bubbles: true, detail: { node, x, y } }));
}

export function nodeContextMenu(ev: MouseEvent, node: TreeNode, host: Element): void {
  ev.preventDefault();
  // Keyboard-originated contextmenu events carry button 0 and are handled on keydown.
  if (ev.button !== 2) return;
  select(node.relParts);
  dispatch(host, node, ev.clientX, ev.clientY);
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

function openMenu({ node, x, y }: MenuRequest, restoreFocus: () => void): void {
  dismissOpen?.();
  const a = node.actions;

  const rename = menuItem("Rename", "menuitem", a.rename, () => {
    close(false);
    startEdit(node.relParts);
  });
  const favorite = menuItem(node.favorite ? "Remove from favorites" : "Add to favorites", "menuitem", a.favorite, () => {
    close();
    void setFavorite(node, !node.favorite);
  });
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
  if (node.nodeType?.base === "WKB") subs.push(submenu("Status", statusChoices, a.editDetails));
  const copy = menuItem("Copy path", "menuitem", a.copyPath, () => {
    close();
    void copyPath(node);
  });
  const del = menuItem("Delete", "menuitem", a.delete, () => {
    close();
    void remove(node);
  }, "ctx-item danger");

  const items = [rename, favorite, ...subs.map((s) => s.trigger), copy, del];
  const closeSubs = () => subs.filter((s) => !s.element.contains(document.activeElement)).forEach((s) => s.close());
  items.forEach((item) => item.addEventListener("mouseenter", closeSubs));
  subs.forEach((s) => s.trigger.addEventListener("mouseenter", s.open));
  const menu = el("div", { class: "ctx-menu", role: "menu", "aria-label": `Actions for ${node.name}` }, rename, favorite, ...subs.map((s) => s.element), separator(), copy, del);
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

export function installContextMenu(host: HTMLElement, restoreFocus: () => void): void {
  host.addEventListener("fm:node-menu", (ev) => openMenu(ev.detail, restoreFocus));
}
