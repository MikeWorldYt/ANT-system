// Text buttons: the submit buttons a dialog closes with, and the plain, outline or danger buttons used everywhere else.
import { el } from "../dom";

export type ButtonTone = "primary" | "danger";

const btnClass = (extra?: string): string => (extra ? `btn ${extra}` : "btn");

export function dialogButton(label: string, value: string, tone?: ButtonTone): HTMLButtonElement {
  return el("button", { type: "submit", class: btnClass(tone && `btn-${tone}`), value }, label);
}

// Escape also leaves a dialog with "", which every caller reads as this button.
export const cancelButton = (label = "Cancel"): HTMLButtonElement => dialogButton(label, "cancel");

export function textButton(label: string, extraClass?: string, onClick?: () => void): HTMLButtonElement {
  const button = el("button", { type: "button", class: btnClass(extraClass) }, label);
  if (onClick) button.addEventListener("click", onClick);
  return button;
}

export const buttonControl = (label: string, onClick: () => void, variant: "plain" | "outline" | "danger" = "plain"): HTMLButtonElement =>
  textButton(label, variant === "plain" ? undefined : `sw-${variant}`, onClick);
