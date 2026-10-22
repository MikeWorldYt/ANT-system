// Actions panel: edit name, type and status of the selected node; every control follows node.actions.
import { el } from "./dom";
import { nodeDetails } from "./node-info";
import { apply, editablePart, remove } from "./ops";
import { selectedNode, state } from "./state";
import type { TreeNode, WkbStatus } from "./types";

export const TYPES: [string, string][] = [["NODE", "Node"], ["WKB", "Workboard"], ["PORTF", "Portfolio"]];
const STATUSES: [WkbStatus, string][] = [
  ["in-progress", "In progress"],
  ["for-review", "For review"],
  ["hold", "Hold"],
  ["done", "Done"],
];

function field(label: string, id: string, control: HTMLElement): HTMLElement {
  return el("div", { class: "field" }, el("label", { for: id }, label), control);
}

function fill(select: HTMLSelectElement, options: [string, string][]): void {
  select.replaceChildren(...options.map(([value, text]) => el("option", { value }, text)));
}

export function createActionsPanel(): { element: HTMLElement; render(): void } {
  const name = el("input", { id: "f-name", type: "text", autocomplete: "off", spellcheck: "false" });
  const type = el("select", { id: "f-type" });
  const status = el("select", { id: "f-status" });
  const applyBtn = el("button", { type: "submit", class: "btn btn-primary" }, "Apply");
  const deleteBtn = el("button", { type: "button", class: "btn btn-danger" }, "Delete");
  const statusRow = field("Status", "f-status", status);
  statusRow.hidden = true;
  fill(status, STATUSES);

  const form = el("form", { class: "actions-form" }, field("Name", "f-name", name), field("Type", "f-type", type), statusRow, applyBtn, deleteBtn);
  const details = el("dl", { class: "details" });
  const element = el("aside", { class: "actions", "aria-labelledby": "actions-title" }, el("h2", { id: "actions-title" }, "Actions"), form, details);

  let shown = "";
  const current = (): TreeNode | null => selectedNode();

  function populate(n: TreeNode | null): void {
    name.value = n ? editablePart(n.name) : "";
    const base = n?.nodeType?.base ?? "NODE";
    const options = [...TYPES];
    if (!options.some(([v]) => v === base)) options.push([base, base]);
    fill(type, options);
    type.value = base;
    status.value = n?.status ?? "in-progress";
  }

  type.addEventListener("change", () => {
    statusRow.hidden = type.value !== "WKB";
  });
  form.addEventListener("submit", (ev) => {
    ev.preventDefault();
    const n = current();
    if (n) void apply(n, { editable: name.value, type: type.value, status: status.value as WkbStatus });
  });
  deleteBtn.addEventListener("click", () => {
    const n = current();
    if (n) void remove(n);
  });

  return {
    element,
    render() {
      const n = current();
      const signature = n ? `${n.name}|${n.nodeType?.raw}|${n.status}` : "";
      if (signature !== shown) {
        shown = signature;
        populate(n);
      }
      details.replaceChildren(...(n ? nodeDetails(n) : []).flatMap(([k, v]) => [el("dt", {}, k), el("dd", {}, v)]));
      const a = n?.actions;
      const idle = !state.busy;
      statusRow.hidden = type.value !== "WKB";
      name.disabled = !(a?.rename && idle);
      type.disabled = !(a?.changeType && idle);
      status.disabled = !(a?.changeType && idle);
      applyBtn.disabled = !((a?.rename || a?.changeType) && idle);
      deleteBtn.disabled = !(a?.delete && idle);
    },
  };
}
