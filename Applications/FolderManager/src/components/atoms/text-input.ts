// Single-line text field; most hold names and paths, so spellcheck and autocomplete stay off unless asked for.
import { el } from "../dom";

export function textInput(attrs: Record<string, string>, assisted = false): HTMLInputElement {
  return el("input", assisted ? { type: "text", ...attrs } : { type: "text", spellcheck: "false", autocomplete: "off", ...attrs });
}
