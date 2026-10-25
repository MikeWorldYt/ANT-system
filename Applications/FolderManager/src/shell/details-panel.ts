// Details of the selected node; its name, and the status and owner of a workboard, are edited here.
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { statusLabeled } from "../ui/status-mark";
import { isWorkboard, nodeDetails, ownerOf, STATUSES, UNKNOWN } from "../store/node-info";
import { renameTo, setDetails } from "../actions/ops";
import { extensionArea } from "../extensions/panel";
import { detailFields, onDetailsChange, runDetailsAction } from "../extensions/details";
import { keyOf, selectedNode } from "../store/state";
import type { TreeNode, WkbStatus } from "../ipc/types";

// Status and owner live in info.json, so a broken file blocks them but not the rename.
const canEditFields = (n: TreeNode) => isWorkboard(n) && !n.ghost && n.actions.changeType && !n.info?.error;
const canEdit = (n: TreeNode) => n.actions.editDetails;
const termId = (label: string) => `details-term-${label.toLowerCase()}`;
const fieldId = (label: string) => `details-field-${label.toLowerCase()}`;
const labelled = (label: string) => ({ id: fieldId(label), "aria-labelledby": termId(label) });

export function createDetailsPanel(): { element: HTMLElement; render(): void } {
  const list = el("dl", { class: "details" });
  const edit = el("button", { type: "button", class: "icon-btn", "aria-label": "Edit details", title: "Edit details" }, guiIcon("pencil"));
  const head = el("div", { class: "details-head" }, el("h2", { id: "details-title" }, "Details"), edit);
  const empty = el("p", { class: "details-empty" }, "Select a folder to see its details.");
  const element = el("aside", { class: "details-panel", "aria-labelledby": "details-title" }, head, list, empty, extensionArea());
  let editing: string | null = null;
  // Built once per editing key, so store notifications do not discard what was typed.
  let form: { key: string; inputs: Record<string, Node> } | null = null;

  const stop = () => {
    editing = null;
    render();
    if (!edit.hidden) edit.focus();
  };

  function controls(n: TreeNode): Record<string, Node> {
    const inputs: Record<string, Node> = {};
    let first: HTMLElement | null = null;
    let name: HTMLInputElement | null = null;
    let status: HTMLSelectElement | null = null;
    let owner: HTMLInputElement | null = null;

    if (n.actions.rename) {
      name = el("input", { class: "details-input", type: "text", ...labelled("Name"), spellcheck: "false", autocomplete: "off" });
      name.value = n.editableName;
      inputs.Name = el("div", { class: "details-name" }, el("span", { class: "name-fixed" }, n.fixedPrefix), name);
      first = name;
    }
    if (canEditFields(n)) {
      const select = el("select", { class: "details-input", ...labelled("Status") });
      if (!n.status) select.append(el("option", { value: "", disabled: "", selected: "" }, UNKNOWN));
      const tint = () => {
        select.className = `details-input${select.value ? ` st-${select.value}` : ""}`;
      };
      select.addEventListener("change", tint);
      for (const [value, label] of STATUSES) {
        const option = el("option", { value }, label);
        option.selected = value === n.status;
        select.append(option);
      }
      tint();
      owner = el("input", { class: "details-input", type: "text", ...labelled("Owner"), placeholder: UNKNOWN, spellcheck: "false", autocomplete: "off" });
      owner.value = ownerOf(n) ?? "";
      status = select;
      inputs.Status = select;
      inputs.Owner = owner;
      first ??= owner;
    }

    const save = el("button", { type: "button", class: "btn btn-primary" }, "Save");
    const cancel = el("button", { type: "button", class: "btn" }, "Cancel");
    save.addEventListener("click", async () => {
      // The store may have refreshed the node since the form was built.
      const current = selectedNode();
      stop();
      if (!current) return;
      const fieldsChanged = (status?.value && status.value !== current.status) || (owner && owner.value.trim() !== (ownerOf(current) ?? ""));
      if (fieldsChanged && !(await setDetails(current, (status?.value || null) as WkbStatus | null, owner?.value ?? null))) return;
      // A details save re-selects the node, so the rename works on its fresh copy.
      const fresh = selectedNode();
      if (name && fresh && name.value.trim() !== fresh.editableName) await renameTo(fresh, name.value);
    });
    cancel.addEventListener("click", stop);
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") stop();
      else if (ev.key === "Enter" && ev.target !== status) save.click();
    };
    const fields: (HTMLElement | null)[] = [name, status, owner];
    for (const field of fields) field?.addEventListener("keydown", onKey);
    queueMicrotask(() => first?.focus());
    inputs[""] = el("div", { class: "details-actions" }, save, cancel);
    return inputs;
  }

  function render(): void {
    const n = selectedNode();
    if (!n || (editing !== null && (editing !== keyOf(n.relParts) || !canEdit(n)))) editing = null;
    edit.hidden = !n || !canEdit(n);
    empty.hidden = n !== null;
    list.hidden = n === null;
    if (editing === null) form = null;
    else if (n && form?.key !== editing) form = { key: editing, inputs: controls(n) };
    const inputs = form?.inputs ?? {};
    const rows = (n ? nodeDetails(n) : []).flatMap(([k, v]) => [el("dt", { id: termId(k) }, k), el("dd", k === "Status" && n?.status ? { class: `st-${n.status}` } : {}, inputs[k] ?? (k === "Status" && n?.status ? statusLabeled(n.status) : v))]);
    if (inputs[""]) rows.push(el("dd", {}, inputs[""]));
    for (const ext of detailFields(n)) {
      for (const f of ext.fields) {
        rows.push(el("dt", {}, f.label, el("span", { class: "ext-source" },)));
        const value = el("dd", {}, f.value);
        if (f.action && n) {
          const { id } = f.action;
          const button = el("button", { type: "button", class: "btn ext-action" }, f.action.label);
          button.addEventListener("click", () => runDetailsAction(ext.extId, id, n));
          value.append(button);
        }
        rows.push(value);
      }
    }
    // Moving a field into the fresh rows blurs it, so focus goes back to it.
    const focused = document.activeElement;
    list.replaceChildren(...rows);
    if (focused instanceof HTMLElement && focused !== document.activeElement && list.contains(focused)) focused.focus();
  }

  edit.addEventListener("click", () => {
    const n = selectedNode();
    if (!n || !canEdit(n)) return;
    editing = keyOf(n.relParts);
    render();
  });

  onDetailsChange(render);
  return { element, render };
}
