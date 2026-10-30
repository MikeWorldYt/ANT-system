// Editor for the collapsed side panel: which rail buttons show and in what order.
import { el } from "../../../ui/dom";
import { iconButton } from "../../../ui/icon-button";
import { railLayout, setRailLayout, type RailId, type RailItem } from "../../../store/side-layout";
import { setDisabled } from "../../side-action";
import { switchControl } from "../page";

const LABELS: Record<RailId, string> = {
  settings: "Settings",
  favorite: "Favorite",
  edit: "Edit details",
  moveUp: "Move up",
  moveDown: "Move down",
  delete: "Delete",
};

type Direction = "up" | "down";

export function railEditor(): HTMLElement {
  const list = el("ul", { class: "sw-rail", "aria-label": "Collapsed side panel buttons" });

  function move(id: RailId, dir: Direction): void {
    const items = [...railLayout()];
    const from = items.findIndex((i) => i.id === id);
    const to = dir === "up" ? from - 1 : from + 1;
    if (from < 0 || to < 0 || to >= items.length) return;
    [items[from], items[to]] = [items[to]!, items[from]!];
    setRailLayout(items);
    render();
    // The rows are rebuilt, so focus follows the moved button to its new row.
    list.querySelector<HTMLButtonElement>(`[data-move="${id}-${dir}"]`)?.focus();
  }

  function toggle(id: RailId, shown: boolean): Promise<boolean> {
    setRailLayout(railLayout().map((i) => (i.id === id ? { id, shown } : i)));
    return Promise.resolve(true);
  }

  function row(item: RailItem, index: number, count: number): HTMLElement {
    const label = LABELS[item.id];
    const mover = (dir: Direction, edge: boolean) => {
      const button = iconButton(`Move ${label} ${dir}`, dir === "up" ? "moveUp" : "moveDown", () => move(item.id, dir));
      button.dataset.move = `${item.id}-${dir}`;
      setDisabled(button, edge);
      return button;
    };
    return el(
      "li",
      {},
      el("span", { class: "sw-rail-label" }, label),
      switchControl(`Show ${label}`, item.shown, (on) => toggle(item.id, on)),
      mover("up", index === 0),
      mover("down", index === count - 1),
    );
  }

  function render(): void {
    const items = railLayout();
    list.replaceChildren(...items.map((item, index) => row(item, index, items.length)));
  }

  render();
  return list;
}
