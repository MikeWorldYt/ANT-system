// Footer status line for errors, warnings and review-index progress.
import { el } from "./dom";

type Level = "info" | "warning" | "error";

let line: HTMLElement;

export function createStatusLine(): HTMLElement {
  line = el("footer", { class: "status-line", role: "status", "aria-live": "polite" });
  return line;
}

export function setStatus(level: Level, text: string): void {
  line.dataset.level = level;
  line.textContent = text;
}

export function clearStatus(): void {
  line.dataset.level = "";
  line.textContent = "";
}
