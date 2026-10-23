// Context menu shared by every view; item enablement comes only from node.actions.
import { el } from "./dom";
import { guiIcon } from "./gui-icons";
import { STATUSES, TYPES } from "./node-info";
import { remove, setDetails, setFavorite, setType } from "./ops";
import { select, startEdit } from "./state";
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

interface Submenu {
  element: HTMLElement;
  trigger: HTMLButtonElement;
  choices: HTMLButtonElement[];
  open(): void;
  close(): void;
  onKey(ev: KeyboardEvent): void;
}

// A trigger row that opens its choices in a flyout beside it, so long option lists stay out of the main menu.
function submenu(label: string, choices: HTMLButtonElement[], enabled: boolean): Submenu {
  const trigger = menuItem(label, "menuitem", enabled, () => open(), "ctx-item ctx-sub-trigger");
  trigger.setAttribute("aria-haspopup", "menu");
  trigger.setAttribute("aria-expanded", "false");
  trigger.append(guiIcon("chevronRight"));
  const flyout = el("div", { class: "ctx-menu ctx-flyout", role: "menu", "aria-label": label }, ...choices);
  flyout.hidden = true;
  const element = el("div", { class: "ctx-sub" }, trigger, flyout);

  function open(): void {
    if (!enabled) return;
    flyout.hidden = false;
    trigger.setAttribute("aria-expanded", "true");
    const t = trigger.getBoundingClientRect();
    const { width, height } = flyout.getBoundingClientRect();
    const fitsRight = t.right + width + EDGE <= window.innerWidth;
    flyout.style.left = `${fitsRight ? t.right : Math.max(EDGE, t.left - width)}px`;
    flyout.style.top = `${Math.max(EDGE, Math.min(t.top - 4, window.innerHeight - height - EDGE))}px`;
  }
  function close(): void {
    flyout.hidden = true;
    trigger.setAttribute("aria-expanded", "false");
  }
  function onKey(ev: KeyboardEvent): void {
    const i = choices.indexOf(document.activeElement as HTMLButtonElement);
    if (ev.key === "ArrowDown") choices[(i + 1) % choices.length]?.focus();
    else if (ev.key === "ArrowUp") choices[(i - 1 + choices.length) % choices.length]?.focus();
    else if (ev.key === "ArrowLeft" || ev.key === "Escape") {
      close();
      trigger.focus();
    } else return;
    ev.preventDefault();
    ev.stopPropagation();
  }
  return { element, trigger, choices, open, close, onKey };
}

function openMenu({ node, x, y }: MenuRequest, restoreFocus: () => void): void {
  dismissOpen?.();
  const a = node.actions;

  const rename = menuItem("Rename", "menuitem", a.rename, () => {
    close(false);
    startEdit(node.relParts);
  });
  const favorite = menuItem(node.favorite ? "Remove from favorites" : "Add to favorites", "menuitem", !node.ghost && !node.reserved, () => {
    close();
    void setFavorite(node, !node.favorite);
  });
  const typeChoices = TYPES.map(([base, label]) => {
    const b = menuItem(label, "menuitemradio", a.changeType, () => {
      close();
      void setType(node, base, node.status ?? "in-progress");
    });
    b.setAttribute("aria-checked", String(node.nodeType?.base === base));
    return b;
  });
  const statusChoices = STATUSES.map(([value, label]) => {
    const b = menuItem(label, "menuitemradio", a.changeType && !node.ghost, () => {
      close();
      void setDetails(node, value, null);
    });
    b.setAttribute("aria-checked", String(node.status === value));
    b.classList.add(`st-${value}`);
    return b;
  });
  const subs: Submenu[] = [submenu("Change type", typeChoices, a.changeType)];
  if (node.nodeType?.base === "WKB") subs.push(submenu("Status", statusChoices, a.changeType && !node.ghost));
  const copy = menuItem("Copy path", "menuitem", a.copyPath, () => {
    close();
    void copyPath(node);
  });
  const del = menuItem("Delete", "menuitem", a.delete, () => {
    close();
    void remove(node);
  }, "ctx-item danger");

  const items = [rename, favorite, ...subs.map((s) => s.trigger), copy, del];
  const closeSubs = () => subs.forEach((s) => s.close());
  items.forEach((item) => item.addEventListener("mouseenter", closeSubs));
  subs.forEach((s) => s.trigger.addEventListener("mouseenter", s.open));
  const menu = el(
    "div",
    { class: "ctx-menu", role: "menu", "aria-label": `Actions for ${node.name}` },
    rename,
    favorite,
    ...subs.map((s) => s.element),
    el("div", { class: "ctx-sep", role: "separator" }),
    copy,
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
    const inside = subs.find((s) => s.element.contains(document.activeElement) && document.activeElement !== s.trigger);
    if (inside) {
      inside.onKey(ev);
      return;
    }
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    const current = subs.find((s) => s.trigger === document.activeElement);
    if (current && (ev.key === "ArrowRight" || ev.key === "Enter" || ev.key === " ")) {
      current.open();
      (current.choices.find((c) => c.getAttribute("aria-checked") === "true") ?? current.choices[0])?.focus();
      ev.preventDefault();
      ev.stopPropagation();
      return;
    }
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
