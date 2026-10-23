// Search box of the navigation bar: filter chips, completions for `key:` and a results list.
import { describeError, searchNodes, suggestSearch, type Suggestion } from "./api";
import { el } from "./dom";
import { guiIcon } from "./gui-icons";
import { findNode, reveal } from "./state";
import { setStatus } from "./status";

interface Chip {
  key: string;
  value: string;
  label: string;
}

interface Item {
  label: string;
  hint: string;
  pick(): void;
}

const LIMIT = 50;
const DELAY_MS = 200;
// The last word of the input when it reads `key:partial`.
const TOKEN = /(?:^|\s)([A-Za-z]+):(\S*)$/;

// Picked values are sent whole (`=`), and quoted so spaces survive the tokenizer.
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

const chipQuery = (c: Chip) => (c.key === "in" ? `in:"/${c.value}"` : `${c.key}:"=${c.value}"`);

export function createSearchBar(): HTMLElement {
  const input = el("input", {
    class: "search-input",
    type: "text",
    placeholder: "Search",
    "aria-label": "Search folders",
    role: "combobox",
    "aria-expanded": "false",
    "aria-controls": "search-results",
    "aria-haspopup": "listbox",
    "aria-autocomplete": "list",
    spellcheck: "false",
    autocomplete: "off",
  });
  const field = el("div", { class: "search-field" }, input, guiIcon("search"));
  const list = el("ul", { class: "search-results", id: "search-results", role: "listbox", "aria-label": "Search results" });
  list.hidden = true;
  const live = el("div", { role: "status", "aria-live": "polite" });
  // Screen-reader-only: the visible messages sit in the list, outside the option set.
  Object.assign(live.style, { position: "absolute", width: "1px", height: "1px", overflow: "hidden", clip: "rect(0 0 0 0)", whiteSpace: "nowrap" });
  const element = el("div", { class: "search-box" }, field, list, live);

  const chips: Chip[] = [];
  let hits: string[][] = [];
  let items: Item[] = [];
  let lastSuggest: { key: string; prefix: string; found: Suggestion[] } | null = null;
  let active = 0;
  let latest = 0;
  let timer: number | undefined;

  function renderChips(): void {
    field.querySelectorAll(".search-chip").forEach((c) => c.remove());
    chips.forEach((chip, i) => {
      const remove = el("button", { type: "button", class: "chip-x", "aria-label": `Remove ${chip.key}: ${chip.label}`, tabindex: "-1" }, "×");
      remove.addEventListener("click", () => {
        chips.splice(i, 1);
        renderChips();
        void run();
      });
      field.insertBefore(el("span", { class: "search-chip" }, el("span", { class: "chip-key" }, `${chip.key}:`), el("span", { class: "chip-value" }, chip.label), remove), input);
    });
  }

  function close(): void {
    list.hidden = true;
    input.setAttribute("aria-expanded", "false");
    input.removeAttribute("aria-activedescendant");
    live.textContent = "";
  }

  function mark(): void {
    const options = list.querySelectorAll("li[role=option]");
    options.forEach((li, i) => li.setAttribute("aria-selected", String(i === active)));
    const current = options[active];
    if (current) input.setAttribute("aria-activedescendant", current.id);
    else input.removeAttribute("aria-activedescendant");
    current?.scrollIntoView({ block: "nearest" });
  }

  function show(extra: string[] = []): void {
    const rows = items.map((item, i) => {
      const li = el("li", { class: "search-hit", role: "option", id: `search-hit-${i}` }, el("span", { class: "hit-name" }, item.label), el("span", { class: "hit-path" }, item.hint));
      li.addEventListener("click", item.pick);
      return li;
    });
    const notes = items.length === 0 ? ["No matches", ...extra] : extra;
    rows.push(...notes.map((text) => el("li", { class: "search-empty", role: "presentation" }, text)));
    list.replaceChildren(...rows);
    live.textContent = items.length === 0 ? "No matches" : plural(items.length, "result");
    list.hidden = false;
    input.setAttribute("aria-expanded", "true");
    active = 0;
    mark();
  }

  function splitToken(value: string): { before: string; key: string; partial: string } | null {
    const m = TOKEN.exec(value);
    return m ? { before: value.slice(0, m.index), key: (m[1] ?? "").toLowerCase(), partial: m[2] ?? "" } : null;
  }

  function pickSuggestion(key: string, before: string, s: Suggestion): void {
    chips.push({ key, value: s.value, label: s.label });
    input.value = before;
    lastSuggest = null;
    renderChips();
    input.focus();
    void run();
  }

  async function suggest(key: string, before: string, partial: string, mine: number): Promise<boolean> {
    const found = await suggestSearch(key, partial);
    if (mine !== latest) return true;
    lastSuggest = { key, prefix: partial, found };
    if (found.length === 0) return false;
    items = found.map((s) => ({ label: s.label, hint: s.hint, pick: () => pickSuggestion(key, before, s) }));
    show();
    return true;
  }

  async function run(): Promise<void> {
    const mine = ++latest;
    const text = input.value.trim();
    if (!text && chips.length === 0) {
      close();
      return;
    }
    try {
      const token = splitToken(input.value);
      if (token && (await suggest(token.key, token.before, token.partial, mine))) return;
      const found = await searchNodes([...chips.map(chipQuery), text].filter(Boolean).join(" "));
      if (mine !== latest) return;
      hits = found;
      items = found.slice(0, LIMIT).map((parts, i) => ({
        label: parts[parts.length - 1] ?? "",
        hint: parts.slice(0, -1).join(" › "),
        pick: () => void open(i),
      }));
      show(found.length > LIMIT ? [`${found.length - LIMIT} more, narrow the search`] : []);
    } catch (e) {
      if (mine !== latest) return;
      close();
      setStatus("error", describeError(e));
    }
  }

  async function open(i: number): Promise<void> {
    const parts = hits[i];
    if (!parts) return;
    close();
    try {
      await reveal(parts);
    } catch (e) {
      setStatus("error", describeError(e));
      return;
    }
    if (!findNode(parts)) setStatus("warning", "That folder is no longer there. Refresh and search again");
  }

  // Typing a space right after a complete `key:value` closes it into a chip, as long as a suggestion matches it exactly.
  function confirmWithSpace(): boolean {
    if (!/\s$/.test(input.value) || !lastSuggest) return false;
    const token = splitToken(input.value.trimEnd());
    if (!token || token.key !== lastSuggest.key) return false;
    const exact = lastSuggest.found.find((s) => [s.label, s.value].some((v) => v.toLowerCase() === token.partial.toLowerCase()));
    if (!exact) return false;
    pickSuggestion(token.key, token.before, exact);
    return true;
  }

  input.addEventListener("input", () => {
    window.clearTimeout(timer);
    if (confirmWithSpace()) return;
    timer = window.setTimeout(() => void run(), DELAY_MS);
  });
  input.addEventListener("keydown", (ev) => {
    ev.stopPropagation();
    const isOpen = !list.hidden && items.length > 0;
    if (ev.key === "ArrowDown" && isOpen) active = (active + 1) % items.length;
    else if (ev.key === "ArrowUp" && isOpen) active = (active - 1 + items.length) % items.length;
    else if (ev.key === "Enter" || (ev.key === "Tab" && isOpen && !ev.shiftKey)) {
      window.clearTimeout(timer);
      if (isOpen) items[active]?.pick();
      else void run();
    } else if (ev.key === "Escape") {
      if (list.hidden) input.value = "";
      close();
    } else if (ev.key === "Backspace" && input.value === "" && chips.length > 0) {
      chips.pop();
      renderChips();
      void run();
    } else return;
    ev.preventDefault();
    if (ev.key.startsWith("Arrow")) mark();
  });
  input.addEventListener("focus", () => {
    if (items.length > 0 && (input.value.trim() || chips.length > 0)) show();
  });
  // Mousedown on a hit would blur the input first and close the list before the click lands.
  list.addEventListener("mousedown", (ev) => ev.preventDefault());
  input.addEventListener("blur", close);
  document.addEventListener("keydown", (ev) => {
    if (ev.ctrlKey && !ev.altKey && !ev.shiftKey && !ev.metaKey && ev.key.toLowerCase() === "f") {
      ev.preventDefault();
      input.focus();
      input.select();
    }
  });

  return element;
}
