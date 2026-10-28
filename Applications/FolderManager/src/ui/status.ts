// Footer status line for errors, warnings and review-index progress.
import { el } from "./dom";

type Level = "info" | "warning" | "error";

let sink: ((level: "warning" | "error", text: string) => void) | null = null;
let activeOperation: string | null = null;

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

export function beginOperationStatus(text: string): void {
  activeOperation = text;
  setStatus("info", text);
}

export function currentOperationStatus(): string | null {
  return activeOperation;
}

export function endOperationStatus(): void {
  const text = activeOperation;
  if (text === null) return;
  activeOperation = null;
  if (line.dataset.level === "info" && line.textContent.startsWith(text)) clearStatus();
}
