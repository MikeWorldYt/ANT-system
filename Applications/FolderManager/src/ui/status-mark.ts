// Status as a colored icon; the label stays available to screen readers and as a tooltip.
import type { WkbStatus } from "../ipc/types";
import { statusLabel } from "../store/node-info";
import { el } from "./dom";
import { guiIcon, type GuiIcon } from "./gui-icons";

export const statusIcon = (status: WkbStatus): GuiIcon => ICONS[status];

const ICONS: Record<WkbStatus, GuiIcon> = {
  hold: "statusHold",
  "in-progress": "statusProgress",
  "for-review": "statusReview",
  done: "statusDone",
};

export function statusMark(status: WkbStatus): HTMLElement {
  const label = statusLabel(status);
  return el("span", { class: `status-mark st-${status}`, role: "img", "aria-label": label, title: label }, guiIcon(ICONS[status]));
}

export const statusLabeled = (status: WkbStatus): HTMLElement =>
  el("span", { class: "status-labeled" }, guiIcon(ICONS[status]), statusLabel(status));
