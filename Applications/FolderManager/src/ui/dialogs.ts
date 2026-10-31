// Modal confirm built on <dialog> for the native focus trap and Escape handling.
import { el } from "./dom";
import { guiIcon } from "./gui-icons";

// The first choice is the safest one and gets focus; Escape resolves to "later".
export function choiceDialog(title: string, message: string, choices: [value: string, label: string][]): Promise<string> {
  return new Promise((resolve) => {
    const buttons = el("div", { class: "dialog-buttons" }, ...choices.map(([value, label], i) => el("button", { type: "submit", class: i === choices.length - 1 ? "btn btn-primary" : "btn", value }, label)));
    const form = el("form", { method: "dialog" }, el("h2", { id: "choice-title" }, title), el("p", { class: "dialog-text" }, message), buttons);
    const dialog = el("dialog", { class: "confirm", "aria-labelledby": "choice-title" }, form);

    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "" ? "later" : dialog.returnValue);
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    buttons.querySelector<HTMLButtonElement>("button")?.focus();
  });
}

export function confirmDialog(title: string, message: string, confirmLabel: string): Promise<boolean> {
  return new Promise((resolve) => {
    const cancel = el("button", { type: "submit", class: "btn", value: "cancel" }, "Cancel");
    const ok = el("button", { type: "submit", class: "btn btn-danger", value: "ok" }, confirmLabel);
    const buttons = el("div", { class: "dialog-buttons" }, ok, cancel);
    const form = el("form", { method: "dialog" }, el("h2", { id: "confirm-title" }, title), el("p", {}, message), buttons);
    const dialog = el("dialog", { class: "confirm", "aria-labelledby": "confirm-title" }, form);

    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "ok");
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    cancel.focus();
  });
}

export function warningConfirmDialog(title: string, paragraphs: (string | HTMLElement)[], confirmLabel: string, confirmTone: "primary" | "danger" = "primary"): Promise<boolean> {
  return new Promise((resolve) => {
    const cancel = el("button", { type: "submit", class: "btn", value: "cancel" }, "Cancel");
    const ok = el("button", { type: "submit", class: `btn btn-${confirmTone}`, value: "ok" }, confirmLabel);
    const buttons = el("div", { class: "dialog-buttons" }, ok, cancel);
    const copy = el("div", { class: "warning-confirm-copy" }, ...paragraphs.map((paragraph) => typeof paragraph === "string" ? el("p", {}, paragraph) : paragraph));
    const form = el("form", { method: "dialog" }, el("span", { class: "warning-confirm-icon", "aria-hidden": "true" }, guiIcon("warning")), el("h2", { id: "warning-confirm-title" }, title), copy, buttons);
    const dialog = el("dialog", { class: "confirm warning-confirm", "aria-labelledby": "warning-confirm-title" }, form);

    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "ok");
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    cancel.focus();
  });
}

// Resolves to the typed text, or null when cancelled.
export function promptDialog(title: string, label: string, initial: string, confirmLabel: string): Promise<string | null> {
  return new Promise((resolve) => {
    const input = el("input", { type: "text", class: "dialog-input", "aria-label": label, value: initial });
    const cancel = el("button", { type: "submit", class: "btn", value: "cancel", formnovalidate: "" }, "Cancel");
    const ok = el("button", { type: "submit", class: "btn btn-primary", value: "ok" }, confirmLabel);
    const form = el("form", { method: "dialog" }, el("h2", { id: "prompt-title" }, title), el("p", {}, label), input, el("div", { class: "dialog-buttons" }, cancel, ok));
    const dialog = el("dialog", { class: "confirm", "aria-labelledby": "prompt-title" }, form);
    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "ok" ? input.value.trim() : null);
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    input.select();
  });
}

// Resolves to the chosen value, or null when cancelled.
export function selectDialog(title: string, message: string, options: [value: string, label: string][], confirmLabel: string): Promise<string | null> {
  return new Promise((resolve) => {
    const select = el("select", { class: "dialog-input", "aria-label": title }, ...options.map(([value, label]) => el("option", { value }, label)));
    const cancel = el("button", { type: "submit", class: "btn", value: "cancel" }, "Cancel");
    const ok = el("button", { type: "submit", class: "btn btn-danger", value: "ok" }, confirmLabel);
    const form = el("form", { method: "dialog" }, el("h2", { id: "select-title" }, title), el("p", { class: "dialog-text" }, message), select, el("div", { class: "dialog-buttons" }, cancel, ok));
    const dialog = el("dialog", { class: "confirm", "aria-labelledby": "select-title" }, form);
    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "ok" ? select.value : null);
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    select.focus();
  });
}
