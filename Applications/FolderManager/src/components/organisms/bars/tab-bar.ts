// Strip of open tabs above the navigation bar.
import { el } from "../../dom";
import { iconButton } from "../../atoms/icon-button";
import { closeHalf, focusPane, openSplit } from "../../../hooks/split";
import { closingItem, menuIcon } from "../../molecules/menu";
import { openMenuWith } from "../menus/context-menu";
import { closeTab, duplicateTab, moveTab, openTab, selectTab, tabViews, type TabView } from "../../../hooks/tabs";

const DRAG_START_PX = 5;
// Matches the negative margin in tab-bar.css.
const TAB_OVERLAP_PX = 8;

// The focused half's title takes the accent, so the tab says which side the keyboard and toolbar act on.
const splitDot = (on: boolean, label: string): HTMLElement => el("span", { class: on ? "tab-half-label on" : "tab-half-label" }, label);

function tabMenu(ev: MouseEvent, index: number, split: boolean): void {
  ev.preventDefault();
  selectTab(index);
  openMenuWith((close) => {
    const action = closingItem(close);
    const items = [action("Split tab", !split, openSplit), action("Duplicate tab", true, () => duplicateTab(index))];
    for (const [item, icon] of [[items[0], "split"], [items[1], "duplicate"]] as const) {
      item?.classList.add("ctx-iconed");
      item?.prepend(menuIcon(icon));
    }
    return { items, subs: [], content: items };
  }, "Tab", ev.clientX, ev.clientY, () => undefined);
}

function tabItem(tab: TabView, index: number, closable: boolean, drag: (ev: PointerEvent, item: HTMLElement) => void): HTMLElement {
  const name = el("button", { type: "button", class: "tab-name", role: "tab", "aria-selected": String(tab.active), title: tab.title, "aria-label": tab.split ? `Split: ${tab.label}` : tab.label }, tab.split ? splitDot(tab.split === "left", tab.halves?.[0] ?? "") : tab.label);
  name.addEventListener("click", () => void selectTab(index).then(() => {
    if (tab.split) focusPane(false);
  }));
  const item = el("div", { class: tab.active ? "tab active" : "tab", role: "presentation" }, name);
  if (tab.split) {
    // Each half owns its dot and its ×, so either side can be shown or closed from the tab itself.
    const half = (onRight: boolean) => {
      const close = el("button", { type: "button", class: "tab-close", "aria-label": `Close ${onRight ? "right" : "left"} half`, title: `Close ${onRight ? "right" : "left"} half` }, "×");
      close.addEventListener("click", () => void selectTab(index).then(() => closeHalf(onRight)));
      return close;
    };
    const right = el("button", { type: "button", class: "tab-name tab-half", "aria-label": "Show right half", title: tab.title }, splitDot(tab.split === "right", tab.halves?.[1] ?? ""));
    right.addEventListener("click", () => void selectTab(index).then(() => focusPane(true)));
    item.append(half(false), el("span", { class: "tab-dots-bar" }), right, half(true));
  } else if (closable) {
    const close = el("button", { type: "button", class: "tab-close", "aria-label": `Close ${tab.label}`, title: "Close tab" }, "×");
    close.addEventListener("click", () => closeTab(index));
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

  const fades = (): void => {
    list.classList.toggle("fade-start", list.scrollLeft > 1);
    list.classList.toggle("fade-end", list.scrollLeft + list.clientWidth < list.scrollWidth - 1);
  };
  list.addEventListener("scroll", fades, { passive: true });
  new ResizeObserver(fades).observe(list);

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
    fades();
  }

  return { element, render };
}
