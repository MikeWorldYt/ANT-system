// Labelled action row of the side panel, with its icon and keyboard hint.
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";

export interface SideAction {
  button: HTMLButtonElement;
  setLabel(text: string): void;
}

export function sideAction(label: string, icon: GuiIcon | null, run: () => void, tone = "", hint?: [shown: string, keys: string]): SideAction {
  const text = el("span", { class: "side-action-label" }, label);
  const slot = el("span", { class: "side-action-icon" }, ...(icon ? [guiIcon(icon)] : []));
  const button = el("button", { type: "button", class: `side-action ${tone}`.trim() }, text, ...(icon ? [slot] : []));
  if (hint) {
    button.setAttribute("aria-keyshortcuts", hint[1]);
    button.append(el("kbd", { class: "side-action-hint", "aria-hidden": "true" }, hint[0]));
  }
  // aria-disabled keeps the row focusable, so the click is ignored here instead.
  button.addEventListener("click", () => {
    if (button.getAttribute("aria-disabled") !== "true") run();
  });
  return { button, setLabel: (t) => (text.textContent = t) };
}

export const setDisabled = (b: HTMLButtonElement, disabled: boolean) => b.setAttribute("aria-disabled", String(disabled));
