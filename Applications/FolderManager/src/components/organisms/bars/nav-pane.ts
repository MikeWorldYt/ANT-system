// The navigation pane left of the views: quick access entries above the storage tree, shown or hidden with Ctrl+Left.
import { el } from "../../dom";
import { createTreeView } from "../../windows/views/tree-view";
import { createStorageFooter } from "./storage-footer";
import { readPref, savePref } from "../../../services/store/prefs";

const PREF = "fm.navPane";
const WIDTH_PREF = "fm.navPaneWidth";
const DEFAULT_WIDTH = 260;
// Narrower than this the rows no longer read, so letting go there closes the pane instead.
const MIN_WIDTH = 150;
const KEY_STEP = 20;

export interface NavPane {
  element: HTMLElement;
  handle: HTMLElement;
  render(): void;
  toggle(): void;
}

function savedWidth(): number {
  const w = Number(readPref(WIDTH_PREF));
  return Number.isFinite(w) && w >= MIN_WIDTH ? w : DEFAULT_WIDTH;
}

const maxWidth = (): number => Math.max(MIN_WIDTH, Math.floor(window.innerWidth / 2));

export function createNavPane(): NavPane {
  const element = el("aside", { class: "nav-pane", "aria-label": "Navigation" });
  const handle = el("div", { class: "nav-resize", role: "separator", "aria-orientation": "vertical", "aria-label": "Resize navigation pane", tabindex: "0" });
  const tree = createTreeView();
  const footer = createStorageFooter();
  const scroll = el("div", { class: "nav-scroll" });
  tree.mount(scroll);
  element.append(scroll, footer.element);
  let width = savedWidth();
  element.style.width = `${width}px`;
  element.hidden = handle.hidden = readPref(PREF) === "hidden";

  // A hidden pane skips its rows, so it costs nothing while closed.
  const render = (): void => {
    if (element.hidden) return;
    tree.render();
    footer.update();
  };

  function show(on: boolean): void {
    const hadFocus = element.contains(document.activeElement) || handle === document.activeElement;
    element.hidden = handle.hidden = !on;
    savePref(PREF, on ? "shown" : "hidden");
    if (on) element.style.width = `${(width = savedWidth())}px`;
    render();
    if (on) tree.focus();
    else if (hadFocus) document.querySelector<HTMLElement>(".tree-host.focused, .tree-host")?.focus();
  }

  const toggle = (): void => show(element.hidden !== false);

  // A width too narrow to read closes the pane and keeps the last good width for its reopening.
  function settle(next: number): void {
    if (next < MIN_WIDTH) {
      element.style.width = `${width}px`;
      show(false);
      return;
    }
    width = Math.min(next, maxWidth());
    element.style.width = `${width}px`;
    savePref(WIDTH_PREF, String(width));
  }

  handle.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    ev.preventDefault();
    handle.setPointerCapture(ev.pointerId);
    const left = element.getBoundingClientRect().left;
    let next = width;
    const move = (e: PointerEvent): void => {
      next = Math.round(e.clientX - left);
      element.style.width = `${Math.max(0, Math.min(next, maxWidth()))}px`;
    };
    const up = (): void => {
      handle.removeEventListener("pointermove", move);
      settle(next);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("lostpointercapture", up, { once: true });
  });

  handle.addEventListener("keydown", (ev) => {
    if (ev.key !== "ArrowLeft" && ev.key !== "ArrowRight") return;
    ev.preventDefault();
    ev.stopPropagation();
    settle(width + (ev.key === "ArrowRight" ? KEY_STEP : -KEY_STEP));
  });

  return { element, handle, render, toggle };
}
