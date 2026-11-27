// Puts keyboard focus back where the user was: the text field they typed in, or the selected item of the active view.
import { isField, overlayOpen } from "./shortcuts";

let lastField: HTMLElement | null = null;
let focusView: () => void = () => undefined;

export function trackFieldFocus(focusActiveView: () => void): void {
  focusView = focusActiveView;
  document.addEventListener("focusin", (ev) => {
    lastField = isField(ev.target) && ev.target instanceof HTMLElement ? ev.target : null;
  });
}

// Focus fell to the page itself, as after a click on a view's empty background or the removal of the focused control.
const focusLost = (): boolean => {
  const at = document.activeElement;
  return at === null || at === document.body || at === document.documentElement || !at.isConnected;
};

// Never takes focus from a field, a menu, a dialog or any control the user moved to; force is for an action that just ended.
export function refocusSelection(force = false): boolean {
  if (!overlayOpen() && (force || focusLost())) focusView();
  return !focusLost();
}

// A refocus never moves the list: the user may have scrolled away from the selection on purpose.
export function focusItem(item: HTMLElement | null | undefined): void {
  item?.focus({ preventScroll: true });
}

// Details re-renders replace their inputs, so the field is looked up again by id when it was removed.
export function restoreFocus(): void {
  const field = lastField?.isConnected ? lastField : lastField?.id ? document.getElementById(lastField.id) : null;
  if (field?.offsetParent) field.focus();
  else refocusSelection();
}
