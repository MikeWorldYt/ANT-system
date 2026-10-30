// Tree actions (add, sort, view, refresh) that share the navigation row.
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";
import { iconButton } from "../ui/icon-button";
import { attachMenu } from "../ui/anchored-menu";
import { EDGE, autoDismiss, place } from "../ui/menu";
import { addNext, refresh } from "../actions/ops";
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

  // ─────────♦ ADD ────────────────────────────────────────────────────────────────
  const add = iconButton("Add", "add");
  attachMenu(add, "Add", () => {
    const atRoot = selectedNode()?.relParts.length === 1;
    return [
      { label: atRoot ? "Add main category" : "Add node", run: () => withSelected((n) => void addNext(n, "node")) },
      ...(!atRoot ? [{ label: "Add workboard", run: () => withSelected((n) => void addNext(n, "workboard")) }] : []),
    ];
  });

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
  let closeViewPopup: ((restore: boolean) => void) | null = null;
  const modeButtons = views.modes.map((m): [string, HTMLButtonElement] => [m.id, iconButton(`${m.label} view`, VIEW_ICONS[m.id] ?? "layout", () => {
    views.onSelect(m.id);
    closeViewPopup?.(true);
  })]);
  const viewLabel = el("span", { class: "view-label" });
  const viewBtn = el("button", { type: "button", class: "view-picker", "aria-haspopup": "true", "aria-expanded": "false" }, guiIcon("layout"), viewLabel, guiIcon("chevronDown"));

  function openViewPopup(): void {
    const popup = el("div", { class: "view-segment", role: "group", "aria-label": "View" }, ...modeButtons.map(([, b]) => b));
    const close = (restore: boolean) => {
      closeViewPopup = null;
      popup.remove();
      stopWatching();
      viewBtn.setAttribute("aria-expanded", "false");
      if (restore) viewBtn.focus();
    };
    popup.addEventListener("keydown", (ev) => {
      if (ev.key !== "Escape") return;
      ev.preventDefault();
      close(true);
    });
    // Keeps a press on the popup from moving focus away before the click lands.
    popup.addEventListener("mousedown", (ev) => ev.preventDefault());
    const r = viewBtn.getBoundingClientRect();
    document.body.append(popup);
    place(popup, r.left, r.bottom + EDGE);
    const stopWatching = autoDismiss((t) => popup.contains(t) || viewBtn.contains(t), close);
    viewBtn.setAttribute("aria-expanded", "true");
    closeViewPopup = close;
    modeButtons.find(([id]) => id === views.active())?.[1].focus();
  }
  viewBtn.addEventListener("click", () => (closeViewPopup ? closeViewPopup(true) : openViewPopup()));

  // ─────────♦ REFRESH ────────────────────────────────────────────────────────────
  const reload = iconButton("Refresh", "refresh", () => void refresh());
  reload.classList.add("toolbar-end");
  const divider = el("span", { class: "toolbar-divider", "aria-hidden": "true" });

  return {
    navLead: [add, divider],
    trailing: [viewBtn, sortBtn, reload],
    render() {
      const a = selectedNode()?.actions;
      const idle = !state.busy;
      const setDisabled = (b: HTMLButtonElement, disabled: boolean) => b.setAttribute("aria-disabled", String(disabled));
      setDisabled(add, !(a?.addNext && idle));
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
