// On/off switch that flips only once the change it reports is accepted.
import { el } from "../dom";

export function switchControl(label: string, checked: boolean, onChange: (on: boolean) => Promise<boolean>): HTMLElement {
  const button = el("button", { type: "button", class: "sw-switch", role: "switch", "aria-label": label, "aria-checked": String(checked) });
  button.addEventListener("click", async () => {
    const next = button.getAttribute("aria-checked") !== "true";
    if (await onChange(next)) button.setAttribute("aria-checked", String(next));
  });
  return button;
}
