// Labelled action row of the side panel, with its icon.
import { el, isDisabled } from "../dom";
import { guiIcon, type GuiIcon } from "../atoms/gui-icons";

export interface SideAction {
  button: HTMLButtonElement;
  setLabel(text: string): void;
}

export function sideAction(label: string, icon: GuiIcon | null, run: () => void, tone = ""): SideAction {
  const text = el("span", { class: "side-action-label" }, label);
  const slot = el("span", { class: "side-action-icon" }, ...(icon ? [guiIcon(icon)] : []));
  const button = el("button", { type: "button", class: `side-action ${tone}`.trim() }, text, ...(icon ? [slot] : []));
  button.addEventListener("click", () => {
    if (!isDisabled(button)) run();
  });
  return { button, setLabel: (t) => (text.textContent = t) };
}
