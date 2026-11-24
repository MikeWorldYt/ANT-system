// One tooltip for the whole app, in GitHub's style: it takes over every `title`, so the native one never shows, and adds the button's shortcut keys.
import { el } from "../../dom";
import { comboKeys } from "../../../services/key-label";
import { comboOf } from "../../../hooks/shortcuts";
import type { KeyCombo } from "../../../services/store/state";

const DELAY_MS = 400;
const GAP = 6;

// The buttons whose action has a shortcut, by their label; editable ones follow what the user set in Settings.
const SHORTCUTS: Record<string, string | KeyCombo> = {
  "New tab": { code: "KeyT", mod: true, alt: false },
  Add: "newNode",
  Settings: "openSettings",
  "Up one level": "goUp",
  "Collapse side panel": "toggleSidePanel",
  "Expand side panel": "toggleSidePanel",
  "Edit details": "editDetails",
  Delete: { code: "Delete", mod: false, alt: false },
};

const keysFor = (text: string): string[] => {
  const s = SHORTCUTS[text];
  if (s === undefined) return [];
  return comboKeys(typeof s === "string" ? comboOf(s) : s);
};

const tip = el("div", { class: "app-tooltip", role: "tooltip", "aria-hidden": "true" });
let owner: HTMLElement | null = null;
let timer: number | undefined;

// The text moves to data-tip, so the browser has no title left to show; a title set again later is taken over the same way.
function textOf(target: HTMLElement): string {
  const title = target.getAttribute("title");
  if (title) {
    target.dataset.tip = title;
    target.removeAttribute("title");
    // The title also named or described the element for screen readers, so that stays with an ARIA attribute.
    if (!target.hasAttribute("aria-label") && !target.hasAttribute("aria-labelledby")) target.setAttribute(target.textContent?.trim() ? "aria-description" : "aria-label", title);
  }
  return target.dataset.tip ?? "";
}

function place(target: HTMLElement): void {
  const r = target.getBoundingClientRect();
  const t = tip.getBoundingClientRect();
  const below = r.bottom + GAP + t.height <= window.innerHeight;
  const top = below ? r.bottom + GAP : r.top - GAP - t.height;
  const left = Math.min(Math.max(4, r.left + r.width / 2 - t.width / 2), window.innerWidth - t.width - 4);
  Object.assign(tip.style, { top: `${Math.max(4, top)}px`, left: `${left}px` });
}

function show(target: HTMLElement): void {
  const text = textOf(target);
  if (!text || !target.isConnected) return;
  const keys = keysFor(text);
  tip.replaceChildren(el("span", { class: "app-tooltip-text" }, text), ...(keys.length > 0 ? [el("span", { class: "app-tooltip-keys" }, ...keys.map((k) => el("kbd", {}, k)))] : []));
  tip.classList.add("shown");
  place(target);
}

function hide(): void {
  window.clearTimeout(timer);
  owner = null;
  tip.classList.remove("shown");
}

function enter(target: HTMLElement | null): void {
  if (target === owner) return;
  hide();
  if (!target) return;
  owner = target;
  textOf(target);
  timer = window.setTimeout(() => show(target), DELAY_MS);
}

const tipped = (node: EventTarget | null): HTMLElement | null => (node instanceof Element ? node.closest<HTMLElement>("[title], [data-tip]") : null);

export function installTooltips(): void {
  document.body.append(tip);
  document.addEventListener("mouseover", (ev) => enter(tipped(ev.target)));
  document.addEventListener("focusin", (ev) => {
    const t = tipped(ev.target);
    if (t?.matches(":focus-visible")) enter(t);
  });
  document.addEventListener("focusout", hide);
  document.addEventListener("pointerdown", hide, true);
  document.addEventListener("keydown", hide, true);
  document.addEventListener("scroll", hide, true);
  document.documentElement.addEventListener("mouseleave", hide);
}
