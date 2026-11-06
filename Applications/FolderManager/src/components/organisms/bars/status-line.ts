// Footer status line for errors, warnings and review-index progress.
import { describeError } from "../../../services/ipc/api";
import { el } from "../../dom";
import { state, subscribe } from "../../../services/store/state";

type Level = "info" | "warning" | "error";

let sink: ((level: "warning" | "error", text: string) => void) | null = null;
let activeOperation: string | null = null;

export const onStatus = (fn: (level: "warning" | "error", text: string) => void): void => {
  sink = fn;
};

const line = el("span", { class: "status-text", role: "status", "aria-live": "polite" });
const count = el("span", { class: "status-count" });

// Only a group of marked items is counted; a single selection is already visible.
export function createStatusLine(): HTMLElement {
  subscribe(() => {
    const n = state.selectedKeys.length;
    count.textContent = n > 1 ? `${n} items selected` : "";
  });
  return el("footer", { class: "status-line" }, line, count);
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

// A failed call in the status line, after what was being tried when a prefix names it.
export function reportError(level: "warning" | "error", e: unknown, prefix?: string): void {
  setStatus(level, prefix ? `${prefix}: ${describeError(e)}` : describeError(e));
}
