// Reusable modal for messages from the app, with optional extra actions beside Close.
import { el } from "../ui/dom";

interface NotificationAction {
  label: string;
  run: () => void;
}

interface NotificationOptions {
  message: string;
  actions?: NotificationAction[];
}

// Three buttons at most: Close plus two actions.
const MAX_ACTIONS = 2;

// Calls are chained so a second dialog opens only after the first one closed.
let queue: Promise<void> = Promise.resolve();

function open({ message, actions = [] }: NotificationOptions): Promise<void> {
  return new Promise((resolve) => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const close = el("button", { type: "submit", class: "btn btn-primary", value: "close" }, "Close");
    const extra = actions.slice(0, MAX_ACTIONS).map((action) => {
      const button = el("button", { type: "button", class: "btn" }, action.label);
      button.addEventListener("click", () => {
        action.run();
        dialog.close();
      });
      return button;
    });
    const form = el(
      "form",
      { method: "dialog" },
      el("p", { id: "notification-message", class: "dialog-text" }, message),
      el("div", { class: "dialog-buttons" }, ...extra, close),
    );
    const dialog = el("dialog", { class: "confirm", role: "dialog", "aria-modal": "true", "aria-labelledby": "notification-message" }, form);

    dialog.addEventListener("close", () => {
      dialog.remove();
      previous?.focus();
      resolve();
    });
    document.body.append(dialog);
    dialog.showModal();
    close.focus();
  });
}

export function showNotificationDialog(options: NotificationOptions): Promise<void> {
  const next = queue.then(() => open(options));
  queue = next;
  return next;
}
