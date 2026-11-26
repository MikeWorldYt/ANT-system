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
export async function choiceDialog(title: string, message: string | HTMLElement, choices: [value: string, label: string, tone?: ButtonTone][]): Promise<string> {
  const buttons = choices.map(([value, label, tone], i) => dialogButton(label, value, tone ?? (i === choices.length - 1 ? "primary" : undefined)));
  const value = await infoDialog({ title, body: [typeof message === "string" ? dialogText(message) : message], buttons, focus: () => buttons[0]?.focus() });
  return value === "" ? "later" : value;
}

export const confirmDialog = (title: string, message: string, confirmLabel: string): Promise<boolean> =>
  confirmed({ title, body: [el("p", {}, message)] }, dialogButton(confirmLabel, "ok", "danger"));

const warningCopy = (paragraphs: (string | HTMLElement)[]): HTMLElement =>
  el("div", { class: "warning-confirm-copy" }, ...paragraphs.map((paragraph) => (typeof paragraph === "string" ? el("p", {}, paragraph) : paragraph)));

export function warningConfirmDialog(title: string, paragraphs: (string | HTMLElement)[], confirmLabel: string, confirmTone: ButtonTone = "primary"): Promise<boolean> {
  return confirmed({ title, tone: "warning", body: [warningCopy(paragraphs)] }, dialogButton(confirmLabel, "ok", confirmTone));
}

// Closes itself as declined once whoever asked has stopped waiting for the answer.
export async function timedConfirmDialog(title: string, paragraphs: (string | HTMLElement)[], confirmLabel: string, seconds: number): Promise<boolean> {
  const cancel = cancelButton();
  const { dialog, closed } = openDialog({ title, tone: "warning", body: [warningCopy(paragraphs)], buttons: [dialogButton(confirmLabel, "ok", "danger"), cancel], focus: () => cancel.focus() });
  const timer = window.setTimeout(() => dialog.close(""), Math.max(1, seconds - 1) * 1000);
  const value = await closed;
  window.clearTimeout(timer);
  return value === "ok";
}
