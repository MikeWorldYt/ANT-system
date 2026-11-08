// A tab's second pane: the store only holds the focused pane, so the other one is parked here and swapped in to render.
import { el } from "../components/dom";
import { reportError } from "../components/organisms/bars/status-line";
import { listChildren } from "../services/ipc/api";
import { findNode, notify, state } from "../services/store/state";
import { virtualKeys } from "../services/store/virtual";
import type { ViewSwitcher } from "./view-switch";
import type { TabNavigator, Trail } from "./tabs";

type Parked = Pick<typeof state, "selectedKey" | "selectedKeys" | "expanded" | "virtual" | "virtualKeys" | "via" | "pasteKey" | "editing">;

export interface Split {
  other: Parked;
  trail: Trail;
  view: string;
  right: boolean;
  // Width of the left pane, in percent.
  ratio: number;
}

export interface SavedSplit {
  key: string | null;
  view: string;
  right: boolean;
  ratio: number;
}

interface Pane {
  host: HTMLElement;
  views: ViewSwitcher;
}

type PaneNavigator = Pick<TabNavigator, "trail" | "setTrail">;

const read = (): Parked => {
  const { selectedKey, selectedKeys, expanded, virtual, virtualKeys, via, pasteKey, editing } = state;
  return { selectedKey, selectedKeys, expanded, virtual, virtualKeys, via, pasteKey, editing };
};

let left: Pane | null = null;
let right: Pane | null = null;
let navigator: PaneNavigator | null = null;
let split: Split | null = null;
let renderingOther = false;

const MIN_RATIO = 15;
const KEY_STEP = 5;
const divider = el("div", { class: "pane-divider", role: "separator", "aria-orientation": "vertical", "aria-label": "Resize split", tabindex: "0" });

const focusedPane = () => (split?.right ? right : left);
const otherPane = () => (split?.right ? left : right);

function setRatio(value: number): void {
  if (!split) return;
  split.ratio = Math.min(Math.max(value, MIN_RATIO), 100 - MIN_RATIO);
  showPanes();
}

function dragDivider(down: PointerEvent): void {
  const bar = divider.parentElement?.getBoundingClientRect();
  if (!bar) return;
  down.preventDefault();
  const move = (ev: PointerEvent) => setRatio(((ev.clientX - bar.left) / bar.width) * 100);
  const end = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", end);
    window.removeEventListener("pointercancel", end);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", end);
  window.addEventListener("pointercancel", end);
}

function showPanes(): void {
  if (right) right.host.hidden = split === null;
  divider.hidden = split === null;
  divider.setAttribute("aria-valuenow", String(Math.round(split?.ratio ?? 50)));
  if (left) left.host.style.flex = split ? `0 0 calc(${split.ratio}% - 5px)` : "";
  left?.host.classList.toggle("focused", split !== null && !split.right);
  right?.host.classList.toggle("focused", split !== null && split.right);
}

export function bindPanes(a: Pane, b: Pane, nav: PaneNavigator): void {
  left = a;
  right = b;
  navigator = nav;
  a.host.after(divider);
  divider.addEventListener("pointerdown", dragDivider);
  divider.addEventListener("keydown", (ev) => {
    if (ev.key === "ArrowLeft") setRatio((split?.ratio ?? 50) - KEY_STEP);
    if (ev.key === "ArrowRight") setRatio((split?.ratio ?? 50) + KEY_STEP);
  });
  a.host.addEventListener("pointerdown", () => focusPane(false), { capture: true });
  b.host.addEventListener("pointerdown", () => focusPane(true), { capture: true });
  showPanes();
}

export function focusedViews(modes: ViewSwitcher["modes"]): ViewSwitcher {
  return {
    modes,
    current: () => {
      const views = focusedPane()?.views;
      if (!views) throw new Error("Panes are not bound");
      return views.current();
    },
    switchTo: (id) => focusedPane()?.views.switchTo(id),
  };
}

export const currentSplit = () => split;
export const isRenderingOther = () => renderingOther;

// A change of tab brings that tab's panes, whose views must be back before it renders.
export function restoreSplit(next: Split | null): void {
  split = next;
  if (next) otherPane()?.views.switchTo(next.view);
  showPanes();
  void loadOther();
}

export const savedSplit = (s: Split | null): SavedSplit | null => s && { key: s.other.selectedKey, view: s.view, right: s.right, ratio: s.ratio };

export function parkedSplit({ key, view, right: onRight, ratio }: SavedSplit): Split {
  const parts = key?.split("/") ?? [];
  const expanded = new Set(parts.map((_, i) => parts.slice(0, i + 1).join("/")));
  return { other: { selectedKey: key, selectedKeys: key === null ? [] : [key], expanded, virtual: null, virtualKeys: [], via: null, pasteKey: null, editing: null }, trail: { history: [], at: -1 }, view, right: onRight, ratio };
}

// The parked pane may stand deeper than anything the focused pane loaded.
export async function loadOther(): Promise<void> {
  const parts = split?.other.selectedKey?.split("/") ?? [];
  let loaded = false;
  try {
    for (let i = 1; i <= parts.length; i++) {
      const n = findNode(parts.slice(0, i));
      if (!n || n.children !== null || !n.hasChildren) continue;
      n.children = await listChildren(n.relParts);
      loaded = true;
    }
  } catch (e) {
    reportError("warning", e);
  }
  if (loaded) notify();
}

// Read without swapping the parked state in, as the user may act on the focused pane while the list loads.
export async function loadOtherVirtual(): Promise<void> {
  const id = split?.other.virtual ?? null;
  if (id === null) return;
  const keys = await virtualKeys(id);
  if (split?.other.virtual === id) split.other.virtualKeys = keys;
}

export function openSplit(): void {
  if (split || !focusedPane()) return;
  const here = read();
  const view = focusedPane()?.views.current().id ?? "";
  split = { other: { ...here, expanded: new Set(here.expanded), editing: null }, trail: { history: [], at: -1 }, view, right: false, ratio: 50 };
  otherPane()?.views.switchTo(view);
  showPanes();
  notify();
}

export function focusPane(toRight: boolean): void {
  if (!split || !navigator || split.right === toRight) return;
  const here = read();
  const trail = navigator.trail();
  const view = focusedPane()?.views.current().id ?? split.view;
  Object.assign(state, split.other);
  navigator.setTrail(split.trail);
  split.other = { ...here, editing: null };
  split.trail = trail;
  split.view = view;
  split.right = toRight;
  showPanes();
  notify();
}

export function closeSplit(): void {
  focusPane(false);
  split = null;
  showPanes();
  notify();
}

export function renderOther(): void {
  if (!split) return;
  const here = read();
  renderingOther = true;
  Object.assign(state, split.other);
  try {
    otherPane()?.views.current().render();
  } finally {
    split.other = read();
    Object.assign(state, here);
    renderingOther = false;
  }
}
