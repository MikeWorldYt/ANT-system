// Strip of open tabs above the navigation bar.
import { el } from "../../dom";
import { iconButton } from "../../atoms/icon-button";
import { closeTab, openTab, selectTab, tabViews, type TabView } from "../../../hooks/tabs";

function tabItem(tab: TabView, index: number, closable: boolean): HTMLElement {
  const name = el("button", { type: "button", class: "tab-name", role: "tab", "aria-selected": String(tab.active), title: tab.title }, tab.label);
  name.addEventListener("click", () => selectTab(index));
  const item = el("div", { class: tab.active ? "tab active" : "tab", role: "presentation" }, name);
  if (closable) {
    const close = el("button", { type: "button", class: "tab-close", "aria-label": `Close ${tab.label}`, title: "Close tab" }, "×");
    close.addEventListener("click", () => closeTab(index));
    item.append(close);
  }
  item.addEventListener("auxclick", (ev) => {
    if (ev.button === 1) closeTab(index);
  });
  return item;
}

export function createTabBar(): { element: HTMLElement; render(): void } {
  const list = el("div", { class: "tab-list", role: "tablist", "aria-label": "Open folders" });
  const element = el("div", { class: "tab-bar" }, list, iconButton("New tab", "add", () => openTab()));
  let shown = "";

  // Rebuilding only on a real change keeps keyboard focus on the tab while the store notifies.
  function render(): void {
    const tabs = tabViews();
    const next = JSON.stringify(tabs);
    if (next === shown) return;
    shown = next;
    list.replaceChildren(...tabs.map((t, i) => tabItem(t, i, tabs.length > 1)));
  }

  return { element, render };
}
