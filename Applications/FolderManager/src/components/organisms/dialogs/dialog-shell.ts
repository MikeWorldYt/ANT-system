// One modal shell that the info and action dialogs both open through, built on <dialog> for the native focus trap and Escape handling.
import "../../../styles/components/organisms/dialogs/dialog-shell.css";
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";

export interface DialogOptions {
  title?: string;
  // Labels the dialog when it has no title.
  label?: HTMLElement;
  tone?: "warning";
  wide?: boolean;
  body: (Node | string)[];
  buttons: HTMLButtonElement[];
}

export interface OpenDialog {
  dialog: HTMLDialogElement;
  form: HTMLFormElement;
  // The value of the button that closed it; "" when left with Escape.
  closed: Promise<string>;
}

let nextId = 0;

export function openDialog({ title, label, tone, wide, body, buttons }: DialogOptions): OpenDialog {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const id = `dialog-${++nextId}`;
  const heading = title === undefined ? null : el("h2", {}, title);
  const labelled = heading ?? label;
  if (labelled) labelled.id = id;
  const icon = tone === "warning" ? [el("span", { class: "warning-confirm-icon", "aria-hidden": "true" }, guiIcon("warning"))] : [];
  const form = el("form", { method: "dialog" }, ...icon, ...(heading ? [heading] : []), ...body, el("div", { class: "dialog-buttons" }, ...buttons));
  const classes = ["confirm", ...(tone === "warning" ? ["warning-confirm"] : []), ...(wide ? ["dialog-wide"] : [])];
  const dialog = el("dialog", labelled ? { class: classes.join(" "), "aria-labelledby": id } : { class: classes.join(" ") }, form);
  const closed = new Promise<string>((resolve) => {
    dialog.addEventListener("close", () => {
      dialog.remove();
      previous?.focus();
      resolve(dialog.returnValue);
    });
  });
  document.body.append(dialog);
  dialog.showModal();
  return { dialog, form, closed };
}
