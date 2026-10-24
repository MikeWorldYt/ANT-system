// What every settings page shares: its shape, the row layout and the controls rows are made of.
import type { StorageInfo } from "../../ipc/types";
import { el } from "../../ui/dom";
import type { GuiIcon } from "../../ui/gui-icons";

export interface RowHandle {
  describe(text: string): void;
}

export interface PageContext {
  storage: StorageInfo | null;
  refresh(): void;
}

export interface SettingRow {
  section?: string;
  title: string;
  description?: string;
  control(row: RowHandle): HTMLElement;
}

export interface SettingsPage {
  id: string;
  group: string;
  title: string;
  icon: GuiIcon;
  rows(ctx: PageContext): SettingRow[];
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

export function switchControl(label: string, checked: boolean, onChange: (on: boolean) => Promise<boolean>): HTMLElement {
  const button = el("button", { type: "button", class: "sw-switch", role: "switch", "aria-label": label, "aria-checked": String(checked) });
  button.addEventListener("click", async () => {
    const next = button.getAttribute("aria-checked") !== "true";
    if (await onChange(next)) button.setAttribute("aria-checked", String(next));
  });
  return button;
}

export function selectControl<T extends string>(label: string, options: [value: T, label: string][], value: T, onChange: (value: T) => void): HTMLElement {
  const select = el("select", { class: "sw-select", "aria-label": label }, ...options.map(([v, text]) => el("option", { value: v }, text)));
  select.value = value;
  select.addEventListener("change", () => onChange(select.value as T));
  return select;
}

export function buttonControl(label: string, onClick: () => void, outline = false): HTMLButtonElement {
  const button = el("button", { type: "button", class: outline ? "btn sw-outline" : "btn" }, label);
  button.addEventListener("click", onClick);
  return button;
}

export const actions = (...buttons: HTMLElement[]): HTMLElement => el("div", { class: "sw-actions" }, ...buttons);

export const valueControl = (text: string): HTMLElement => el("span", { class: "sw-value" }, text);

export const pathControl = (path: string): HTMLElement => el("span", { class: "sw-path", title: path }, path);
