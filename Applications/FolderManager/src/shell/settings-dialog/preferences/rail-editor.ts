// Editor for the collapsed side panel: which rail buttons show and in what order; HTML5 drag events never reach the webview.
import { LEFT_BUTTON, pastThreshold, type Point } from "../../../actions/drag-out";
import { el } from "../../../ui/dom";
import { guiIcon, type GuiIcon } from "../../../ui/gui-icons";
import { railLayout, setRailLayout, type RailId, type RailItem } from "../../../store/side-layout";
import { switchControl } from "../page";

const ROWS: Record<RailId, { label: string; icon: GuiIcon }> = {
  settings: { label: "Settings", icon: "settings" },
  favorite: { label: "Favorite", icon: "starred" },
  edit: { label: "Edit details", icon: "pencil" },
  moveUp: { label: "Move up", icon: "moveUp" },
  moveDown: { label: "Move down", icon: "moveDown" },
  delete: { label: "Delete", icon: "trash" },
};

const LIFTED = "is-lifted";
const SORTING = "is-sorting";
const DROP_BEFORE = "drop-before";
const DROP_AFTER = "drop-after";

interface Drag {
  row: HTMLElement;
  from: number;
  start: Point;
  lifted: boolean;
  slot: number | null;
}

export function railEditor(): HTMLElement {
  const list = el("ul", { class: "sw-rail-list", "aria-label": "Collapsed side panel buttons" });
  const status = el("div", { class: "sr-only", role: "status" });
  let drag: Drag | null = null;

  const rows = (): HTMLElement[] => Array.from(list.children) as HTMLElement[];
  const rowOf = (target: EventTarget | null): HTMLElement | null =>
    target instanceof Element ? target.closest<HTMLElement>("li[data-id]") : null;

  function reorder(from: number, to: number): void {
    const items = [...railLayout()];
    const [item] = items.splice(from, 1);
    if (!item || to === from || to < 0 || to > items.length) return;
    items.splice(to, 0, item);
    setRailLayout(items);
    render();
    // The rows are rebuilt, so focus follows the moved item to its new row.
    list.querySelector<HTMLElement>(`[data-id="${item.id}"]`)?.focus();
    status.textContent = `${ROWS[item.id].label} moved to position ${to + 1} of ${items.length}.`;
  }

  function toggle(id: RailId, shown: boolean): Promise<boolean> {
    setRailLayout(railLayout().map((i) => (i.id === id ? { id, shown } : i)));
    return Promise.resolve(true);
  }

  // The gap under the pointer, counted in rows above it; both edges of the dragged row leave it in place.
  function slotAt(d: Drag, y: number): number | null {
    const slot = rows().filter((r) => {
      const box = r.getBoundingClientRect();
      return box.top + box.height / 2 < y;
    }).length;
    return slot === d.from || slot === d.from + 1 ? null : slot;
  }

  function mark(slot: number | null): void {
    const all = rows();
    for (const r of all) r.classList.remove(DROP_BEFORE, DROP_AFTER);
    if (slot === null) return;
    if (slot < all.length) all[slot]?.classList.add(DROP_BEFORE);
    else all[all.length - 1]?.classList.add(DROP_AFTER);
  }

  function end(): void {
    if (!drag) return;
    drag.row.classList.remove(LIFTED);
    list.classList.remove(SORTING);
    mark(null);
    document.removeEventListener("keydown", onEscape, true);
    window.removeEventListener("blur", end);
    drag = null;
  }

  // Captured so Escape cancels the drag without also closing the settings dialog.
  function onEscape(ev: KeyboardEvent): void {
    if (ev.key !== "Escape") return;
    ev.preventDefault();
    ev.stopPropagation();
    end();
  }

  list.addEventListener("pointerdown", (ev) => {
    end();
    if (ev.button !== 0 || !(ev.target instanceof Element) || ev.target.closest(".sw-switch")) return;
    const row = rowOf(ev.target);
    if (!row) return;
    drag = { row, from: rows().indexOf(row), start: { x: ev.clientX, y: ev.clientY }, lifted: false, slot: null };
  });
  list.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    if ((ev.buttons & LEFT_BUTTON) === 0) return end();
    if (!drag.lifted) {
      if (!pastThreshold(drag.start, { x: ev.clientX, y: ev.clientY })) return;
      drag.lifted = true;
      drag.row.classList.add(LIFTED);
      list.classList.add(SORTING);
      if (!list.hasPointerCapture(ev.pointerId)) list.setPointerCapture(ev.pointerId);
      document.addEventListener("keydown", onEscape, true);
      window.addEventListener("blur", end);
    }
    drag.slot = slotAt(drag, ev.clientY);
    mark(drag.slot);
  });
  list.addEventListener("pointerup", () => {
    const done = drag;
    end();
    if (done?.lifted && done.slot !== null) reorder(done.from, done.slot > done.from ? done.slot - 1 : done.slot);
  });
  list.addEventListener("pointercancel", end);
  list.addEventListener("keydown", (ev) => {
    if (!ev.altKey || ev.ctrlKey || ev.metaKey || ev.shiftKey || (ev.key !== "ArrowUp" && ev.key !== "ArrowDown")) return;
    const row = rowOf(ev.target);
    if (!row) return;
    ev.preventDefault();
    const from = rows().indexOf(row);
    reorder(from, ev.key === "ArrowUp" ? from - 1 : from + 1);
  });

  function row(item: RailItem, index: number, count: number): HTMLElement {
    const { label, icon } = ROWS[item.id];
    return el(
      "li",
      { role: "listitem", tabindex: "0", "data-id": item.id, "aria-label": `${label}, ${index + 1} of ${count}` },
      guiIcon(icon),
      el("span", { class: "sw-rail-label" }, label),
      switchControl(`Show ${label}`, item.shown, (on) => toggle(item.id, on)),
    );
  }

  function render(): void {
    const items = railLayout();
    list.replaceChildren(...items.map((item, index) => row(item, index, items.length)));
  }

  render();
  return el("div", { class: "sw-rail" }, list, status);
}
