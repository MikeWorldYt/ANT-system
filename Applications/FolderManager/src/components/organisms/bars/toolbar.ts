// Tree actions (add, sort, view, refresh) that share the navigation row.
import { VIEW_ICONS, viewChoice } from "../../molecules/view-choice";
import { el, setDisabled } from "../../dom";
import { guiIcon, type GuiIcon } from "../../atoms/gui-icons";
import { iconButton } from "../../atoms/icon-button";
import { BATCH_VIEWS, batch } from "../../../hooks/automations/batch";
import { attachMenu } from "../menus/anchored-menu";
import { EDGE, openPopup, place, popupTrigger } from "../../molecules/menu";
import { addChild, addNext, refresh } from "../../../hooks/ops";
import { activeColumnKey, selectedNode, setColumnSort, sortOf, state, type SortBy } from "../../../services/store/state";
import type { TreeNode } from "../../../services/ipc/types";

export interface ViewPicker {
  modes: { id: string; label: string }[];
  active(): string;
  folder(): TreeNode | null | undefined;
  onSelect(id: string): void;
}

export const SORT_KEYS: [SortBy, string, GuiIcon][] = [["track", "Track", "sortTrack"], ["name", "Name", "sortName"], ["status", "Status", "sortStatus"], ["created", "Date created", "sortCreated"], ["modified", "Date modified", "sortModified"], ["size", "Size", "sortSize"]];
const VIEW_ORDER = ["list", "details", "cards", "grid", "columns", "graph"];
export const orderedModes = (views: ViewPicker) => [...views.modes].sort((a, b) => VIEW_ORDER.indexOf(a.id) - VIEW_ORDER.indexOf(b.id));

export function createToolbar(views: ViewPicker): { navLead: HTMLElement[]; trailing: HTMLElement[]; render(): void } {
  // ─────────♦ ADD ────────────────────────────────────────────────────────────────
  // Folder views add inside the folder on screen; tree and columns add next to the selected row.
  function addItems(): { label: string; run: () => void }[] {
    const shown = state.virtual === null ? views.folder() : undefined;
    if (shown === null) {
      const anchor = state.tree?.nodes.find((n) => n.actions.addNext);
      return anchor ? [{ label: "New main category", run: () => void addNext(anchor, "node") }] : [];
    }
    if (shown) {
      const { addChild: canNode, addFolder } = shown.actions;
      if (!canNode && !addFolder) return [];
      return [
        { label: `New ${addFolder ? "folder" : "node"} in "${shown.name}"`, run: () => void addChild(shown, "node") },
        ...(shown.relParts.length > 1 ? [{ label: `New worknode in "${shown.name}"`, run: () => void addChild(shown, "worknode") }] : []),
      ];
    }
    const n = selectedNode();
    if (!n?.actions.addNext) return [];
    if (n.relParts.length === 1) return [{ label: "New main category", run: () => void addNext(n, "node") }];
    const parent = n.relParts[n.relParts.length - 2];
    return [
      { label: `New node in "${parent}"`, run: () => void addNext(n, "node") },
      { label: `New worknode in "${parent}"`, run: () => void addNext(n, "worknode") },
    ];
  }
  const add = iconButton("Add", "add");
  add.classList.add("nav-add");
  attachMenu(add, "Add", addItems);

  // ─────────♦ SORT AND VIEW ────────────────────────────────────────────────────
  const sortLabel = el("span", {});
  const sortDir = guiIcon("up");
  sortDir.classList.add("sort-dir");
  const caret = (): SVGElement => {
    const icon = guiIcon("chevronDown");
    icon.classList.add("picker-caret");
    return icon;
  };
  const sortBtn = el("button", { type: "button", class: "view-picker sort-btn" }, sortLabel, sortDir, caret());
  attachMenu(sortBtn, "Sort by", () => {
    const column = activeColumnKey();
    const { by, dir } = sortOf(column);
    return [
      ...SORT_KEYS.map(([id, label, icon]) => ({ label, icon, checked: by === id, run: () => setColumnSort(column, { by: id }) })),
      { label: "Ascending", icon: "up", checked: dir === "asc", divider: true, run: () => setColumnSort(column, { dir: "asc" }) },
      { label: "Descending", icon: "up", flip: true, checked: dir === "desc", run: () => setColumnSort(column, { dir: "desc" }) },
    ];
  });

  let closeViewPopup: ((restore?: boolean) => void) | null = null;
  const modeButtons = orderedModes(views).map((m): [string, HTMLButtonElement] => [m.id, viewChoice(m.label, VIEW_ICONS[m.id] ?? "layout", () => {
    if (batch !== null && !BATCH_VIEWS.includes(m.id)) return;
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
      const idle = !state.busy;
      setDisabled(add, !(addItems().length > 0 && idle));
      setDisabled(reload, !idle);
      sortBtn.hidden = state.virtual === "board";
      const { by, dir } = sortOf(activeColumnKey());
      const sortText = `Sort by ${SORT_KEYS.find(([id]) => id === by)?.[1] ?? ""}`;
      sortLabel.textContent = sortText;
      sortBtn.title = sortText;
      sortBtn.setAttribute("aria-label", `${sortText}, ${dir === "asc" ? "ascending" : "descending"}`);
      sortBtn.dataset.dir = dir;
      const mode = views.modes.find((m) => m.id === views.active())?.label ?? "";
      viewLabel.textContent = mode;
      viewBtn.setAttribute("aria-label", `View: ${mode}`);
      viewBtn.replaceChildren(viewLabel, guiIcon(VIEW_ICONS[views.active()] ?? "layout"), caret());
      modeButtons.forEach(([id, b]) => {
        b.setAttribute("aria-pressed", String(id === views.active()));
        setDisabled(b, batch !== null && !BATCH_VIEWS.includes(id));
      });
    },
  };
}
