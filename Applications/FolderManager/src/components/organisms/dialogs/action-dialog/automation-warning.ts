// Switching an automation on changes every file that lands later, with no undo, so the first time asks first.
import { el } from "../../../dom";
import { cancelButton, dialogButton } from "../../../atoms/button";
import { checkbox } from "../../../atoms/checkbox";
import { openDialog } from "../dialog-shell";
import { readPref, savePref } from "../../../../services/store/prefs";

const SKIP_KEY = "fm.automationWarningSkipped";

export async function confirmAutomation(): Promise<boolean> {
  if (readPref(SKIP_KEY) === "1") return true;
  const skip = checkbox({ id: "automation-warning-skip" });
  const body = [
    el("p", { class: "dialog-text" }, "When select this, active a rule runs by itself:"),
    el("p", { class: "dialog-text" }, "From now on, every file that lands in this folder gets its effect ", el("strong", {}, "automatically"), ", and ", el("u", {}, "it cannot be undone"), "."),
    el("p", { class: "dialog-text" }, "Switch on and ", el("strong", {}, "verify only the format and rules you want for,"), el("br"), "and keep out of this folder any file that should not be changed with that way"),
    el("label", { class: "dialog-check", for: "automation-warning-skip" }, skip, "Don't show again"),
  ];
  const ok = (await openDialog({ title: "Automatically run", tone: "warning", body, buttons: [cancelButton(), dialogButton("Accept and continue", "ok", "primary")] }).closed) === "ok";
  if (ok && skip.checked) savePref(SKIP_KEY, "1");
  return ok;
}
