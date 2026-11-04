// Tree actions (add, sort, view, refresh) that share the navigation row.
import { el, setDisabled } from "../../dom";
import { guiIcon, type GuiIcon } from "../../atoms/gui-icons";
import { iconButton } from "../../atoms/icon-button";
import { attachMenu } from "../menus/anchored-menu";
import { EDGE, openPopup, place, popupTrigger } from "../../molecules/menu";
import { addNext, refresh } from "../../../hooks/ops";
import { activeColumnKey, selectedNode, setColumnSort, sortOf, state, type SortBy } from "../../../services/store/state";
import type { TreeNode } from "../../../services/ipc/types";

export interface ViewPicker {
  modes: { id: string; label: string }[];
  active(): string;
  onSelect(id: string): void;
}

export const SORT_KEYS: [SortBy, string][] = [["name", "Name"], ["type", "Type"], ["status", "Status"]];
const VIEW_ORDER = ["tree", "grid", "columns"];
export const orderedModes = (views: ViewPicker) => [...views.modes].sort((a, b) => VIEW_ORDER.indexOf(a.id) - VIEW_ORDER.indexOf(b.id));

export function createToolbar(views: ViewPicker): { navLead: HTMLElement[]; trailing: HTMLElement[]; render(): void } {
  const withSelected = (fn: (n: TreeNode) => void) => {
    const n = selectedNode();
    if (n) fn(n);
  };

  // ─────────♦ ADD ────────────────────────────────────────────────────────────────
  const add = iconButton("Add", "add");
  add.classList.add("nav-add");
  attachMenu(add, "Add", () => {
    const atRoot = selectedNode()?.relParts.length === 1;
    return [
      { label: atRoot ? "Add main category" : "New node", run: () => withSelected((n) => void addNext(n, "node")) },
      ...(!atRoot ? [{ label: "New workboard", run: () => withSelected((n) => void addNext(n, "workboard")) }] : []),
    ];
  });

  // ─────────♦ SORT AND VIEW ────────────────────────────────────────────────────
  const sortLabel = el("span", {});
  const sortBtn = el("button", { type: "button", class: "view-picker sort-btn" }, sortLabel, guiIcon("up"));
  attachMenu(sortBtn, "Sort by", () => {
    const column = activeColumnKey();
    const { by, dir } = sortOf(column);
    return [
      ...SORT_KEYS.map(([id, label]) => ({ label, checked: by === id, run: () => setColumnSort(column, { by: id }) })),
      { label: "Ascending", checked: dir === "asc", divider: true, run: () => setColumnSort(column, { dir: "asc" }) },
      { label: "Descending", checked: dir === "desc", run: () => setColumnSort(column, { dir: "desc" }) },
    ];
  });

  const VIEW_ICONS: Record<string, GuiIcon> = { tree: "viewTree", columns: "viewColumns", grid: "viewGrid" };
  let closeViewPopup: ((restore?: boolean) => void) | null = null;
  const modeButtons = orderedModes(views).map((m): [string, HTMLButtonElement] => [m.id, iconButton(`${m.label} view`, VIEW_ICONS[m.id] ?? "layout", () => {
    views.onSelect(m.id);
    closeViewPopup?.(true);
  })]);
  const viewLabel = el("span", { class: "view-label" });
  const viewBtn = el("button", { type: "button", class: "view-picker", "aria-haspopup": "true", "aria-expanded": "false" }, viewLabel);

  function openViewPopup(): void {
    const popup = el("div", { class: "view-segment", role: "group", "aria-label": "View" }, ...modeButtons.map(([, b]) => b));
    popup.addEventListener("keydown", (ev) => {
      if (ev.key !== "Escape") return;
      ev.preventDefault();
      closeViewPopup?.(true);
    });
    // Keeps a press on the popup from moving focus away before the click lands.
    popup.addEventListener("mousedown", (ev) => ev.preventDefault());
    const r = viewBtn.getBoundingClientRect();
    closeViewPopup = openPopup(viewBtn, popup, () => place(popup, r.left, r.bottom + EDGE), () => (closeViewPopup = null));
    modeButtons.find(([id]) => id === views.active())?.[1].focus();
  }
  popupTrigger(viewBtn, [], () => closeViewPopup, openViewPopup);

  // ─────────♦ REFRESH ────────────────────────────────────────────────────────────
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  reload.classList.add("toolbar-end");
  const divider = el("span", { class: "toolbar-divider", "aria-hidden": "true" });

  return {
    navLead: [add, divider],
    trailing: [sortBtn, viewBtn, reload],
    render() {
      const a = selectedNode()?.actions;
      const idle = !state.busy;
      setDisabled(add, !(a?.addNext && idle));
      setDisabled(reload, !idle);
      sortBtn.hidden = views.active() === "tree" || state.virtual === "board";
      const { by, dir } = sortOf(activeColumnKey());
      const sortText = `Sort by ${SORT_KEYS.find(([id]) => id === by)?.[1] ?? ""}`;
      sortLabel.textContent = sortText;
      sortBtn.title = sortText;
      sortBtn.setAttribute("aria-label", `${sortText}, ${dir === "asc" ? "ascending" : "descending"}`);
      sortBtn.dataset.dir = dir;
      const mode = views.modes.find((m) => m.id === views.active())?.label ?? "";
      viewLabel.textContent = mode;
      viewBtn.setAttribute("aria-label", `View: ${mode}`);
      viewBtn.replaceChildren(viewLabel, guiIcon(VIEW_ICONS[views.active()] ?? "layout"));
      modeButtons.forEach(([id, b]) => b.setAttribute("aria-pressed", String(id === views.active())));
    },
  };
}
