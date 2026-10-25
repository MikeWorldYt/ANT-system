// Footer status line for errors, warnings and review-index progress.
import { el } from "./dom";

type Level = "info" | "warning" | "error";

let sink: ((level: "warning" | "error", text: string) => void) | null = null;

export const onStatus = (fn: (level: "warning" | "error", text: string) => void): void => {
  sink = fn;
};

const line = el("footer", { class: "status-line", role: "status", "aria-live": "polite" });

export function createStatusLine(): HTMLElement {
  return line;
}

export function setStatus(level: Level, text: string): void {
  line.setAttribute("aria-live", level === "error" ? "assertive" : "polite");
  line.dataset.level = level;
  line.textContent = text;
  if (level !== "info") sink?.(level, text);
}

export function clearStatus(): void {
  line.dataset.level = "";
  line.textContent = "";
}
