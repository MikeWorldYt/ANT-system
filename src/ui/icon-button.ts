// Icon-only button shared by the header toolbar and the navigation bar.
import { el } from "./dom";
import { guiIcon, type GuiIcon } from "./gui-icons";

export function iconButton(label: string, icon: GuiIcon, onClick?: () => void): HTMLButtonElement {
  const b = el("button", { type: "button", class: "icon-btn", "aria-label": label, title: label }, guiIcon(icon));
  const disabled = () => b.getAttribute("aria-disabled") === "true";
  // aria-disabled keeps the button focusable, so the click is ignored here instead.
  // Registered before any attachMenu, so stopping propagation also keeps a disabled menu button closed.
  b.addEventListener("click", (ev) => {
    if (disabled()) ev.stopImmediatePropagation();
    else onClick?.();
  });
  b.addEventListener("keydown", (ev) => {
    if (disabled() && ev.key === "ArrowDown") ev.stopImmediatePropagation();
  });
  return b;
}
