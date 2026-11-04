// Action dialogs with a single field: a typed name or a picked option.
import { el } from "../../../dom";
import { dialogButton } from "../../../atoms/button";
import { openDialog } from "../dialog-shell";

// Resolves to the typed text, or null when cancelled.
export async function promptDialog(title: string, label: string, initial: string, confirmLabel: string): Promise<string | null> {
  const input = el("input", { type: "text", class: "dialog-input", "aria-label": label, value: initial });
  const cancel = dialogButton("Cancel", "cancel");
  cancel.formNoValidate = true;
  const { closed } = openDialog({ title, body: [el("p", {}, label), input], buttons: [cancel, dialogButton(confirmLabel, "ok", "primary")] });
  input.select();
  return (await closed) === "ok" ? input.value.trim() : null;
}

// Resolves to the chosen value, or null when cancelled.
export async function selectDialog(title: string, message: string, options: [value: string, label: string][], confirmLabel: string): Promise<string | null> {
  const select = el("select", { class: "dialog-input", "aria-label": title }, ...options.map(([value, label]) => el("option", { value }, label)));
  const { closed } = openDialog({ title, body: [el("p", { class: "dialog-text" }, message), select], buttons: [dialogButton("Cancel", "cancel"), dialogButton(confirmLabel, "ok", "danger")] });
  select.focus();
  return (await closed) === "ok" ? select.value : null;
}
