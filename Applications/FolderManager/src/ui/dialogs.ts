// The app's stock prompts, each rendered through the shared dialog shell.
import { el } from "./dom";
import { dialogButton, openDialog } from "./dialog/shell";
import type { ButtonTone } from "./dialog/shell";

// The first choice is the safest one and gets focus; Escape resolves to "later".
export async function choiceDialog(title: string, message: string, choices: [value: string, label: string][]): Promise<string> {
  const buttons = choices.map(([value, label], i) => dialogButton(label, value, i === choices.length - 1 ? "primary" : undefined));
  const { closed } = openDialog({ title, body: [el("p", { class: "dialog-text" }, message)], buttons });
  buttons[0]?.focus();
  const value = await closed;
  return value === "" ? "later" : value;
}

export async function confirmDialog(title: string, message: string, confirmLabel: string): Promise<boolean> {
  const cancel = dialogButton("Cancel", "cancel");
  const { closed } = openDialog({ title, body: [el("p", {}, message)], buttons: [dialogButton(confirmLabel, "ok", "danger"), cancel] });
  cancel.focus();
  return (await closed) === "ok";
}

export async function warningConfirmDialog(title: string, paragraphs: (string | HTMLElement)[], confirmLabel: string, confirmTone: ButtonTone = "primary"): Promise<boolean> {
  const cancel = dialogButton("Cancel", "cancel");
  const copy = el("div", { class: "warning-confirm-copy" }, ...paragraphs.map((paragraph) => (typeof paragraph === "string" ? el("p", {}, paragraph) : paragraph)));
  const { closed } = openDialog({ title, tone: "warning", body: [copy], buttons: [dialogButton(confirmLabel, "ok", confirmTone), cancel] });
  cancel.focus();
  return (await closed) === "ok";
}

// Resolves to the typed text, or null when cancelled.
export async function promptDialog(title: string, label: string, initial: string, confirmLabel: string): Promise<string | null> {
  const input = el("input", { type: "text", class: "dialog-input", "aria-label": label, value: initial });
  const cancel = dialogButton("Cancel", "cancel");
  cancel.formNoValidate = true;
  const { closed } = openDialog({ title, body: [el("p", {}, label), input], buttons: [cancel, dialogButton(confirmLabel, "ok", "primary")] });
  input.select();
  return (await closed) === "ok" ? input.value.trim() : null;
}

// Resolves to the chosen value, or null when cancelled.
export async function selectDialog(title: string, message: string, options: [value: string, label: string][], confirmLabel: string): Promise<string | null> {
  const select = el("select", { class: "dialog-input", "aria-label": title }, ...options.map(([value, label]) => el("option", { value }, label)));
  const { closed } = openDialog({ title, body: [el("p", { class: "dialog-text" }, message), select], buttons: [dialogButton("Cancel", "cancel"), dialogButton(confirmLabel, "ok", "danger")] });
  select.focus();
  return (await closed) === "ok" ? select.value : null;
}
