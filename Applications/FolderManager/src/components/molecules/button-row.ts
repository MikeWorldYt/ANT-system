// Right-aligned group of buttons at the end of a settings row.
import { el } from "../dom";

export const actions = (...buttons: HTMLElement[]): HTMLElement => el("div", { class: "sw-actions" }, ...buttons);
