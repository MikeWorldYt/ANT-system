// Status field of the details form; a native select cannot show the coloured status icons.
import { el } from "../dom";
import { guiIcon } from "../atoms/gui-icons";
import { focusInitial, menuItem, openPopup, placeUnder, popupTrigger, rove } from "./menu";
import { statusIcon } from "../atoms/status-mark";
import { STATUSES, statusLabel, UNKNOWN } from "../../services/store/node-info";
import type { WkbStatus } from "../../services/ipc/types";

export interface StatusPicker {
  button: HTMLButtonElement;
  readonly value: string;
}

let pickerCount = 0;

export function statusPicker(current: WkbStatus | null, attrs: Record<string, string>): StatusPicker {
  const listId = `status-picker-${++pickerCount}`;
  const button = el("button", { type: "button", ...attrs, "aria-haspopup": "listbox", "aria-expanded": "false", "aria-controls": listId });
  let value: WkbStatus | null = current;
  let closeList: ((restore?: boolean) => void) | null = null;
  const close = (restore = true) => closeList?.(restore);

  const paint = () => {
    button.className = `details-input status-picker${value ? ` st-${value}` : ""}`;
    const face = value ? [guiIcon(statusIcon(value)), statusLabel(value)] : [UNKNOWN];
    button.replaceChildren(...face, guiIcon("chevronDown"));
  };

  function open(): void {
    const options = STATUSES.map(([v, label]) => {
      const option = menuItem(label, "option", true, () => {
        value = v;
        paint();
        close();
      }, `ctx-item ctx-status st-${v}`, v === value);
      option.setAttribute("aria-selected", String(v === value));
      option.replaceChildren(guiIcon(statusIcon(v)), label);
      return option;
    });
    // Nodes without a status show it as an unpickable first entry, like the old select did.
    const unknown = current ? [] : [el("div", { class: "ctx-item", role: "option", "aria-disabled": "true", "aria-selected": String(value === null) }, UNKNOWN)];
    const labelledBy = attrs["aria-labelledby"];
    const list = el("div", { id: listId, class: "ctx-menu", role: "listbox", ...(labelledBy ? { "aria-labelledby": labelledBy } : {}) }, ...unknown, ...options);

    list.addEventListener("keydown", (ev) => {
      if (ev.key === "Escape" || ev.key === "Tab") {
        close();
        ev.preventDefault();
        ev.stopPropagation();
      } else rove(options, ev);
    });
    // Keeps a press on the list padding from blurring the focused option.
    list.addEventListener("mousedown", (ev) => ev.preventDefault());

    const r = button.getBoundingClientRect();
    list.style.minWidth = `${r.width}px`;
    closeList = openPopup(button, list, () => placeUnder(list, r), () => (closeList = null));
    focusInitial(options);
  }

  popupTrigger(button, ["ArrowDown", "ArrowUp"], () => closeList, open);
  paint();
  return {
    button,
    get value() {
      return value ?? "";
    },
  };
}
