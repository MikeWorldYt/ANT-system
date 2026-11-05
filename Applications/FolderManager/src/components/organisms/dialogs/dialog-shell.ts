// One modal shell that the info and action dialogs both open through, built on <dialog> for the native focus trap and Escape handling.
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";

export interface DialogOptions {
  title: string;
  tone?: "warning";
  wide?: boolean;
  body: (Node | string)[];
  buttons: HTMLButtonElement[];
  // Runs once the dialog is open, to move focus off the first control the browser picked.
  focus?: () => void;
}

export interface OpenDialog {
  dialog: HTMLDialogElement;
  form: HTMLFormElement;
  // The value of the button that closed it; "" when left with Escape.
  closed: Promise<string>;
}

let nextId = 0;

export const dialogText = (text: string): HTMLElement => el("p", { class: "dialog-text" }, text);

// Shows a modal and resolves with its return value; a closed modal leaves the page.
export function showModal(dialog: HTMLDialogElement, onClosed?: () => void): Promise<string> {
  const closed = new Promise<string>((resolve) => {
    dialog.addEventListener("close", () => {
      dialog.remove();
      onClosed?.();
      resolve(dialog.returnValue);
    });
  });
  document.body.append(dialog);
  dialog.showModal();
  return closed;
}

export function openDialog({ title, tone, wide, body, buttons, focus }: DialogOptions): OpenDialog {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const id = `dialog-${++nextId}`;
  const heading = el("h2", { id }, title);
  const icon = tone === "warning" ? [el("span", { class: "warning-confirm-icon", "aria-hidden": "true" }, guiIcon("warning"))] : [];
  const form = el("form", { method: "dialog" }, ...icon, heading, ...body, el("div", { class: "dialog-buttons" }, ...buttons));
  const classes = ["confirm", ...(tone === "warning" ? ["warning-confirm"] : []), ...(wide ? ["dialog-wide"] : [])];
  const dialog = el("dialog", { class: classes.join(" "), "aria-labelledby": id }, form);
  const closed = showModal(dialog, () => previous?.focus());
  focus?.();
  return { dialog, form, closed };
}
