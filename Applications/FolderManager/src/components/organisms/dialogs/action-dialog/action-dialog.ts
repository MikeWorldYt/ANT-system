// Base of the action dialogs: forms that settle a table of folders, or ask for one field.
import { el } from "../../../dom";
import { cancelButton, dialogButton } from "../../../atoms/button";
import { dialogText, openDialog, type OpenDialog } from "../dialog-shell";

export interface ActionDialog extends OpenDialog {
  later: HTMLButtonElement;
  apply: HTMLButtonElement;
}

export function colHeader(kids: (Node | string)[], attrs: Record<string, string> = {}): HTMLTableCellElement {
  return el("th", { scope: "col", ...attrs }, ...kids);
}

export function dialogTable(label: string, headers: HTMLTableCellElement[], rows: HTMLElement[]): { table: HTMLTableElement; view: HTMLElement } {
  const table = el("table", { class: "dialog-table", "aria-label": label }, el("thead", {}, el("tr", {}, ...headers)), el("tbody", {}, ...rows));
  return { table, view: el("div", { class: "dialog-table-scroll" }, table) };
}

// A wide dialog around a table: Later leaves every row as it is, Apply settles them.
export function actionDialog(title: string, intro: string | HTMLElement[], view: HTMLElement, focus?: () => void): ActionDialog {
  const later = cancelButton("Later");
  const apply = dialogButton("Apply", "ok", "primary");
  return { ...openDialog({ title, wide: true, body: [...(typeof intro === "string" ? [dialogText(intro)] : intro), view], buttons: [later, apply], focus }), later, apply };
}

// One field and its confirm button; Cancel skips the field's validation. Resolves to whether it was confirmed.
export async function fieldDialog(title: string, intro: HTMLElement, field: HTMLElement, confirm: HTMLButtonElement, focus: () => void): Promise<boolean> {
  const cancel = cancelButton();
  cancel.formNoValidate = true;
  return (await openDialog({ title, body: [intro, field], buttons: [cancel, confirm], focus }).closed) === "ok";
}
