// Filter chips of the search box and the parsing of the `key:value` token being typed.
import { el } from "../ui/dom";

export interface Chip {
  key: string;
  value: string;
  label: string;
}

// The last word of the input when it reads `key:partial`.
const TOKEN = /(?:^|\s)([A-Za-z]+):(\S*)$/;

// Picked values are sent whole (`=`), and quoted so spaces survive the tokenizer.
export const chipQuery = (c: Chip) => (c.key === "in" ? `in:"/${c.value}"` : `${c.key}:"=${c.value}"`);

export function splitToken(value: string): { before: string; key: string; partial: string } | null {
  const m = TOKEN.exec(value);
  return m ? { before: value.slice(0, m.index), key: (m[1] ?? "").toLowerCase(), partial: m[2] ?? "" } : null;
}

export function renderChips(field: HTMLElement, input: HTMLElement, chips: Chip[], onChange: () => void): void {
  field.querySelectorAll(".search-chip").forEach((c) => c.remove());
  chips.forEach((chip, i) => {
    const remove = el("button", { type: "button", class: "chip-x", "aria-label": `Remove ${chip.key}: ${chip.label}`, tabindex: "-1" }, "×");
    remove.addEventListener("click", () => {
      chips.splice(i, 1);
      renderChips(field, input, chips, onChange);
      onChange();
    });
    field.insertBefore(el("span", { class: "search-chip" }, el("span", { class: "chip-key" }, `${chip.key}:`), el("span", { class: "chip-value" }, chip.label), remove), input);
  });
}
