// Modal confirm built on <dialog> for the native focus trap and Escape handling.
import { el } from "./dom";

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
