// Groups of pressed-state buttons that pick one value: text segments, or color swatches such as the accent.
import { el } from "../dom";

function pressGroup<T>(cls: string, label: string, choices: [value: T, button: HTMLButtonElement][], value: T, onChange: (value: T) => void): HTMLElement {
  const buttons = choices.map(([, b]) => b);
  for (const [v, button] of choices) {
    button.setAttribute("aria-pressed", String(v === value));
    button.addEventListener("click", () => {
      buttons.forEach((b) => b.setAttribute("aria-pressed", String(b === button)));
      onChange(v);
    });
  }
  return el("div", { class: cls, role: "group", "aria-label": label }, ...buttons);
}

export function segControl<T extends string>(label: string, options: [value: T, label: string][], value: T, onChange: (value: T) => void): HTMLElement {
  return pressGroup("sw-seg", label, options.map(([v, text]) => [v, el("button", { type: "button" }, text)]), value, onChange);
}

export function dotsControl<T extends string>(label: string, options: { value: T; label: string; color: string }[], value: T, onChange: (value: T) => void): HTMLElement {
  const choices = options.map((o): [T, HTMLButtonElement] => {
    const button = el("button", { type: "button", title: o.label, "aria-label": o.label });
    button.style.background = o.color;
    return [o.value, button];
  });
  return pressGroup("sw-dots", label, choices, value, onChange);
}
