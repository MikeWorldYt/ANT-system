// Context menu shared by every view; item enablement comes only from node.actions.
import { TYPES } from "./actions-panel";
import { el } from "./dom";
import { apply, editablePart, remove } from "./ops";
import { select } from "./state";
import { setStatus } from "./status";
import type { TreeNode } from "./types";

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

const EDGE = 4;
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

function menuItem(label: string, role: string, enabled: boolean, run: () => void, cls = "ctx-item"): HTMLButtonElement {
  const b = el("button", { type: "button", role, tabindex: "-1", class: cls }, label);
  if (!enabled) b.setAttribute("aria-disabled", "true");
  b.addEventListener("click", () => {
    if (enabled) run();
  });
  return b;
}

async function copyPath(node: TreeNode): Promise<void> {
  try {
    await navigator.clipboard.writeText(node.absPath);
    setStatus("info", `Path copied: ${node.absPath}`);
  } catch {
    setStatus("warning", `Could not copy to the clipboard. Path: ${node.absPath}`);
  }
}

function focusNameField(node: TreeNode): void {
  select(node.relParts);
  const input = document.querySelector<HTMLInputElement>("#f-name");
  input?.focus();
  input?.select();
}

function openMenu({ node, x, y }: MenuRequest, restoreFocus: () => void): void {
  dismissOpen?.();
  const a = node.actions;

  const rename = menuItem("Rename", "menuitem", a.rename, () => {
    close(false);
    focusNameField(node);
  });
  const types = TYPES.map(([base, label]) => {
    const b = menuItem(label, "menuitemradio", a.changeType, () => {
      close();
      void apply(node, { editable: editablePart(node.name), type: base, status: node.status ?? "in-progress" });
    });
    b.setAttribute("aria-checked", String(node.nodeType?.base === base));
    return b;
  });
  const copy = menuItem("Copy path", "menuitem", a.copyPath, () => {
    close();
    void copyPath(node);
  });
  const del = menuItem("Delete", "menuitem", a.delete, () => {
    close();
    void remove(node);
  }, "ctx-item danger");

  const items = [rename, ...types, copy, del];
  const group = el(
    "div",
    { class: "ctx-group", role: "group", "aria-label": "Change node type" },
    el("div", { class: "ctx-label", "aria-hidden": "true" }, "Change node type"),
    ...types,
  );
  const menu = el(
    "div",
    { class: "ctx-menu", role: "menu", "aria-label": `Actions for ${node.name}` },
    rename,
    group,
    copy,
    el("div", { class: "ctx-sep", role: "separator" }),
    del,
  );

  const outside = (ev: Event) => {
    if (!menu.contains(ev.target as Node)) close(false);
  };
  const dismiss = () => close(false);
  function close(restore = true): void {
    menu.remove();
    document.removeEventListener("pointerdown", outside, true);
    window.removeEventListener("blur", dismiss);
    window.removeEventListener("resize", dismiss);
    dismissOpen = null;
    if (restore) restoreFocus();
  }

  const move = (to: number) => items[(to + items.length) % items.length]?.focus();
  menu.addEventListener("keydown", (ev) => {
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    switch (ev.key) {
      case "ArrowDown":
        move(i + 1);
        break;
      case "ArrowUp":
        move(i - 1);
        break;
      case "Tab":
        move(ev.shiftKey ? i - 1 : i + 1);
        break;
      case "Home":
        move(0);
        break;
      case "End":
        move(items.length - 1);
        break;
      case "Escape":
        close();
        break;
      default:
        return;
    }
    ev.preventDefault();
    ev.stopPropagation();
  });

  document.body.append(menu);
  const { width, height } = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(EDGE, Math.min(x, window.innerWidth - width - EDGE))}px`;
  menu.style.top = `${Math.max(EDGE, Math.min(y, window.innerHeight - height - EDGE))}px`;
  document.addEventListener("pointerdown", outside, true);
  window.addEventListener("blur", dismiss);
  window.addEventListener("resize", dismiss);
  dismissOpen = dismiss;
  items[0]?.focus();
}

export function installContextMenu(host: HTMLElement, restoreFocus: () => void): void {
  host.addEventListener("fm:node-menu", (ev) => openMenu(ev.detail, restoreFocus));
}
