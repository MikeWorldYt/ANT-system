// Icon-only button shared by the toolbar, the navigation bar, the side panel and the details form.
import { el, isDisabled } from "../dom";
import { guiIcon, type GuiIcon } from "./gui-icons";

export function iconButton(label: string, icon: GuiIcon, onClick?: () => void): HTMLButtonElement {
  const b = el("button", { type: "button", class: "icon-btn", "aria-label": label, title: label }, guiIcon(icon));
  // Registered before any attachMenu, so stopping propagation also keeps a disabled menu button closed.
  b.addEventListener("click", (ev) => {
    if (isDisabled(b)) ev.stopImmediatePropagation();
    else onClick?.();
  });
  b.addEventListener("keydown", (ev) => {
    if (isDisabled(b) && ev.key === "ArrowDown") ev.stopImmediatePropagation();
  });
  return b;
}
