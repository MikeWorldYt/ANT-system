// Search box of the navigation bar: queries the db index and jumps to the picked folder.
import { describeError, searchNodes } from "./api";
import { el } from "./dom";
import { findNode, reveal } from "./state";
import { setStatus } from "./status";

const LIMIT = 50;
const DELAY_MS = 200;

export function createSearchBar(): HTMLElement {
  const input = el("input", {
    class: "search-input",
    type: "search",
    placeholder: "Search",
    "aria-label": "Search folders",
    role: "combobox",
    "aria-expanded": "false",
    "aria-controls": "search-results",
    spellcheck: "false",
    autocomplete: "off",
  });
  const list = el("ul", { class: "search-results", id: "search-results", role: "listbox", "aria-label": "Search results" });
  list.hidden = true;
  const element = el("div", { class: "search-box" }, input, list);

  let hits: string[][] = [];
  let active = 0;
  let latest = 0;
  let timer: number | undefined;

  function close(): void {
    list.hidden = true;
    input.setAttribute("aria-expanded", "false");
    input.removeAttribute("aria-activedescendant");
  }

  function mark(): void {
    list.querySelectorAll("li[role=option]").forEach((li, i) => li.setAttribute("aria-selected", String(i === active)));
    const current = list.children[active];
    if (current) input.setAttribute("aria-activedescendant", current.id);
    current?.scrollIntoView({ block: "nearest" });
  }

  function show(): void {
    const shown = hits.slice(0, LIMIT);
    const rows = shown.map((parts, i) => {
      const li = el(
        "li",
        { class: "search-hit", role: "option", id: `search-hit-${i}` },
        el("span", { class: "hit-name" }, parts[parts.length - 1] ?? ""),
        el("span", { class: "hit-path" }, parts.slice(0, -1).join(" › ")),
      );
      li.addEventListener("click", () => void open(i));
      return li;
    });
    if (rows.length === 0) rows.push(el("li", { class: "search-empty" }, "No matches"));
    if (hits.length > LIMIT) rows.push(el("li", { class: "search-empty" }, `${hits.length - LIMIT} more, narrow the search`));
    list.replaceChildren(...rows);
    list.hidden = false;
    input.setAttribute("aria-expanded", "true");
    mark();
  }

  async function run(): Promise<void> {
    const query = input.value.trim();
    const mine = ++latest;
    if (!query) {
      close();
      return;
    }
    try {
      const found = await searchNodes(query);
      if (mine !== latest) return;
      hits = found;
      active = 0;
      show();
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
    await reveal(parts);
    if (!findNode(parts)) setStatus("warning", "That folder is no longer there. Refresh and search again");
  }

  input.addEventListener("input", () => {
    window.clearTimeout(timer);
    timer = window.setTimeout(() => void run(), DELAY_MS);
  });
  input.addEventListener("keydown", (ev) => {
    ev.stopPropagation();
    const count = Math.min(hits.length, LIMIT);
    if (ev.key === "ArrowDown" && !list.hidden && count > 0) active = (active + 1) % count;
    else if (ev.key === "ArrowUp" && !list.hidden && count > 0) active = (active - 1 + count) % count;
    else if (ev.key === "Enter") {
      window.clearTimeout(timer);
      if (list.hidden) void run();
      else void open(active);
    } else if (ev.key === "Escape") {
      if (list.hidden) input.value = "";
      close();
    } else return;
    ev.preventDefault();
    if (ev.key.startsWith("Arrow")) mark();
  });
  input.addEventListener("focus", () => {
    if (hits.length > 0 && input.value.trim()) show();
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
