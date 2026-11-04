// Action dialogs with a single field: a typed name or a picked option.
import { el } from "../../../dom";
import { dialogButton } from "../../../atoms/button";
import { selectBox } from "../../../atoms/select";
import { textInput } from "../../../atoms/text-input";
import { dialogText } from "../dialog-shell";
import { fieldDialog } from "./action-dialog";

// Resolves to the typed text, or null when cancelled.
export async function promptDialog(title: string, label: string, initial: string, confirmLabel: string): Promise<string | null> {
  const input = textInput({ class: "dialog-input", "aria-label": label, value: initial }, true);
  const ok = await fieldDialog(title, el("p", {}, label), input, dialogButton(confirmLabel, "ok", "primary"), () => input.select());
  return ok ? input.value.trim() : null;
}

// Resolves to the chosen value, or null when cancelled.
export async function selectDialog(title: string, message: string, options: [value: string, label: string][], confirmLabel: string): Promise<string | null> {
  const select = selectBox(title, options, "dialog-input");
  const ok = await fieldDialog(title, dialogText(message), select, dialogButton(confirmLabel, "ok", "danger"), () => select.focus());
  return ok ? select.value : null;
}
