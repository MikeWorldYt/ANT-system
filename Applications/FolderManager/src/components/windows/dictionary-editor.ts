// The dictionary editor window: a node's dictionary.md with line numbers, saved as the user types and checked line by line.
import "../../styles/index.css";
import { el } from "../dom";
import { checkDictionary, describeError, readDictionary, saveDictionary } from "../../services/ipc/api";
import type { DictionaryIssue } from "../../services/ipc/types";

const SAVE_DELAY = 400;

function statusText(issues: DictionaryIssue[]): string {
  const first = issues[0];
  if (!first) return "No problems found.";
  const more = issues.length > 1 ? ` (+${issues.length - 1} more)` : "";
  return `Line ${first.line}: ${first.message}${more}`;
}

export async function mountDictionaryEditor(folder: string[]): Promise<void> {
  const app = document.querySelector<HTMLDivElement>("#app");
  if (!app) throw new Error("Missing #app mount point");
  const gutter = el("div", { class: "dict-gutter", "aria-hidden": "true" });
  const text = el("textarea", { class: "dict-text", spellcheck: "false", "aria-label": `Dictionary of /${folder.join("/")}`, wrap: "off" });
  const status = el("span", { class: "status-text", role: "status", "aria-live": "polite" });
  const list = el("ul", { class: "dict-issues", "aria-label": "Problems" });
  app.replaceChildren(
    el("main", { class: "dict-editor" }, el("h1", { class: "dict-title" }, `/${folder.join("/")}/.ant/dictionary.md`), el("div", { class: "dict-body" }, gutter, text), list, el("footer", { class: "status-line" }, status)),
  );

  let issues: DictionaryIssue[] = [];
  const setStatus = (level: "info" | "warning" | "error", message: string): void => {
    status.dataset.level = level;
    status.textContent = message;
  };

  // Moves the caret to a line, so a problem in the list is one click from its place.
  const goTo = (line: number): void => {
    const start = text.value.split("\n").slice(0, line - 1).reduce((n, l) => n + l.length + 1, 0);
    text.focus();
    text.setSelectionRange(start, start + (text.value.split("\n")[line - 1]?.length ?? 0));
    const height = parseFloat(getComputedStyle(text).lineHeight) || 18;
    text.scrollTop = Math.max(0, (line - 3) * height);
  };

  const paint = (): void => {
    const count = text.value.split("\n").length;
    const marks = new Map(issues.map((i) => [i.line, i.level]));
    gutter.replaceChildren(...Array.from({ length: count }, (_, k) => el("div", { class: `dict-line ${marks.get(k + 1) ?? ""}` }, String(k + 1))));
    gutter.scrollTop = text.scrollTop;
    list.replaceChildren(
      ...issues.map((i) => {
        const item = el("button", { type: "button", class: `dict-issue ${i.level}` }, `${i.line}: ${i.message}`);
        item.addEventListener("click", () => goTo(i.line));
        return el("li", {}, item);
      }),
    );
    const worst = issues.some((i) => i.level === "error") ? "error" : issues.length > 0 ? "warning" : "info";
    setStatus(worst, statusText(issues));
  };

  let timer: number | undefined;
  let saving = Promise.resolve();
  const save = (): void => {
    const value = text.value;
    saving = saving.then(async () => {
      try {
        issues = await saveDictionary(folder, value);
        if (text.value === value) paint();
      } catch (e) {
        setStatus("error", `Not saved: ${describeError(e)}`);
      }
    });
  };

  text.addEventListener("input", () => {
    paint();
    setStatus("info", "Saving…");
    window.clearTimeout(timer);
    timer = window.setTimeout(save, SAVE_DELAY);
  });
  text.addEventListener("scroll", () => {
    gutter.scrollTop = text.scrollTop;
  });
  // A save still waiting when the window closes runs at once, so the last keystrokes reach the file.
  window.addEventListener("beforeunload", () => {
    if (timer !== undefined) save();
  });

  try {
    text.value = await readDictionary(folder);
    // Opening only checks; the file is written once the user types, so an untouched folder gets no dictionary.
    issues = await checkDictionary(text.value);
    paint();
  } catch (e) {
    text.disabled = true;
    setStatus("error", `Could not read the dictionary: ${describeError(e)}`);
  }
}
