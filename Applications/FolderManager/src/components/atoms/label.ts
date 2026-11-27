// Read-only value and path labels shown at the end of a settings row.
import { el } from "../dom";

export const valueControl = (text: string): HTMLElement => el("span", { class: "sw-value" }, text);

// The isolated LTR run keeps the path readable while the RTL box clips and ellipsizes its start.
export const pathControl = (path: string): HTMLElement => el("span", { class: "sw-path", title: path }, el("bdi", { dir: "ltr" }, path));
