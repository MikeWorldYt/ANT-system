// Header with the storage root name and the tree toolbar.
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { iconButton } from "../ui/icon-button";
import { attachMenu } from "../ui/anchored-menu";
import { openSettings } from "./settings-dialog";
import { addNext, move, refresh } from "../actions/ops";
import { activeColumnKey, selectedNode, setColumnSort, sortOf, state, type SortBy } from "../store/state";
import type { TreeNode } from "../ipc/types";

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
  const settingsBtn = iconButton("Settings", "settings", () => void openSettings());
  const titleRow = el("div", { class: "header-title" }, settingsBtn);

  const withSelected = (fn: (n: TreeNode) => void) => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const act = (fn: (n: TreeNode) => void) => () => withSelected(fn);

  // ─────────♦ ADD AND MOVE ─────────────────────────────────────────────────────
  const add = iconButton("Add", "add");
  attachMenu(add, "Add", () => [
    { label: "Add node", run: () => withSelected((n) => addNext(n, "node")) },
    { label: "Add workboard", run: () => withSelected((n) => addNext(n, "workboard")) },
  ]);
  const up = iconButton("Move up", "moveUp", act((n) => void move(n, "up")));
  const down = iconButton("Move down", "moveDown", act((n) => void move(n, "down")));

  // ─────────♦ SORT AND VIEW ────────────────────────────────────────────────────
  const sortBtn = el("button", { type: "button", class: "view-picker", title: "Sort by", "aria-label": "Sort by" }, guiIcon("up"), guiIcon("chevronDown"));
  const SORT_KEYS: [SortBy, string][] = [["name", "Name"], ["type", "Type"], ["status", "Status"]];
  attachMenu(sortBtn, "Sort by", () => {
    const column = activeColumnKey();
    const { by, dir } = sortOf(column);
    return [
      ...SORT_KEYS.map(([id, label]) => ({ label, checked: by === id, run: () => setColumnSort(column, { by: id }) })),
      { label: "Ascending", checked: dir === "asc", divider: true, run: () => setColumnSort(column, { dir: "asc" }) },
      { label: "Descending", checked: dir === "desc", run: () => setColumnSort(column, { dir: "desc" }) },
    ];
  });
  const pickerLabel = el("span", { class: "view-label" });
  const picker = el("button", { type: "button", class: "view-picker", title: "Change view" }, guiIcon("layout"), pickerLabel, guiIcon("chevronDown"));
  attachMenu(picker, "View", () => views.modes.map((m) => ({ label: m.label, checked: m.id === views.active(), run: () => views.onSelect(m.id) })));

  // ─────────♦ REFRESH AND PANEL ────────────────────────────────────────────────
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  const panelBtn = iconButton("Details panel", "infoPanel", panel.toggle);

  const bar = el("div", { class: "toolbar", role: "group", "aria-label": "Tree actions" }, add, up, down, sortBtn, picker, reload, panelBtn);
  const element = el("header", { class: "app-header" }, titleRow, el("div", { class: "header-tools" }, bar));

  return {
    element,
    render() {
      const a = selectedNode()?.actions;
      const idle = !state.busy;
      const setDisabled = (b: HTMLButtonElement, disabled: boolean) => b.setAttribute("aria-disabled", String(disabled));
      setDisabled(add, !(a?.addNext && idle));
      setDisabled(up, !(a?.moveUp && idle));
      setDisabled(down, !(a?.moveDown && idle));
      setDisabled(reload, !idle);
      sortBtn.hidden = views.active() !== "columns";
      const mode = views.modes.find((m) => m.id === views.active())?.label ?? "";
      pickerLabel.textContent = mode;
      picker.setAttribute("aria-label", `View: ${mode}`);
      panelBtn.setAttribute("aria-pressed", String(panel.visible()));
      panelBtn.title = panel.visible() ? "Hide details" : "Show details panel";
    },
  };
}
