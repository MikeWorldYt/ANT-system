// The defaults an Outcomes folder hands to every new worknode of its node; each one saves as soon as it changes.
import { el } from "../../dom";
import { selectBox } from "../../atoms/select";
import { textInput } from "../../atoms/text-input";
import { setOutcomeDefault } from "../../../hooks/ops";
import { listChildren, suggestSearch } from "../../../services/ipc/api";
import { STATUSES } from "../../../services/store/node-info";
import { keyOf, selectedNode, state } from "../../../services/store/state";
import { reportError } from "./status-line";
import type { TreeNode } from "../../../services/ipc/types";

const fieldOf = (n: TreeNode, key: string): string => n.info?.fields.find((f) => f.key === key)?.value ?? "";
const TEMPLATES_ROOT = "ANT.TEMPLATES";
const row = (id: string, label: string, control: HTMLElement) => el("div", { class: "detail-field" }, el("dt", { id }, label), el("dd", {}, control));

export function createAutomationsPanel(): { element: HTMLElement; render(): void } {
  const list = el("dl", { class: "details" });
  const element = el("section", { class: "side-section", "aria-labelledby": "automations-title" }, el("h3", { id: "automations-title", class: "side-label" }, "Automations"), list);
  let built: string | null = null;

  function build(n: TreeNode): void {
    const output = textInput({ class: "details-input", "aria-labelledby": "auto-output", placeholder: "Empty" });
    output.value = fieldOf(n, "defaultOutput");
    const saveOutput = () => {
      if (output.value.trim() !== fieldOf(selectedNode() ?? n, "defaultOutput")) void setOutcomeDefault(n, "defaultOutput", output.value);
    };
    output.addEventListener("change", saveOutput);
    output.addEventListener("keydown", (ev) => ev.key === "Enter" && saveOutput());

    // Unset means the in-progress every new worknode already gets; "none" is the explicit empty choice.
    const status = selectBox("Default status", [["none", "Empty"], ...STATUSES], "details-input");
    status.setAttribute("aria-labelledby", "auto-status");
    status.value = fieldOf(n, "defaultStatus") || "in-progress";
    status.addEventListener("change", () => void setOutcomeDefault(n, "defaultStatus", status.value));

    const current = fieldOf(n, "defaultOwner");
    const owner = selectBox("Default owner", [["", "unknown"], ...(current ? [[current, current] as [string, string]] : [])], "details-input");
    owner.setAttribute("aria-labelledby", "auto-owner");
    owner.value = current;
    owner.addEventListener("change", () => void setOutcomeDefault(n, "defaultOwner", owner.value || null));
    void suggestSearch("owner", "")
      .then((owners) => {
        const names = owners.map((o) => o.value).filter((v) => v !== current);
        owner.append(...names.map((v) => el("option", { value: v }, v)));
      })
      .catch((e: unknown) => reportError("warning", e));

    // Each folder of ANT.TEMPLATES is one template; a saved one whose folder is gone stays listed so the choice shows.
    const chosen = fieldOf(n, "defaultTemplate");
    const template = selectBox("Default template", [["", "None"], ...(chosen ? [[chosen, chosen] as [string, string]] : [])], "details-input");
    template.setAttribute("aria-labelledby", "auto-template");
    template.value = chosen;
    template.addEventListener("change", () => void setOutcomeDefault(n, "defaultTemplate", template.value || null));
    const home = state.tree?.nodes.find((t) => t.name.toUpperCase() === TEMPLATES_ROOT);
    if (home)
      void listChildren(home.relParts)
        .then((kids) => template.append(...kids.filter((k) => !k.file && k.name !== chosen).map((k) => el("option", { value: k.name }, k.name))))
        .catch((e: unknown) => reportError("warning", e));

    list.replaceChildren(row("auto-output", "Default output", output), row("auto-status", "Default status", status), row("auto-owner", "Default owner", owner), row("auto-template", "Default template", template));
  }

  function render(): void {
    const n = selectedNode();
    const show = n?.nodeType?.base === "OUT" && !n.ghost;
    element.hidden = !show;
    if (!show) {
      built = null;
      return;
    }
    // Rebuilt only when the folder or its saved values change, so a field being typed in keeps its text.
    const key = `${keyOf(n.relParts)}|${fieldOf(n, "defaultOutput")}|${fieldOf(n, "defaultStatus")}|${fieldOf(n, "defaultOwner")}|${fieldOf(n, "defaultTemplate")}`;
    if (key === built) return;
    built = key;
    build(n);
  }

  return { element, render };
}
