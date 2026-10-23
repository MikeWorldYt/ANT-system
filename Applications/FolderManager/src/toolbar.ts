// Header with the storage root name and the tree toolbar.
import { el } from "./dom";
import { guiIcon, type GuiIcon } from "./gui-icons";
import { addChild, move, refresh } from "./ops";
import { selectedNode, state } from "./state";
import type { TreeNode } from "./types";

function iconButton(label: string, icon: GuiIcon, onClick: () => void): HTMLButtonElement {
  const b = el("button", { type: "button", class: "icon-btn", "aria-label": label, title: label }, guiIcon(icon));
  b.addEventListener("click", onClick);
  return b;
}

export interface ViewPicker {
  modes: { id: string; label: string }[];
  active(): string;
  onSelect(id: string): void;
}

export interface PanelToggle {
  visible(): boolean;
  toggle(): void;
}

export function createToolbar(views: ViewPicker, panel: PanelToggle): { element: HTMLElement; render(): void } {
  const title = el("h1", {}, "Storage root name: ");
  const rootName = el("span", { class: "root-name" });
  title.append(rootName);

  const act = (fn: (n: TreeNode) => void) => () => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const add = iconButton("Add child", "add", act((n) => void addChild(n)));
  const down = iconButton("Move down", "moveDown", act((n) => void move(n, "down")));
  const up = iconButton("Move up", "moveUp", act((n) => void move(n, "up")));
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  const bar = el("div", { class: "toolbar", role: "toolbar", "aria-label": "Tree actions" }, add, down, up, reload);

  const pickerLabel = el("span", {});
  const picker = el("button", { type: "button", class: "view-picker", "aria-haspopup": "menu", "aria-expanded": "false", title: "Change view" }, guiIcon("layout"), pickerLabel, guiIcon("chevronDown"));
  const panelBtn = iconButton("Show details panel", "infoPanel", panel.toggle);
  let closeMenu: (() => void) | null = null;

  function openMenu(): void {
    const items = views.modes.map((m) => {
      const b = el("button", { type: "button", role: "menuitemradio", tabindex: "-1", class: "ctx-item" }, m.label);
      b.setAttribute("aria-checked", String(m.id === views.active()));
      b.addEventListener("click", () => {
        close();
        views.onSelect(m.id);
      });
      return b;
    });
    const menu = el("div", { class: "ctx-menu", role: "menu", "aria-label": "View" }, el("div", { class: "ctx-group", role: "group" }, ...items));
    const outside = (ev: Event) => {
      if (!menu.contains(ev.target as Node) && ev.target !== picker && !picker.contains(ev.target as Node)) close();
    };
    function close(): void {
      menu.remove();
      document.removeEventListener("pointerdown", outside, true);
      picker.setAttribute("aria-expanded", "false");
      closeMenu = null;
      picker.focus();
    }
    const move = (to: number) => items[(to + items.length) % items.length]?.focus();
    menu.addEventListener("keydown", (ev) => {
      const i = items.indexOf(document.activeElement as HTMLButtonElement);
      if (ev.key === "ArrowDown") move(i + 1);
      else if (ev.key === "ArrowUp") move(i - 1);
      else if (ev.key === "Escape") close();
      else return;
      ev.preventDefault();
    });
    const r = picker.getBoundingClientRect();
    menu.style.left = `${r.left}px`;
    menu.style.top = `${r.bottom + 4}px`;
    document.body.append(menu);
    document.addEventListener("pointerdown", outside, true);
    picker.setAttribute("aria-expanded", "true");
    closeMenu = close;
    (items.find((b) => b.getAttribute("aria-checked") === "true") ?? items[0])?.focus();
  }
  picker.addEventListener("click", () => (closeMenu ? closeMenu() : openMenu()));

  const element = el("header", { class: "app-header" }, title, el("div", { class: "header-tools" }, picker, panelBtn, bar));

  return {
    element,
    render() {
      const a = selectedNode()?.actions;
      const idle = !state.busy;
      rootName.textContent = (state.tree?.root ?? "").split(/[\\/]/).filter(Boolean).pop() ?? "";
      add.disabled = !(a?.addChild && idle);
      down.disabled = !(a?.moveDown && idle);
      up.disabled = !(a?.moveUp && idle);
      reload.disabled = !idle;
      pickerLabel.textContent = views.modes.find((m) => m.id === views.active())?.label ?? "";
      panelBtn.setAttribute("aria-pressed", String(panel.visible()));
      panelBtn.title = panel.visible() ? "Hide details panel" : "Show details panel";
      panelBtn.setAttribute("aria-label", panelBtn.title);
    },
  };
}
