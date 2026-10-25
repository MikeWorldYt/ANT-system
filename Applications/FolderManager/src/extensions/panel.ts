// Collapsible "Extensions" area of the details panel; each running extension owns one section in it.
import { el } from "../ui/dom";

const list = el("div", { class: "ext-sections" });
const area = el("details", { class: "ext-area" }, el("summary", {}, "Extensions"), list);
area.hidden = true;

const sync = () => {
  area.hidden = ![...list.children].some((s) => !(s as HTMLElement).hidden);
};

export const extensionArea = (): HTMLElement => area;

export interface PanelSection {
  element: HTMLElement;
  show(on: boolean): void;
  destroy(): void;
}

// The iframe never moves in the DOM after this, because moving it would reload the extension.
export function addSection(name: string, frame: HTMLIFrameElement): PanelSection {
  const element = el("section", { class: "ext-panel", "aria-label": name }, el("h3", {}, name), frame);
  element.hidden = true;
  list.append(element);
  return {
    element,
    show(on) {
      element.hidden = !on;
      if (on) area.open = true;
      sync();
    },
    destroy() {
      element.remove();
      sync();
    },
  };
}
