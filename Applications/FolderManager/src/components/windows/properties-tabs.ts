// The Properties window's tabs as a real tablist: arrows, Home and End move between them, and each tab owns one tabpanel.
import { el } from "../dom";

export interface Tabs {
  list: HTMLElement;
  add(label: string, panel: HTMLElement): void;
}

export function tabs(label: string): Tabs {
  const list = el("div", { class: "prop-tabs", role: "tablist", "aria-label": label });
  const pairs: { tab: HTMLButtonElement; panel: HTMLElement }[] = [];
  const show = (at: number): void => {
    pairs.forEach(({ tab, panel }, i) => {
      tab.setAttribute("aria-selected", String(i === at));
      tab.tabIndex = i === at ? 0 : -1;
      panel.hidden = i !== at;
    });
  };
  list.addEventListener("keydown", (ev) => {
    const now = pairs.findIndex(({ tab }) => tab === document.activeElement);
    const last = pairs.length - 1;
    const to = ev.key === "ArrowRight" ? (now >= last ? 0 : now + 1) : ev.key === "ArrowLeft" ? (now <= 0 ? last : now - 1) : ev.key === "Home" ? 0 : ev.key === "End" ? last : -1;
    if (now < 0 || to < 0) return;
    ev.preventDefault();
    show(to);
    pairs[to]?.tab.focus();
  });
  return {
    list,
    add(text, panel) {
      const n = pairs.length;
      const tab = el("button", { type: "button", class: "prop-tab", role: "tab", id: `prop-tab-${n}`, "aria-controls": `prop-panel-${n}` }, text);
      panel.id = `prop-panel-${n}`;
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", tab.id);
      tab.addEventListener("click", () => show(pairs.findIndex((p) => p.tab === tab)));
      pairs.push({ tab, panel });
      list.append(tab);
      const selected = pairs.findIndex(({ tab: t }) => t.getAttribute("aria-selected") === "true");
      show(selected < 0 ? 0 : selected);
    },
  };
}
