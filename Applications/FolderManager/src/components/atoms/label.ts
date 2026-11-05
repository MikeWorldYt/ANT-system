// Read-only value and path labels shown at the end of a settings row.
import { el } from "../dom";

export const valueControl = (text: string): HTMLElement => el("span", { class: "sw-value" }, text);

export const pathControl = (path: string): HTMLElement => el("span", { class: "sw-path", title: path }, path);
