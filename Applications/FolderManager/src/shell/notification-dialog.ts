// Reusable modal for messages from the app, with optional extra actions beside Close.
import { el } from "../ui/dom";
import { dialogButton, openDialog } from "../ui/dialog/shell";

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

async function open({ message, actions = [] }: NotificationOptions): Promise<void> {
  const close = dialogButton("Close", "close", "primary");
  const extra = actions.slice(0, MAX_ACTIONS).map((action) => {
    const button = el("button", { type: "button", class: "btn" }, action.label);
    button.addEventListener("click", () => {
      action.run();
      dialog.close();
    });
    return button;
  });
  const text = el("p", { class: "dialog-text" }, message);
  const { dialog, closed } = openDialog({ label: text, body: [text], buttons: [...extra, close] });
  close.focus();
  await closed;
}

export function showNotificationDialog(options: NotificationOptions): Promise<void> {
  const next = queue.then(() => open(options));
  queue = next;
  return next;
}
