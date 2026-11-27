// One row of the reorder dialog: icon, a worknode's tracker, the new id to type, the title, and the id it had.
import { el } from "../../../dom";
import { selectBox } from "../../../atoms/select";
import { idOf, keyFor, padId, trackerOf, type IdKind } from "../../../../services/store/id-order";

const BAD_KEY = "is-bad-key";
const BAD_KEY_MS = 600;

export interface IdGroup {
  kind: IdKind;
  title: string;
  root: boolean;
  order: string[];
  // Every row's new id as digits and, for worknodes, its new tracker; both start as the current ones.
  ids: Map<string, string>;
  trackers: Map<string, string>;
  taken: Set<string>;
  choices: string[];
}

export interface RowResult {
  name: string;
  id: string;
  problem: string | null;
}

export interface RowHooks {
  onId: (name: string, digits: string) => void;
  onTracker: (name: string, tracker: string) => void;
  announce: (text: string) => void;
}

export const newTracker = (group: IdGroup, name: string): string => (group.kind === "worknode" ? (group.trackers.get(name) ?? trackerOf(name, "worknode")) : "");

export const rowKey = (group: IdGroup, name: string, digits: string): string => keyFor(newTracker(group, name), digits);

const titleOf = (name: string, kind: IdKind): string => {
  const id = idOf(name, kind);
  return name.slice(name.indexOf(id) + id.length).replace(/^[._]/, "");
};

function idInput(group: IdGroup, name: string, hooks: RowHooks): HTMLInputElement {
  const input = el("input", { class: "reorder-input", type: "text", inputmode: "numeric", maxlength: "6", "aria-label": `New id for ${name}`, value: group.ids.get(name) ?? "" });
  const reject = () => {
    input.classList.add(BAD_KEY);
    window.setTimeout(() => input.classList.remove(BAD_KEY), BAD_KEY_MS);
    hooks.announce("Ids take digits only.");
  };
  input.addEventListener("beforeinput", (ev) => {
    if (ev.data === null || /^\d*$/.test(ev.data)) return;
    ev.preventDefault();
    reject();
  });
  // Drops and autofill can skip beforeinput, so whatever still slips in is taken out.
  input.addEventListener("input", () => {
    if (/\D/.test(input.value)) {
      input.value = input.value.replace(/\D/g, "");
      reject();
    }
    hooks.onId(name, input.value);
  });
  const commit = () => {
    if (input.value === "") return;
    input.value = padId(input.value, group.kind, group.root);
    hooks.onId(name, input.value);
  };
  input.addEventListener("blur", commit);
  // Enter would submit the dialog form, so it only commits the id.
  input.addEventListener("keydown", (ev) => {
    if (ev.key !== "Enter") return;
    ev.preventDefault();
    commit();
  });
  return input;
}

function trackerSelect(group: IdGroup, name: string, hooks: RowHooks): HTMLSelectElement {
  const current = trackerOf(name, "worknode");
  const choices = group.choices.includes(current) ? group.choices : [current, ...group.choices];
  const select = selectBox(`Tracker for ${name}`, choices.map((t) => [t, t]), "reorder-tracker");
  select.value = newTracker(group, name);
  select.addEventListener("change", () => hooks.onTracker(name, select.value));
  return select;
}

export function idRow(group: IdGroup, name: string, index: number, base: string, icon: Node, hooks: RowHooks): HTMLElement {
  const problemId = `${base}-p${index}`;
  const input = idInput(group, name, hooks);
  input.setAttribute("aria-describedby", problemId);
  return el(
    "li",
    { tabindex: "0", "data-index": String(index), "data-name": name },
    icon,
    ...(group.kind === "worknode" ? [trackerSelect(group, name, hooks)] : []),
    input,
    el("span", { class: "reorder-title" }, titleOf(name, group.kind)),
    el("span", { class: "reorder-was" }),
    el("span", { class: "reorder-problem", id: problemId }),
  );
}

export function paintRow(li: HTMLElement, group: IdGroup, name: string, res: RowResult, index: number, selected: boolean): void {
  const same = res.name === name && res.problem === null;
  const retracked = newTracker(group, name) !== trackerOf(name, group.kind);
  const was = `${retracked ? trackerOf(name, group.kind) : ""}${idOf(name, group.kind)}`;
  li.classList.toggle("is-unchanged", same);
  li.classList.toggle("is-selected", selected);
  const state = res.problem ?? (same ? "unchanged" : `becomes ${res.name}`);
  li.setAttribute("aria-label", `${name}, ${selected ? "selected, " : ""}${index + 1} of ${group.order.length}, ${state}`);
  li.querySelector(".reorder-input")?.setAttribute("aria-invalid", String(res.problem !== null));
  const shown = li.querySelector(".reorder-was");
  if (shown) shown.textContent = same ? "unchanged" : `← was ${was}`;
  const problem = li.querySelector(".reorder-problem");
  if (problem) problem.textContent = res.problem ?? "";
}
