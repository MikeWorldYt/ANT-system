// Text field with its submit button; Enter submits too.
import { el } from "../dom";
import { buttonControl } from "../atoms/button";
import { textInput } from "../atoms/text-input";

export function inputControl(label: string, placeholder: string, buttonLabel: string, onSubmit: (value: string) => void): HTMLElement {
  const input = textInput({ class: "sw-input", "aria-label": label, placeholder });
  const submit = () => {
    if (input.value.trim()) onSubmit(input.value.trim());
  };
  input.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") submit();
  });
  return el("div", { class: "sw-actions sw-input-row" }, input, buttonControl(buttonLabel, submit));
}
