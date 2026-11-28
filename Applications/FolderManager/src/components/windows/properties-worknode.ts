// The Worknode Settings fieldset: the defaults a node's Outcomes folder gives every new worknode, saved together from the window's Apply button.
import { el } from "../dom";
import { selectBox } from "../atoms/select";
import { textInput } from "../atoms/text-input";
import { describeError, listChildren, setOutcomeDefault, suggestSearch } from "../../services/ipc/api";
import { STATUSES } from "../../services/store/node-info";
import type { OutcomeDefaults } from "../../services/ipc/types";
import type { Section } from "./properties-customise";

const TEMPLATES_ROOT = "ANT.TEMPLATES";
// Unset means the in-progress every new worknode already gets; "none" is the explicit empty choice.
const UNSET_STATUS = "in-progress";

type Field = "output" | "status" | "owner" | "template";
const FIELDS: Field[] = ["output", "status", "owner", "template"];
const KEYS: Record<Field, string> = { output: "defaultOutput", status: "defaultStatus", owner: "defaultOwner", template: "defaultTemplate" };

// The output always reads with one leading slash; empty stays empty.
const slashed = (raw: string): string => {
  const parts = raw.replace(/\\/g, "/").split("/").map((p) => p.trim()).filter(Boolean);
  return parts.length ? `/${parts.join("/")}` : "";
};

const row = (label: string, control: HTMLElement) => el("label", { class: "prop-setting" }, el("span", { class: "prop-setting-label" }, label), control);

export function worknodeSection(first: OutcomeDefaults, onChange: () => void, warn: (text: string) => void): Section {
  let saved = first;
  const group = el("fieldset", { class: "prop-support" }, el("legend", { class: "prop-support-title" }, "Presets for New Worknodes"));
  const output = textInput({ class: "prop-input", placeholder: "Empty" });
  const status = selectBox("Status", [["none", "Empty"], ...STATUSES], "prop-input prop-status");
  for (const option of status.options) if (option.value !== "none") option.className = `st-${option.value}`;
  const paintStatus = (): void => {
    status.className = `prop-input prop-status${status.value === "none" ? "" : ` st-${status.value}`}`;
  };
  const owner = selectBox("Owner", [["", "unknown"]], "prop-input");
  const template = selectBox("Template", [["", "None"]], "prop-input");
  // A saved owner or template that no longer exists stays listed, so the choice still shows.
  const keep = (box: HTMLSelectElement, value: string): void => {
    if (value && ![...box.options].some((o) => o.value === value)) box.append(el("option", { value }, value));
  };
  const show = (): void => {
    output.value = slashed(saved.output);
    status.value = saved.status || UNSET_STATUS;
    paintStatus();
    keep(owner, saved.owner);
    owner.value = saved.owner;
    keep(template, saved.template);
    template.value = saved.template;
  };
  show();
  const controls = { output, status, owner, template };
  for (const control of Object.values(controls)) control.addEventListener("input", onChange);
  status.addEventListener("input", paintStatus);
  output.addEventListener("change", () => {
    output.value = slashed(output.value);
  });
  group.append(row("Output", output), row("Status", status), row("Owner", owner), row("Template", template));

  void suggestSearch("owner", "")
    .then((owners) => {
      owner.append(...owners.map((o) => o.value).filter((v) => v !== saved.owner).map((v) => el("option", { value: v }, v)));
    })
    .catch((e: unknown) => warn(`Could not list the owners: ${describeError(e)}`));
  void listChildren([TEMPLATES_ROOT])
    .then((kids) => {
      template.append(...kids.filter((k) => !k.file && k.name !== saved.template).map((k) => el("option", { value: k.name }, k.name)));
    })
    .catch((e: unknown) => warn(`Could not list the templates: ${describeError(e)}`));

  const was = (f: Field): string => (f === "status" ? saved.status || UNSET_STATUS : f === "output" ? slashed(saved.output) : saved[f]);
  const now = (f: Field): string => (f === "output" ? slashed(output.value) : controls[f].value);
  const edited = (): Field[] => FIELDS.filter((f) => now(f) !== was(f));

  async function apply(): Promise<string | null> {
    group.disabled = true;
    let failure: string | null = null;
    // Only the edited defaults are written; one that fails keeps its edit and stays pending.
    for (const f of edited()) {
      try {
        await setOutcomeDefault(saved.outcomes, KEYS[f], now(f) || null);
        saved = { ...saved, [f]: now(f) };
      } catch (e) {
        failure = `Could not save the ${f} setting: ${describeError(e)}`;
        break;
      }
    }
    if (!failure) show();
    group.disabled = false;
    return failure;
  }

  return { group, changed: () => edited().length > 0, apply };
}
