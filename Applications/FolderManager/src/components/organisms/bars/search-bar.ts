// Search box of the navigation bar: filter chips, completions for `key:` and a results list.
import { searchDisk, searchFiles, searchNodes, suggestSearch, type FileHit, type Suggestion } from "../../../services/ipc/api";
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { findNode, reveal } from "../../../services/store/state";
import { setStatus, reportError } from "./status-line";
import { chipQuery, renderChips, splitToken, type Chip } from "../../molecules/search-chips";
import { plural } from "../../../services/plural";
import { textInput } from "../../atoms/text-input";
import { hitIcon, installSearchDrag } from "../../../hooks/search-drag";

interface Item {
  label: string;
  hint: string;
  parts?: string[];
  icon?: HTMLElement;
  pick(): void;
}

export type SearchMode = "folders" | "files";

const MODES: Record<SearchMode, { placeholder: string; label: string }> = {
  folders: { placeholder: "Search", label: "Search folders" },
  files: { placeholder: "Search files", label: "Search files inside folders" },
};

const LIMIT = 50;
const DELAY_MS = 200;
// Walking the whole storage can be heavy, so it waits until typing has been idle this long.
const REST_DELAY_MS = 5_000;

export function createSearchBar(): { element: HTMLElement; focus(mode: SearchMode): void } {
  const input = textInput({
    class: "search-input",
    placeholder: "Search",
    "aria-label": "Search folders",
    title: "Filter with in:folder and status:name, then pick a suggestion to make a chip. Ctrl+F focuses the search, Ctrl+K searches files.",
    role: "combobox",
    "aria-expanded": "false",
    "aria-controls": "search-results",
    "aria-haspopup": "listbox",
    "aria-autocomplete": "list",
  });
  const field = el("div", { class: "search-field" }, input, guiIcon("search"));
  const list = el("ul", { class: "search-results", id: "search-results", role: "listbox", "aria-label": "Search results" });
  list.hidden = true;
  const live = el("div", { role: "status", "aria-live": "polite" });
  // Screen-reader-only: the visible messages sit in the list, outside the option set.
  Object.assign(live.style, { position: "absolute", width: "1px", height: "1px", overflow: "hidden", clip: "rect(0 0 0 0)", whiteSpace: "nowrap" });
  const element = el("div", { class: "search-box collapsed" }, field, list, live);

  const chips: Chip[] = [];
  let mode: SearchMode = "folders";
  let focused = false;
  // Database matches come first in hits, then the rest of the storage from the slow pass.
  let hits: string[][] = [];
  let dbCount = 0;
  let restPending = false;
  let items: Item[] = [];
  // Index of the first slow-pass item, where the divider goes; -1 when there is none.
  let divider = -1;
  let lastSuggest: { key: string; prefix: string; found: Suggestion[] } | null = null;
  let active = 0;
  let latest = 0;
  let timer: number | undefined;
  let restTimer: number | undefined;

  // The input stays focusable while collapsed so Tab and Ctrl+F reach it and expand the box.
  const sync = () => element.classList.toggle("collapsed", !focused && input.value === "" && chips.length === 0);
  const redraw = () => {
    renderChips(field, input, chips, () => {
      sync();
      void run();
    });
    sync();
  };

  function close(): void {
    window.clearTimeout(restTimer);
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
    const rows = items.flatMap((item, i) => {
      const icon = item.icon ? [item.icon] : item.parts ? [hitIcon(item.parts)] : [];
      const li = el("li", { class: "search-hit", role: "option", id: `search-hit-${i}` }, ...icon, el("span", { class: "hit-name" }, item.label), el("span", { class: "hit-path" }, item.hint));
      if (item.parts) li.dataset.key = item.parts.join("/");
      li.addEventListener("click", item.pick);
      return i === divider ? [el("li", { class: "search-divider", role: "separator" }), li] : [li];
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

  function pickSuggestion(key: string, before: string, s: Suggestion): void {
    chips.push({ key, value: s.value, label: s.label });
    input.value = before;
    lastSuggest = null;
    redraw();
    input.focus();
    void run();
  }

  function setMode(next: SearchMode): void {
    mode = next;
    input.placeholder = MODES[next].placeholder;
    input.setAttribute("aria-label", MODES[next].label);
    element.classList.toggle("files-mode", next === "files");
  }

  const queryText = (): string => [...chips.map(chipQuery), input.value.trim()].filter(Boolean).join(" ");

  async function suggest(key: string, before: string, partial: string, mine: number): Promise<boolean> {
    const found = await suggestSearch(key, partial);
    if (mine !== latest) return true;
    lastSuggest = { key, prefix: partial, found };
    if (found.length === 0) return false;
    items = found.map((s) => ({ label: s.label, hint: s.hint, pick: () => pickSuggestion(key, before, s) }));
    divider = -1;
    show();
    return true;
  }

  // Database matches first, then the rest of the storage below a divider, each capped at LIMIT.
  function renderHits(): void {
    const dbShown = Math.min(dbCount, LIMIT);
    const restShown = Math.min(hits.length - dbCount, LIMIT);
    const picks = hits.map((_, at) => at).filter((at) => at < dbShown || (at >= dbCount && at < dbCount + restShown));
    items = picks.map((at) => {
      const parts = hits[at] ?? [];
      return { label: parts[parts.length - 1] ?? "", hint: parts.slice(0, -1).join(" › "), parts, pick: () => void open(at) };
    });
    const notes: string[] = [];
    if (dbCount > LIMIT) notes.push(`${dbCount - LIMIT} more, narrow the search`);
    if (restPending) notes.push("Searching the rest of the storage…");
    divider = dbShown > 0 && restShown > 0 ? dbShown : -1;
    show(notes);
  }

  async function runFiles(mine: number): Promise<void> {
    const found = await searchFiles(queryText());
    if (mine !== latest) return;
    items = found.slice(0, LIMIT).map((hit: FileHit) => ({
      label: hit.name,
      hint: hit.folder.join(" › "),
      icon: hitIcon(hit.folder),
      pick: () => void openFile(hit),
    }));
    divider = -1;
    show(found.length > LIMIT ? [`${found.length - LIMIT} more, narrow the search`] : []);
  }

  async function searchRest(): Promise<void> {
    if (!focused || list.hidden) return;
    const mine = latest;
    restPending = true;
    renderHits();
    let rest: string[][] = [];
    try {
      rest = await searchDisk(queryText());
    } catch (e) {
      if (mine === latest) reportError("error", e);
    }
    if (mine !== latest) return;
    restPending = false;
    const known = new Set(hits.slice(0, dbCount).map((parts) => parts.join("/")));
    hits = [...hits.slice(0, dbCount), ...rest.filter((parts) => !known.has(parts.join("/")))];
    renderHits();
  }

  async function run(): Promise<void> {
    const mine = ++latest;
    window.clearTimeout(restTimer);
    restPending = false;
    const text = input.value.trim();
    if (!text && chips.length === 0) {
      close();
      return;
    }
    try {
      const token = splitToken(input.value);
      if (token && (await suggest(token.key, token.before, token.partial, mine))) return;
      if (mode === "files") return await runFiles(mine);
      const found = await searchNodes(queryText());
      if (mine !== latest) return;
      hits = found;
      dbCount = found.length;
      renderHits();
      restTimer = window.setTimeout(() => void searchRest(), REST_DELAY_MS);
    } catch (e) {
      if (mine !== latest) return;
      close();
      reportError("error", e);
    }
  }

  async function open(i: number): Promise<void> {
    const parts = hits[i];
    if (!parts) return;
    close();
    try {
      await reveal(parts);
    } catch (e) {
      reportError("error", e);
      return;
    }
    if (!findNode(parts)) setStatus("warning", "That folder is no longer there. Refresh and search again");
  }

  // The views list folders only, so a file is shown by selecting the folder that holds it.
  async function openFile(hit: FileHit): Promise<void> {
    close();
    try {
      await reveal(hit.folder);
    } catch (e) {
      reportError("error", e);
      return;
    }
    if (!findNode(hit.folder)) setStatus("warning", "That folder is no longer there. Refresh and search again");
    else setStatus("info", `"${hit.name}" is in ${hit.folder[hit.folder.length - 1] ?? "the storage"}`);
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
    // Ctrl combos must reach the global shortcuts, so Ctrl+F and Ctrl+K work from inside the box.
    if (!ev.ctrlKey && !ev.metaKey) ev.stopPropagation();
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
      redraw();
      void run();
    } else return;
    ev.preventDefault();
    if (ev.key.startsWith("Arrow")) mark();
  });
  input.addEventListener("focus", () => {
    focused = true;
    sync();
    // Data may have changed while the box was closed, so the query runs again instead of showing old hits.
    if (input.value.trim() || chips.length > 0) void run();
  });
  installSearchDrag(list, close);
  // Mousedown on a hit would blur the input first and close the list before the click lands.
  list.addEventListener("mousedown", (ev) => ev.preventDefault());
  input.addEventListener("blur", () => {
    focused = false;
    close();
    if (input.value === "" && chips.length === 0) setMode("folders");
    sync();
  });
  field.addEventListener("click", () => {
    if (element.classList.contains("collapsed")) input.focus();
  });

  return {
    element,
    focus(next) {
      const rerun = focused && next !== mode;
      setMode(next);
      input.focus();
      input.select();
      // Already focused means no focus event, so a changed mode searches again here.
      if (rerun && (input.value.trim() || chips.length > 0)) void run();
    },
  };
}
