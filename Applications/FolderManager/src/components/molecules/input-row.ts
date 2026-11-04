// Text field with its submit button; Enter submits too.
import { el } from "../dom";
import { buttonControl } from "../atoms/button";

export function inputControl(label: string, placeholder: string, buttonLabel: string, onSubmit: (value: string) => void): HTMLElement {
  const input = el("input", { type: "text", class: "sw-input", "aria-label": label, placeholder, spellcheck: "false", autocomplete: "off" });
  const submit = () => {
    if (input.value.trim()) onSubmit(input.value.trim());
  };
  input.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") submit();
  });
  return el("div", { class: "sw-actions sw-input-row" }, input, buttonControl(buttonLabel, submit));
}
