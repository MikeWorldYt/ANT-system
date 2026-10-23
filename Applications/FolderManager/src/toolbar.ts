// Header with the storage root name and the tree toolbar.
import { el } from "./dom";
import { guiIcon, type GuiIcon } from "./gui-icons";
import { attachMenu } from "./anchored-menu";
import { addNext, move, refresh } from "./ops";
import { selectedNode, state } from "./state";
import type { TreeNode } from "./types";

function iconButton(label: string, icon: GuiIcon, onClick?: () => void): HTMLButtonElement {
  const b = el("button", { type: "button", class: "icon-btn", "aria-label": label, title: label }, guiIcon(icon));
  if (onClick) b.addEventListener("click", onClick);
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

  const withSelected = (fn: (n: TreeNode) => void) => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const act = (fn: (n: TreeNode) => void) => () => withSelected(fn);
  const add = iconButton("Add next to selection", "add");
  const down = iconButton("Move down", "moveDown", act((n) => void move(n, "down")));
  const up = iconButton("Move up", "moveUp", act((n) => void move(n, "up")));
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  const bar = el("div", { class: "toolbar", role: "toolbar", "aria-label": "Tree actions" }, add, down, up, reload);

  const pickerLabel = el("span", {});
  const picker = el("button", { type: "button", class: "view-picker", title: "Change view" }, guiIcon("layout"), pickerLabel, guiIcon("chevronDown"));
  const panelBtn = iconButton("Show details panel", "infoPanel", panel.toggle);
  attachMenu(picker, "View", () => views.modes.map((m) => ({ label: m.label, checked: m.id === views.active(), run: () => views.onSelect(m.id) })));
  attachMenu(add, "Add", () => [
    { label: "Add node", run: () => withSelected((n) => addNext(n, "node")) },
    { label: "Add Workboard", run: () => withSelected((n) => addNext(n, "workboard")) },
  ]);

  const element = el("header", { class: "app-header" }, title, el("div", { class: "header-tools" }, picker, panelBtn, bar));

  return {
    element,
    render() {
      const a = selectedNode()?.actions;
      const idle = !state.busy;
      rootName.textContent = (state.tree?.root ?? "").split(/[\\/]/).filter(Boolean).pop() ?? "";
      add.disabled = !(a?.addNext && idle);
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
