// Header with the storage root name and the tree toolbar.
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";
import { attachMenu } from "../ui/anchored-menu";
import { openSettings } from "./settings-dialog";
import { addNext, move, refresh } from "../actions/ops";
import { activeColumnKey, selectedNode, setColumnSort, sortOf, state, type SortBy } from "../store/state";
import type { TreeNode } from "../ipc/types";

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
  const settingsBtn = iconButton("Settings", "settings", () => void openSettings());
  title.append(rootName, settingsBtn);

  const withSelected = (fn: (n: TreeNode) => void) => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const act = (fn: (n: TreeNode) => void) => () => withSelected(fn);

  // ─────────♦ ADD AND MOVE ─────────────────────────────────────────────────────
  const add = iconButton("Add", "add");
  attachMenu(add, "Add", () => [
    { label: "Add node", run: () => withSelected((n) => addNext(n, "node")) },
    { label: "Add Workboard", run: () => withSelected((n) => addNext(n, "workboard")) },
  ]);
  const down = iconButton("Move down", "moveDown", act((n) => void move(n, "down")));
  const up = iconButton("Move up", "moveUp", act((n) => void move(n, "up")));

  // ─────────♦ SORT AND VIEW ────────────────────────────────────────────────────
  const sortBtn = el("button", { type: "button", class: "view-picker", title: "Sort by" }, guiIcon("up"), guiIcon("chevronDown"));
  const SORT_KEYS: [SortBy, string][] = [["name", "Name"], ["type", "Type"], ["status", "Status"]];
  attachMenu(sortBtn, "Sort this column", () => {
    const column = activeColumnKey();
    const { by, dir } = sortOf(column);
    return [
      ...SORT_KEYS.map(([id, label]) => ({ label, checked: by === id, run: () => setColumnSort(column, { by: id }) })),
      { label: "Ascending", checked: dir === "asc", divider: true, run: () => setColumnSort(column, { dir: "asc" }) },
      { label: "Descending", checked: dir === "desc", run: () => setColumnSort(column, { dir: "desc" }) },
    ];
  });
  const pickerLabel = el("span", {});
  const picker = el("button", { type: "button", class: "view-picker", title: "Change view" }, guiIcon("layout"), pickerLabel, guiIcon("chevronDown"));
  attachMenu(picker, "View", () => views.modes.map((m) => ({ label: m.label, checked: m.id === views.active(), run: () => views.onSelect(m.id) })));

  // ─────────♦ REFRESH AND PANEL ────────────────────────────────────────────────
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  const panelBtn = iconButton("Show panel", "infoPanel", panel.toggle);

  const bar = el("div", { class: "toolbar", role: "toolbar", "aria-label": "Tree actions" }, add, down, up, sortBtn, picker, reload, panelBtn);
  const element = el("header", { class: "app-header" }, title, el("div", { class: "header-tools" }, bar));

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
      sortBtn.hidden = views.active() !== "columns";
      pickerLabel.textContent = views.modes.find((m) => m.id === views.active())?.label ?? "";
      panelBtn.setAttribute("aria-pressed", String(panel.visible()));
      panelBtn.title = panel.visible() ? "Hide details" : "Show details panel";
      panelBtn.setAttribute("aria-label", panelBtn.title);
    },
  };
}
