// Settings row: a title, a description the control can rewrite, and the control itself.
import { el } from "../dom";

export interface RowHandle {
  describe(text: string): void;
}

export interface SettingRow {
  section?: string;
  title: string;
  description?: string;
  control(row: RowHandle): HTMLElement;
}

export function renderRow(row: SettingRow): HTMLElement {
  const description = el("div", { class: "sw-desc" }, row.description ?? "");
  description.hidden = !row.description;
  const handle: RowHandle = {
    describe(text) {
      description.textContent = text;
      description.hidden = text === "";
    },
  };
  const text = el("div", { class: "sw-text" }, el("div", { class: "sw-title" }, row.title), description);
  return el("div", { class: "sw-row" }, text, row.control(handle));
}
