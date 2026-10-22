// Header with the storage root name and the tree toolbar.
import { el } from "./dom";
import { addChild, move, refresh } from "./ops";
import { selectedNode, state } from "./state";
import type { TreeNode } from "./types";

function iconButton(label: string, glyph: string, onClick: () => void): HTMLButtonElement {
  const b = el("button", { type: "button", class: "icon-btn", "aria-label": label, title: label }, glyph);
  b.addEventListener("click", onClick);
  return b;
}

export interface ViewToggle {
  modes: { id: string; label: string }[];
  active(): string;
  onSelect(id: string): void;
}

export function createToolbar(views: ViewToggle): { element: HTMLElement; render(): void } {
  const title = el("h1", {}, "Storage root name: ");
  const rootName = el("span", { class: "root-name" });
  title.append(rootName);

  const act = (fn: (n: TreeNode) => void) => () => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const add = iconButton("Add child", "+", act((n) => void addChild(n)));
  const down = iconButton("Move down", "▼", act((n) => void move(n, "down")));
  const up = iconButton("Move up", "▲", act((n) => void move(n, "up")));
  const reload = iconButton("Refresh", "↺", () => void refresh());
  const bar = el("div", { class: "toolbar", role: "toolbar", "aria-label": "Tree actions" }, add, down, up, reload);
  const toggles = views.modes.map((m) => {
    const b = el("button", { type: "button", class: "seg-btn" }, m.label);
    b.addEventListener("click", () => views.onSelect(m.id));
    return [m.id, b] as const;
  });
  const toggle = el("div", { class: "view-toggle", role: "group", "aria-label": "View mode" }, ...toggles.map(([, b]) => b));
  const element = el("header", { class: "app-header" }, title, el("div", { class: "header-tools" }, toggle, bar));

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
      for (const [id, b] of toggles) b.setAttribute("aria-pressed", String(id === views.active()));
    },
  };
}
