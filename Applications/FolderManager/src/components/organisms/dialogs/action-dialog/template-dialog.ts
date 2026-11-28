// Asks what a new worknode template is called: its identifier, its title and an optional description.
import { el } from "../../../dom";
import { dialogButton } from "../../../atoms/button";
import { selectBox } from "../../../atoms/select";
import { textInput } from "../../../atoms/text-input";
import { dialogText } from "../dialog-shell";
import { fieldDialog } from "./action-dialog";

const IDENTIFIERS: [string, string][] = [["PJ", "PJ. Project"], ["PR", "PR. Production"], ["HW", "HW. Homework"], ["CR", "CR. Course"], ["TK", "TK. Task"], ["TH", "TH. Thesis"]];

export interface TemplateAnswer {
  identifier: string;
  name: string;
  description: string | null;
}

export async function templateDialog(): Promise<TemplateAnswer | null> {
  const identifier = selectBox("Identifier", IDENTIFIERS, "dialog-input");
  const name = textInput({ class: "dialog-input", "aria-label": "Template name", placeholder: "e.g. Video Short", required: "" }, true);
  const description = el("textarea", { class: "dialog-input", "aria-label": "Description (optional)", rows: "3", placeholder: "Optional" }) as HTMLTextAreaElement;
  const label = (text: string, field: HTMLElement) => el("label", { class: "dialog-field" }, el("span", {}, text), field);
  const fields = el("div", { class: "dialog-fields" }, label("Identifier", identifier), label("Name", name), label("Description (optional)", description));
  const ok = await fieldDialog("New template", dialogText("New worknodes made from it take its identifier and a copy of its contents."), fields, dialogButton("Create", "ok", "primary"), () => name.focus());
  if (!ok || name.value.trim() === "") return null;
  return { identifier: identifier.value, name: name.value.trim(), description: description.value.trim() || null };
}
