// Row of color swatches that picks one value, such as the accent color.
import { el } from "../dom";

export function dotsControl<T extends string>(label: string, options: { value: T; label: string; color: string }[], value: T, onChange: (value: T) => void): HTMLElement {
  const group = el("div", { class: "sw-dots", role: "group", "aria-label": label });
  const buttons = options.map((o) => {
    const button = el("button", { type: "button", title: o.label, "aria-label": o.label, "aria-pressed": String(o.value === value) });
    button.style.background = o.color;
    button.addEventListener("click", () => {
      buttons.forEach((b) => b.setAttribute("aria-pressed", String(b === button)));
      onChange(o.value);
    });
    return button;
  });
  group.append(...buttons);
  return group;
}
