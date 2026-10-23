// Details of the selected node; its name, and the status and owner of a workboard, are edited here.
import { el } from "./dom";
import { guiIcon } from "./gui-icons";
import { isWorkboard, nodeDetails, ownerOf, STATUSES, UNKNOWN } from "./node-info";
import { renameTo, setDetails } from "./ops";
import { keyOf, selectedNode } from "./state";
import type { TreeNode, WkbStatus } from "./types";

// Status and owner live in info.json, so a broken file blocks them but not the rename.
const canEditFields = (n: TreeNode) => isWorkboard(n) && !n.ghost && n.actions.changeType && !n.info?.error;
const canEdit = (n: TreeNode) => n.actions.rename || canEditFields(n);

export function createDetailsPanel(): { element: HTMLElement; render(): void } {
  const list = el("dl", { class: "details" });
  const edit = el("button", { type: "button", class: "icon-btn", "aria-label": "Edit details", title: "Edit details" }, guiIcon("pencil"));
  const head = el("div", { class: "details-head" }, el("h2", { id: "details-title" }, "Details"), edit);
  const element = el("aside", { class: "details-panel", "aria-labelledby": "details-title" }, head, list);
  let editing: string | null = null;

  const stop = () => {
    editing = null;
    render();
  };

  function controls(n: TreeNode): Record<string, Node> {
    const inputs: Record<string, Node> = {};
    let first: HTMLElement | null = null;
    let name: HTMLInputElement | null = null;
    let status: HTMLSelectElement | null = null;
    let owner: HTMLInputElement | null = null;

    if (n.actions.rename) {
      name = el("input", { class: "details-input", type: "text", "aria-label": "Name", spellcheck: "false", autocomplete: "off" });
      name.value = n.editableName;
      inputs.Name = el("div", { class: "details-name" }, el("span", { class: "name-fixed" }, n.fixedPrefix), name);
      first = name;
    }
    if (canEditFields(n)) {
      const select = el("select", { class: "details-input", "aria-label": "Status" });
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
      owner = el("input", { class: "details-input", type: "text", "aria-label": "Owner", placeholder: UNKNOWN, spellcheck: "false", autocomplete: "off" });
      owner.value = ownerOf(n) ?? "";
      status = select;
      inputs.Status = select;
      inputs.Owner = owner;
      first ??= owner;
    }

    const save = el("button", { type: "button", class: "btn btn-primary" }, "Save");
    const cancel = el("button", { type: "button", class: "btn" }, "Cancel");
    save.addEventListener("click", async () => {
      stop();
      const fieldsChanged = (status?.value && status.value !== n.status) || (owner && owner.value.trim() !== (ownerOf(n) ?? ""));
      if (fieldsChanged) await setDetails(n, (status?.value || null) as WkbStatus | null, owner?.value ?? null);
      // A details save re-selects the node, so the rename works on its fresh copy.
      const fresh = selectedNode();
      if (name && fresh && name.value.trim() !== fresh.editableName) await renameTo(fresh, name.value);
    });
    cancel.addEventListener("click", stop);
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") stop();
      else if (ev.key === "Enter") save.click();
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
    const inputs = n && editing !== null ? controls(n) : {};
    const rows = (n ? nodeDetails(n) : []).flatMap(([k, v]) => [el("dt", {}, k), el("dd", k === "Status" && n?.status ? { class: `st-${n.status}` } : {}, inputs[k] ?? v)]);
    if (inputs[""]) rows.push(el("dd", {}, inputs[""]));
    list.replaceChildren(...rows);
  }

  edit.addEventListener("click", () => {
    const n = selectedNode();
    if (!n || !canEdit(n)) return;
    editing = keyOf(n.relParts);
    render();
  });

  return { element, render };
}
