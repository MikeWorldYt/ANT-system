// Native select over a typed list of values.
import { el } from "../dom";

export function selectBox(label: string, options: [value: string, label: string][], extraClass: string): HTMLSelectElement {
  return el("select", { class: extraClass, "aria-label": label }, ...options.map(([value, text]) => el("option", { value }, text)));
}

export function selectControl<T extends string>(label: string, options: [value: T, label: string][], value: T, onChange: (value: T) => void): HTMLElement {
  const select = selectBox(label, options, "sw-select");
  select.value = value;
  select.addEventListener("change", () => onChange(select.value as T));
  return select;
}
