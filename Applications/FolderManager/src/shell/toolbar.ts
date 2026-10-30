// Tree actions (add, move, sort, view, refresh) that share the navigation row.
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";
import { iconButton } from "../ui/icon-button";
import { attachMenu } from "../ui/anchored-menu";
import { addNext, move, refresh } from "../actions/ops";
import { activeColumnKey, selectedNode, setColumnSort, sortOf, state, type SortBy } from "../store/state";
import type { TreeNode } from "../ipc/types";

export interface ViewPicker {
  modes: { id: string; label: string }[];
  active(): string;
  onSelect(id: string): void;
}

export function createToolbar(views: ViewPicker): { navLead: HTMLElement[]; trailing: HTMLElement[]; render(): void } {
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
  // Placeholder glyphs until the dedicated view icons land.
  const VIEW_ICONS: Record<string, GuiIcon> = { tree: "layout", columns: "layout", grid: "layout" };
  const modeButtons = views.modes.map((m): [string, HTMLButtonElement] => [m.id, iconButton(`${m.label} view`, VIEW_ICONS[m.id] ?? "layout", () => {
    views.onSelect(m.id);
    setViewOpen(false);
  })]);
  const segment = el("div", { class: "view-segment", role: "group", "aria-label": "View" }, ...modeButtons.map(([, b]) => b));
  const viewLabel = el("span", { class: "view-label" });
  const viewBtn = el("button", { type: "button", class: "view-picker", "aria-expanded": "false" }, guiIcon("layout"), viewLabel, guiIcon("chevronDown"));
  const viewControl = el("div", { class: "view-control" }, viewBtn, segment);
  const setViewOpen = (open: boolean) => {
    viewControl.classList.toggle("expanded", open);
    viewBtn.setAttribute("aria-expanded", String(open));
  };
  viewBtn.addEventListener("click", () => setViewOpen(!viewControl.classList.contains("expanded")));
  viewControl.addEventListener("keydown", (ev) => {
    if (ev.key !== "Escape" || !viewControl.classList.contains("expanded")) return;
    setViewOpen(false);
    viewBtn.focus();
  });
  viewControl.addEventListener("focusout", (ev) => {
    if (!viewControl.contains(ev.relatedTarget as Node | null)) setViewOpen(false);
  });

  // ─────────♦ REFRESH ────────────────────────────────────────────────────────────
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  reload.classList.add("toolbar-end");
  const divider = el("span", { class: "toolbar-divider", "aria-hidden": "true" });

  return {
    navLead: [add, up, down, divider],
    trailing: [viewControl, sortBtn, reload],
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
      const mode = views.modes.find((m) => m.id === views.active())?.label ?? "";
      viewLabel.textContent = mode;
      viewBtn.setAttribute("aria-label", `View: ${mode}`);
      modeButtons.forEach(([id, b]) => b.setAttribute("aria-pressed", String(id === views.active())));
    },
  };
}
