// Info dialogs: a message and a couple of buttons, rendered through the shared dialog shell.
import { el } from "../../../dom";
import { dialogButton } from "../../../atoms/button";
import { openDialog } from "../dialog-shell";
import type { ButtonTone } from "../../../atoms/button";

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
