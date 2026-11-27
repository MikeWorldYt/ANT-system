// Floating chrome over each split half: its place name with a close button, and the tab picker while the half is empty.
import { guiIcon } from "../atoms/gui-icons";
import { kindIcon } from "../atoms/icons";
import { el } from "../dom";
import { closeHalf, currentSplit, fillOther, paneLocation } from "../../hooks/split";
import { joinTab, labelOf, tabViews } from "../../hooks/tabs";
import { state } from "../../services/store/state";

const CHIP_MAX_PX = 240;

interface PickerItem {
  title: string;
  path: string;
  icon(): Element;
  pick(): void;
}

function pickerItems(): PickerItem[] {
  const root: PickerItem = { title: "Storage root", path: state.tree?.root ?? "", icon: () => guiIcon("pageStorage"), pick: () => fillOther({ virtual: null, via: null, key: null }, new Set(), { history: [], at: -1 }) };
  return [root, ...tabViews().flatMap((t, i) => (t.active || t.split ? [] : [{ title: t.label, path: t.title, icon: () => kindIcon("folder"), pick: () => joinTab(i) }]))];
}

function createHalf(onRight: boolean, host: HTMLElement) {
  const side = onRight ? "right" : "left";
  const name = el("span", { class: "pane-chip-name" });
  const close = el("button", { type: "button", class: "pane-chip-close", "aria-label": `Close ${side} half`, title: `Close ${side} half` }, el("span", { "aria-hidden": "true" }, "×"));
  close.addEventListener("click", () => closeHalf(onRight));
  const chip = el("div", { class: "pane-chip", role: "group", "aria-label": `${side} half` }, name, close);
  const heading = el("h2", { class: "pane-picker-title", id: `pane-picker-${side}` }, "Open in this half");
  const list = el("div", { class: "pane-picker-list", role: "listbox", "aria-labelledby": heading.id });
  const picker = el("section", { class: "pane-picker", "aria-label": `${side} half` }, el("div", { class: "pane-picker-box" }, heading, list));
  picker.hidden = true;
  let shown = "";

  list.addEventListener("keydown", (ev) => {
    const buttons = [...list.querySelectorAll<HTMLButtonElement>("button")];
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = ev.key === "ArrowDown" ? at + 1 : ev.key === "ArrowUp" ? at - 1 : ev.key === "Home" ? 0 : ev.key === "End" ? buttons.length - 1 : null;
    if (next === null) return;
    ev.preventDefault();
    ev.stopPropagation();
    buttons.forEach((b, i) => (b.tabIndex = i === next ? 0 : -1));
    buttons[Math.min(Math.max(next, 0), buttons.length - 1)]?.focus();
  });

  function fill(): void {
    const items = pickerItems();
    const next = JSON.stringify(items.map(({ title, path }) => [title, path]));
    if (next === shown) return;
    shown = next;
    list.replaceChildren(
      ...items.map((item, i) => {
        const tile = el("span", { class: "pane-picker-icon" }, item.icon());
        const text = el("span", { class: "pane-picker-text" }, el("span", { class: "pane-picker-name" }, item.title), el("span", { class: "pane-picker-path", title: item.path }, item.path));
        const button = el("button", { type: "button", class: "pane-picker-item", role: "option", "aria-selected": i === 0 ? "true" : "false", tabindex: i === 0 ? "0" : "-1" }, tile, text);
        button.addEventListener("click", item.pick);
        button.addEventListener("focus", () =>
          list.querySelectorAll<HTMLButtonElement>("button").forEach((b) => {
            b.tabIndex = b === button ? 0 : -1;
            b.setAttribute("aria-selected", String(b === button));
          }),
        );
        return button;
      }),
    );
  }

  return { onRight, host, chip, name, picker, fill, list };
}

export function createSplitOverlay(panes: HTMLElement, left: HTMLElement, right: HTMLElement): { render(): void } {
  const halves = [createHalf(false, left), createHalf(true, right)];
  const layer = el("div", { class: "pane-chrome" }, ...halves.flatMap((h) => [h.picker, h.chip]));
  layer.hidden = true;
  panes.append(layer);

  function position(): void {
    const split = currentSplit();
    if (!split) return;
    const area = panes.getBoundingClientRect();
    for (const h of halves) {
      const box = h.host.getBoundingClientRect();
      h.chip.hidden = box.width === 0;
      if (box.width === 0) continue;
      h.chip.style.bottom = `${area.bottom - box.bottom}px`;
      h.chip.style.right = `${area.right - box.right}px`;
      h.chip.style.maxWidth = `${Math.min(box.width, CHIP_MAX_PX)}px`;
      Object.assign(h.picker.style, { left: `${box.left - area.left}px`, top: `${box.top - area.top}px`, width: `${box.width}px`, height: `${box.height}px` });
    }
  }

  const observer = new ResizeObserver(position);
  observer.observe(panes);
  for (const h of halves) observer.observe(h.host);

  function render(): void {
    const split = currentSplit();
    layer.hidden = split === null;
    panes.classList.toggle("split", split !== null);
    if (!split) return;
    for (const h of halves) {
      const empty = split.empty && h.onRight === !split.right;
      const wasEmpty = !h.picker.hidden;
      h.picker.hidden = !empty;
      h.chip.classList.toggle("empty", empty);
      h.name.hidden = empty;
      if (empty) {
        h.fill();
        if (!wasEmpty) h.list.querySelector<HTMLElement>("button")?.focus();
      } else {
        const { key, virtual } = paneLocation(h.onRight);
        const label = labelOf({ virtual, via: null, key }, h.onRight === split.right ? state.shownNow : undefined);
        h.name.textContent = label;
        h.chip.title = label;
      }
    }
    position();
  }

  return { render };
}
