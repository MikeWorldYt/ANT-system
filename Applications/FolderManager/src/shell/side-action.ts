// Labelled action row of the side panel, with its icon.
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";

export interface SideAction {
  button: HTMLButtonElement;
  setLabel(text: string): void;
}

export function sideAction(label: string, icon: GuiIcon | null, run: () => void, tone = ""): SideAction {
  const text = el("span", { class: "side-action-label" }, label);
  const slot = el("span", { class: "side-action-icon" }, ...(icon ? [guiIcon(icon)] : []));
  const button = el("button", { type: "button", class: `side-action ${tone}`.trim() }, text, ...(icon ? [slot] : []));
  // aria-disabled keeps the row focusable, so the click is ignored here instead.
  button.addEventListener("click", () => {
    if (button.getAttribute("aria-disabled") !== "true") run();
  });
  return { button, setLabel: (t) => (text.textContent = t) };
}

export const setDisabled = (b: HTMLButtonElement, disabled: boolean) => b.setAttribute("aria-disabled", String(disabled));
