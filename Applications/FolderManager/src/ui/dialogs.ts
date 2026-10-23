// Modal confirm built on <dialog> for the native focus trap and Escape handling.
import { el } from "./dom";

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
    const buttons = el("div", { class: "dialog-buttons" }, cancel, ok);
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
