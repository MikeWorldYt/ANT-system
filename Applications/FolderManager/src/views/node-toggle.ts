// Icon targets in a row that open or fold a folder, kept apart from the row's select and rename gestures.
import { el } from "../ui/dom";

// Not a tab stop: the focused row keeps the keyboard and its keys do what the target does.
export function nodeToggle(label: string, expanded: boolean | null, ...kids: Node[]): HTMLElement {
  const toggle = el("span", { class: "node-toggle", role: "button", "aria-label": label }, ...kids);
  if (expanded !== null) toggle.setAttribute("aria-expanded", String(expanded));
  return toggle;
}

export const fromToggle = (ev: Event): boolean => ev.target instanceof Element && ev.target.closest(".node-toggle") !== null;
