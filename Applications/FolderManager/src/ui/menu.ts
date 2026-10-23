// Building blocks shared by the anchored dropdown and the node context menu.
import { el } from "./dom";
import { guiIcon } from "./gui-icons";

export const EDGE = 4;

export function menuItem(label: string, role: string, enabled: boolean, run: () => void, cls = "ctx-item", checked?: boolean): HTMLButtonElement {
  const b = el("button", { type: "button", role, tabindex: "-1", class: cls }, label);
  if (checked !== undefined) b.setAttribute("aria-checked", String(checked));
  if (!enabled) {
    b.setAttribute("aria-disabled", "true");
    b.title = "Not available for this item";
  }
  b.addEventListener("click", () => {
    if (enabled) run();
  });
  return b;
}

export const separator = () => el("div", { class: "ctx-sep", role: "separator" });

// Arrow keys, Home and End wrap around the items; returns whether the key was one of them.
export function rove(items: HTMLElement[], ev: KeyboardEvent): boolean {
  const i = items.indexOf(document.activeElement as HTMLElement);
  const to = ev.key === "ArrowDown" ? i + 1 : ev.key === "ArrowUp" ? i - 1 : ev.key === "Home" ? 0 : ev.key === "End" ? items.length - 1 : null;
  if (to === null) return false;
  items[(to + items.length) % items.length]?.focus();
  ev.preventDefault();
  return true;
}

export const focusInitial = (items: HTMLElement[]) => (items.find((b) => b.getAttribute("aria-checked") === "true") ?? items[0])?.focus();

// The menu must already be in the document so it can be measured.
export function place(menu: HTMLElement, x: number, y: number): void {
  const { width, height } = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(EDGE, Math.min(x, window.innerWidth - width - EDGE))}px`;
  menu.style.top = `${Math.max(EDGE, Math.min(y, window.innerHeight - height - EDGE))}px`;
}

export function autoDismiss(inside: (target: Node) => boolean, dismiss: (restore: boolean) => void, onBlur = false): () => void {
  const outside = (ev: Event) => {
    if (!inside(ev.target as Node)) dismiss(false);
  };
  const resized = () => dismiss(true);
  const blurred = () => dismiss(false);
  document.addEventListener("pointerdown", outside, true);
  window.addEventListener("resize", resized);
  if (onBlur) window.addEventListener("blur", blurred);
  return () => {
    document.removeEventListener("pointerdown", outside, true);
    window.removeEventListener("resize", resized);
    window.removeEventListener("blur", blurred);
  };
}

export interface Submenu {
  element: HTMLElement;
  trigger: HTMLButtonElement;
  choices: HTMLButtonElement[];
  open(): void;
  close(): void;
  onKey(ev: KeyboardEvent): void;
}

// A trigger row that opens its choices in a flyout beside it, so long option lists stay out of the main menu.
export function submenu(label: string, choices: HTMLButtonElement[], enabled: boolean): Submenu {
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
    const { width } = flyout.getBoundingClientRect();
    place(flyout, t.right + width + EDGE <= window.innerWidth ? t.right : t.left - width, t.top - 4);
  }
  function close(): void {
    flyout.hidden = true;
    trigger.setAttribute("aria-expanded", "false");
  }
  function onKey(ev: KeyboardEvent): void {
    if (ev.key === "ArrowLeft" || ev.key === "Escape") {
      close();
      trigger.focus();
    } else if (!rove(choices, ev)) return;
    ev.preventDefault();
    ev.stopPropagation();
  }
  return { element, trigger, choices, open, close, onKey };
}
