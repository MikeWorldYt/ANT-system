// Icon button named below its icon, for picking a view in the toolbar and in Settings.
import { el } from "../dom";
import { iconButton } from "../atoms/icon-button";
import type { GuiIcon } from "../atoms/gui-icons";

export const VIEW_ICONS: Record<string, GuiIcon> = { tree: "viewTree", columns: "viewColumns", grid: "viewGrid", list: "viewList", cards: "viewCards" };

export function viewChoice(label: string, icon: GuiIcon, onClick: () => void): HTMLButtonElement {
  const button = iconButton(label, icon, onClick);
  button.classList.add("view-choice");
  button.append(el("span", { class: "view-choice-label" }, label));
  return button;
}

export function viewChoices<T extends string>(label: string, options: [T, string, GuiIcon][], value: T, onChange: (value: T) => void): HTMLElement {
  const buttons = options.map(([id, name, icon]): [T, HTMLButtonElement] => [id, viewChoice(name, icon, () => pick(id))]);
  function pick(id: T): void {
    buttons.forEach(([other, b]) => b.setAttribute("aria-pressed", String(other === id)));
    onChange(id);
  }
  buttons.forEach(([id, b]) => b.setAttribute("aria-pressed", String(id === value)));
  return el("div", { class: "view-choices", role: "group", "aria-label": label }, ...buttons.map(([, b]) => b));
}
