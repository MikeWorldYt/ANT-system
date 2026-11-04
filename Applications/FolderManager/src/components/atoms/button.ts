// Text buttons: the submit buttons a dialog closes with and the plain, outline or danger buttons of the settings rows.
import { el } from "../dom";

export type ButtonTone = "primary" | "danger";

export function dialogButton(label: string, value: string, tone?: ButtonTone): HTMLButtonElement {
  return el("button", { type: "submit", class: tone ? `btn btn-${tone}` : "btn", value }, label);
}

export function buttonControl(label: string, onClick: () => void, variant: "plain" | "outline" | "danger" = "plain"): HTMLButtonElement {
  const button = el("button", { type: "button", class: variant === "plain" ? "btn" : `btn sw-${variant}` }, label);
  button.addEventListener("click", onClick);
  return button;
}
