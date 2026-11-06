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

export function dialogTable(label: string, headers: HTMLTableCellElement[], rows: HTMLElement[], subHeaders: HTMLTableCellElement[] = []): { table: HTMLTableElement; view: HTMLElement } {
  const head = el("thead", {}, el("tr", {}, ...headers), ...(subHeaders.length > 0 ? [el("tr", {}, ...subHeaders)] : []));
  const table = el("table", { class: "dialog-table", "aria-label": label }, head, el("tbody", {}, ...rows));
  return { table, view: el("div", { class: "dialog-table-scroll" }, table) };
}

// A wide dialog around a table: Later leaves every row as it is, Apply settles them.
export function actionDialog(title: string, intro: string | HTMLElement[], view: HTMLElement, focus?: () => void, labels: { later: string; apply: string } = { later: "Later", apply: "Apply" }): ActionDialog {
  const later = cancelButton(labels.later);
  const apply = dialogButton(labels.apply, "ok", "primary");
  return { ...openDialog({ title, wide: true, body: [...(typeof intro === "string" ? [dialogText(intro)] : intro), view], buttons: [later, apply], focus }), later, apply };
}

// One field and its confirm button; Cancel skips the field's validation. Resolves to whether it was confirmed.
export async function fieldDialog(title: string, intro: HTMLElement, field: HTMLElement, confirm: HTMLButtonElement, focus: () => void): Promise<boolean> {
  const cancel = cancelButton();
  cancel.formNoValidate = true;
  return (await openDialog({ title, body: [intro, field], buttons: [cancel, confirm], focus }).closed) === "ok";
}
