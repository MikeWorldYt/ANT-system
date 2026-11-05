// Remembers the text field the user was in, so a window focus can put the caret back where it was.
import { isField } from "./shortcuts";

let lastField: HTMLElement | null = null;

export function trackFieldFocus(): void {
  document.addEventListener("focusin", (ev) => {
    lastField = isField(ev.target) && ev.target instanceof HTMLElement ? ev.target : null;
  });
}

// Details re-renders replace their inputs, so the field is looked up again by id when it was removed.
export function restoreFocus(fallback: () => void): void {
  const field = lastField?.isConnected ? lastField : lastField?.id ? document.getElementById(lastField.id) : null;
  if (field?.offsetParent) field.focus();
  else if (!document.querySelector("dialog[open]")) fallback();
}
