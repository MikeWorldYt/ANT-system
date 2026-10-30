// Header with the storage root name and the tree toolbar.
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";
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

export function createToolbar(views: ViewPicker, panel: PanelToggle): { element: HTMLElement; navLead: HTMLElement[]; render(): void } {
  const settingsBtn = iconButton("Settings", "settings", () => void openSettings());
  const titleRow = el("div", { class: "header-title" }, settingsBtn);

  const withSelected = (fn: (n: TreeNode) => void) => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const act = (fn: (n: TreeNode) => void) => () => withSelected(fn);

  // ─────────♦ ADD AND MOVE ─────────────────────────────────────────────────────
  const add = iconButton("Add", "add");
  attachMenu(add, "Add", () => {
    const atRoot = selectedNode()?.relParts.length === 1;
    return [
      { label: atRoot ? "Add main category" : "Add node", run: () => withSelected((n) => void addNext(n, "node")) },
      ...(!atRoot ? [{ label: "Add workboard", run: () => withSelected((n) => void addNext(n, "workboard")) }] : []),
    ];
  });
  const up = iconButton("Move up", "moveUp", act((n) => void move(n, "up")));
  const down = iconButton("Move down", "moveDown", act((n) => void move(n, "down")));

  // ─────────♦ SORT AND VIEW ────────────────────────────────────────────────────
  const SORT_KEYS: [SortBy, string][] = [["name", "Name"], ["type", "Type"], ["status", "Status"]];
  const sortLabel = el("span", {});
  const sortBtn = el("button", { type: "button", class: "view-picker sort-btn" }, guiIcon("up"), sortLabel);
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
  const picker = el("button", { type: "button", class: "view-picker view-dropdown", title: "Change view" }, guiIcon("layout"), pickerLabel, guiIcon("chevronDown"));
  attachMenu(picker, "View", () => views.modes.map((m) => ({ label: m.label, checked: m.id === views.active(), run: () => views.onSelect(m.id) })));
  // Placeholder glyphs until the dedicated view icons land; the segment shows when the toolbar has room, the dropdown otherwise.
  const VIEW_ICONS: Record<string, GuiIcon> = { tree: "layout", columns: "layout", grid: "layout" };
  const modeButtons = views.modes.map((m): [string, HTMLButtonElement] => [m.id, iconButton(`${m.label} view`, VIEW_ICONS[m.id] ?? "layout", () => views.onSelect(m.id))]);
  const segment = el("div", { class: "view-segment", role: "group", "aria-label": "View" }, ...modeButtons.map(([, b]) => b));

  // ─────────♦ REFRESH AND PANEL ────────────────────────────────────────────────
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  const panelBtn = iconButton("Details panel", "infoPanel", panel.toggle);

  reload.classList.add("toolbar-end");
  const divider = el("span", { class: "toolbar-divider", "aria-hidden": "true" });
  const orderGroup = el("div", { class: "toolbar-group" }, up, down, sortBtn);
  const bar = el("div", { class: "toolbar", role: "group", "aria-label": "Tree actions" }, orderGroup, segment, picker, reload, panelBtn);
  const element = el("header", { class: "app-header" }, titleRow, el("div", { class: "header-tools" }, bar));

  return {
    element,
    navLead: [add, divider],
    render() {
      const a = selectedNode()?.actions;
      const idle = !state.busy;
      const setDisabled = (b: HTMLButtonElement, disabled: boolean) => b.setAttribute("aria-disabled", String(disabled));
      setDisabled(add, !(a?.addNext && idle));
      setDisabled(up, !(a?.moveUp && idle));
      setDisabled(down, !(a?.moveDown && idle));
      setDisabled(reload, !idle);
      sortBtn.hidden = views.active() !== "columns";
      const { by, dir } = sortOf(activeColumnKey());
      const sortText = `Sort by ${SORT_KEYS.find(([id]) => id === by)?.[1] ?? ""}`;
      sortLabel.textContent = sortText;
      sortBtn.title = sortText;
      sortBtn.setAttribute("aria-label", `${sortText}, ${dir === "asc" ? "ascending" : "descending"}`);
      sortBtn.dataset.dir = dir;
      modeButtons.forEach(([id, b]) => b.setAttribute("aria-pressed", String(id === views.active())));
      const mode = views.modes.find((m) => m.id === views.active())?.label ?? "";
      pickerLabel.textContent = mode;
      picker.setAttribute("aria-label", `View: ${mode}`);
      panelBtn.setAttribute("aria-pressed", String(panel.visible()));
      panelBtn.title = panel.visible() ? "Hide details" : "Show details panel";
    },
  };
}
