// Segmented control: a group of pressed-state buttons that picks one value.
import { el } from "../dom";

export function segControl<T extends string>(label: string, options: [value: T, label: string][], value: T, onChange: (value: T) => void): HTMLElement {
  const group = el("div", { class: "sw-seg", role: "group", "aria-label": label });
  const buttons = options.map(([v, text]) => {
    const button = el("button", { type: "button", "aria-pressed": String(v === value) }, text);
    button.addEventListener("click", () => {
      buttons.forEach((b) => b.setAttribute("aria-pressed", String(b === button)));
      onChange(v);
    });
    return button;
  });
  group.append(...buttons);
  return group;
}
