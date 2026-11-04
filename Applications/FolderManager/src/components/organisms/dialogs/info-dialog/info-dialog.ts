// Info dialogs: a message and a couple of buttons, opened through the shared dialog shell.
import { el } from "../../../dom";
import { cancelButton, dialogButton } from "../../../atoms/button";
import { dialogText, openDialog } from "../dialog-shell";
import type { ButtonTone } from "../../../atoms/button";
import type { DialogOptions } from "../dialog-shell";

type InfoOptions = Omit<DialogOptions, "wide">;

// Base of every info dialog: resolves to the value of the button that closed it, "" for Escape.
export const infoDialog = (options: InfoOptions): Promise<string> => openDialog(options).closed;

// Cancel holds the focus, so Enter never confirms by accident.
async function confirmed(options: Omit<InfoOptions, "buttons" | "focus">, confirm: HTMLButtonElement): Promise<boolean> {
  const cancel = cancelButton();
  return (await infoDialog({ ...options, buttons: [confirm, cancel], focus: () => cancel.focus() })) === "ok";
}

// The first choice is the safest one and gets focus; Escape resolves to "later".
export async function choiceDialog(title: string, message: string, choices: [value: string, label: string][]): Promise<string> {
  const buttons = choices.map(([value, label], i) => dialogButton(label, value, i === choices.length - 1 ? "primary" : undefined));
  const value = await infoDialog({ title, body: [dialogText(message)], buttons, focus: () => buttons[0]?.focus() });
  return value === "" ? "later" : value;
}

export const confirmDialog = (title: string, message: string, confirmLabel: string): Promise<boolean> =>
  confirmed({ title, body: [el("p", {}, message)] }, dialogButton(confirmLabel, "ok", "danger"));

export function warningConfirmDialog(title: string, paragraphs: (string | HTMLElement)[], confirmLabel: string, confirmTone: ButtonTone = "primary"): Promise<boolean> {
  const copy = el("div", { class: "warning-confirm-copy" }, ...paragraphs.map((paragraph) => (typeof paragraph === "string" ? el("p", {}, paragraph) : paragraph)));
  return confirmed({ title, tone: "warning", body: [copy] }, dialogButton(confirmLabel, "ok", confirmTone));
}
