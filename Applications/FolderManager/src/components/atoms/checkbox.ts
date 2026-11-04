// Checkboxes and radios, and the hide box rows show in manage and hide-links modes.
import { el, keepClicks } from "../dom";

export const checkbox = (attrs: Record<string, string> = {}): HTMLInputElement => el("input", { type: "checkbox", ...attrs });

export const radio = (attrs: Record<string, string>): HTMLInputElement => el("input", { type: "radio", ...attrs });

export function hideCheckbox(name: string, checked: boolean): HTMLInputElement {
  const box = checkbox({ class: "hide-box", "aria-label": `Hide ${name}` });
  box.checked = checked;
  keepClicks(box);
  return box;
}
