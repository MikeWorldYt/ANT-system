// Base of the action dialogs: the scrolling table they settle one row per folder in.
import "../../../../styles/components/organisms/dialogs/action-dialog.css";
import { el } from "../../../dom";

export function colHeader(kids: (Node | string)[], attrs: Record<string, string> = {}): HTMLTableCellElement {
  return el("th", { scope: "col", ...attrs }, ...kids);
}

export function dialogTable(label: string, headers: HTMLTableCellElement[], rows: HTMLElement[]): { table: HTMLTableElement; view: HTMLElement } {
  const table = el("table", { class: "dialog-table", "aria-label": label }, el("thead", {}, el("tr", {}, ...headers)), el("tbody", {}, ...rows));
  return { table, view: el("div", { class: "dialog-table-scroll" }, table) };
}
