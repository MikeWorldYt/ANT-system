// Native select over a typed list of values.
import { el } from "../dom";

export function selectControl<T extends string>(label: string, options: [value: T, label: string][], value: T, onChange: (value: T) => void): HTMLElement {
  const select = el("select", { class: "sw-select", "aria-label": label }, ...options.map(([v, text]) => el("option", { value: v }, text)));
  select.value = value;
  select.addEventListener("change", () => onChange(select.value as T));
  return select;
}
