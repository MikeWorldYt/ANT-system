// Strip of open tabs above the navigation bar.
import { el } from "../../dom";
import { iconButton } from "../../atoms/icon-button";
import { closeSplit, openSplit } from "../../../hooks/split";
import { closingItem } from "../../molecules/menu";
import { openMenuWith } from "../menus/context-menu";
import { closeTab, duplicateTab, moveTab, openTab, selectTab, tabViews, type TabView } from "../../../hooks/tabs";

const DRAG_START_PX = 5;
// Matches the negative margin in tab-bar.css.
const TAB_OVERLAP_PX = 8;

function splitDots(focused: "left" | "right"): HTMLElement {
  const dot = (on: boolean) => el("span", { class: on ? "tab-dot on" : "tab-dot" });
  return el("span", { class: "tab-dots" }, dot(focused === "left"), el("span", { class: "tab-dots-bar" }), dot(focused === "right"));
}

function tabMenu(ev: MouseEvent, index: number, split: boolean): void {
  ev.preventDefault();
  selectTab(index);
  openMenuWith((close) => {
    const action = closingItem(close);
    const items = [action("Split tab", !split, openSplit), action("Duplicate tab", true, () => duplicateTab(index))];
    return { items, subs: [], content: items };
  }, "Tab", ev.clientX, ev.clientY, () => undefined);
}

function tabItem(tab: TabView, index: number, closable: boolean, drag: (ev: PointerEvent, item: HTMLElement) => void): HTMLElement {
  const name = el("button", { type: "button", class: "tab-name", role: "tab", "aria-selected": String(tab.active), title: tab.title, "aria-label": tab.split ? `Split: ${tab.label}` : tab.label }, tab.split ? splitDots(tab.split) : tab.label);
  name.addEventListener("click", () => selectTab(index));
  const item = el("div", { class: tab.active ? "tab active" : "tab", role: "presentation" }, name);
  if (closable || tab.split) {
    const closeLabel = tab.split ? "Close split" : "Close tab";
    const close = el("button", { type: "button", class: "tab-close", "aria-label": tab.split ? closeLabel : `Close ${tab.label}`, title: closeLabel }, "×");
    close.addEventListener("click", () => (tab.split ? closeSplit() : closeTab(index)));
    item.append(close);
  }
  item.addEventListener("pointerdown", (ev) => {
    if (ev.button === 0 && !(ev.target instanceof Element && ev.target.closest(".tab-close"))) drag(ev, item);
  });
  item.addEventListener("contextmenu", (ev) => tabMenu(ev, index, tab.split !== null));
  item.addEventListener("auxclick", (ev) => {
    if (ev.button === 1) closeTab(index);
  });
  return item;
}

export function createTabBar(): { element: HTMLElement; render(): void } {
  const list = el("div", { class: "tab-list", role: "tablist", "aria-label": "Open folders" });
  const element = el("div", { class: "tab-bar" }, list, iconButton("New tab", "add", () => openTab()));
  let shown = "";

  // Pointer events, because the window's native file drop swallows HTML drag events. Transforms show the new order; the store changes on release.
  function drag(down: PointerEvent, item: HTMLElement): void {
    const items = [...list.children] as HTMLElement[];
    const from = items.indexOf(item);
    const centers = items.map((t) => t.getBoundingClientRect()).map((r) => r.left + r.width / 2);
    const start = item.getBoundingClientRect();
    const step = start.width - TAB_OVERLAP_PX;
    let to = from;
    let dragging = false;
    const move = (ev: PointerEvent) => {
      const moved = ev.clientX - down.clientX;
      if (!dragging && Math.abs(moved) < DRAG_START_PX) return;
      dragging = true;
      item.classList.add("dragging");
      list.classList.add("reordering");
      const bar = list.getBoundingClientRect();
      const x = Math.min(Math.max(moved, bar.left - start.left), bar.right - start.right);
      item.style.transform = `translateX(${x}px)`;
      const center = start.left + start.width / 2 + x;
      to = centers.filter((c, i) => i !== from && c < center).length;
      items.forEach((t, i) => {
        if (t === item) return;
        const shift = i > from && i <= to ? -step : i >= to && i < from ? step : 0;
        t.style.transform = shift === 0 ? "" : `translateX(${shift}px)`;
      });
    };
    const end = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      if (!dragging) return;
      list.classList.remove("reordering");
      // The release must not count as a click on the tab beneath it.
      const swallow = (ev: Event) => ev.stopPropagation();
      list.addEventListener("click", swallow, { capture: true, once: true });
      setTimeout(() => list.removeEventListener("click", swallow, { capture: true }));
      moveTab(from, to);
      shown = "";
      render();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  }

  // Rebuilding only on a real change keeps keyboard focus on the tab while the store notifies.
  function render(): void {
    const tabs = tabViews();
    const next = JSON.stringify(tabs);
    if (next === shown) return;
    shown = next;
    list.replaceChildren(...tabs.map((t, i) => tabItem(t, i, tabs.length > 1, drag)));
    list.querySelector(".tab.active")?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }

  return { element, render };
}
