// Details of the selected node; its name, and the status and owner of a workboard, are edited here.
import { el } from "../../dom";
import { textButton } from "../../atoms/button";
import { checkbox } from "../../atoms/checkbox";
import { iconButton } from "../../atoms/icon-button";
import { textInput } from "../../atoms/text-input";
import { statusLabeled } from "../../atoms/status-mark";
import { isWorkboard, nodeDetails, outputOf, ownerOf, UNKNOWN } from "../../../services/store/node-info";
import { renameTo, setDetails, setOutput, setUserFlag } from "../../../hooks/ops";
import { pickInside } from "../../../hooks/outputs";
import { setStatus } from "./status-line";
import { extensionArea } from "../../../extensions/panel";
import { detailFields, onDetailsChange, runDetailsAction } from "../../../extensions/details";
import { keyOf, selectedNode } from "../../../services/store/state";
import { statusPicker, type StatusPicker } from "../../molecules/status-picker";
import type { TreeNode, WkbStatus } from "../../../services/ipc/types";

// Status and owner live in info.json, so a broken file blocks them but not the rename.
const canEditFields = (n: TreeNode) => isWorkboard(n) && !n.ghost && n.actions.changeType && !n.info?.error;
export const canEdit = (n: TreeNode) => n.actions.editDetails || n.actions.editFlags || n.actions.rename;
const termId = (label: string) => `details-term-${label.toLowerCase()}`;
const fieldId = (label: string) => `details-field-${label.toLowerCase()}`;
const labelled = (label: string) => ({ id: fieldId(label), "aria-labelledby": termId(label) });
const detailField = (...children: HTMLElement[]) => el("div", { class: "detail-field" }, ...children);
// A break is allowed only between folders, so each folder stays whole and the line fills before it wraps.
const pathSegments = (path: string): (HTMLElement | string)[] =>
  path.split("/").flatMap((folder, i, all) => (i < all.length - 1 ? [el("span", { class: "seg" }, `${folder}/`), "​"] : [el("span", { class: "seg" }, folder)]));

export interface DetailsSection {
  element: HTMLElement;
  render(): void;
  startEdit(): boolean;
}

export function createDetailsPanel(onEditEnd: () => void): DetailsSection {
  const list = el("dl", { class: "details" });
  const edit = iconButton("Edit details", "pencil", () => startEdit());
  const head = el("div", { class: "details-head" }, el("h3", { id: "details-title", class: "side-label" }, "Details"), edit);
  const empty = el("p", { class: "details-empty" }, "Select a folder to see its details.");
  const element = el("section", { class: "side-section", "aria-labelledby": "details-title" }, head, list, empty, extensionArea());
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
    let status: StatusPicker | null = null;
    let owner: HTMLInputElement | null = null;
    let output: HTMLInputElement | null = null;
    let files: HTMLInputElement | null = null;

    if (n.actions.rename) {
      name = textInput({ class: "details-input", ...labelled("Name") });
      name.value = n.editableName;
      inputs.Name = el("div", { class: "details-name" }, el("span", { class: "name-fixed" }, n.fixedPrefix), name);
      first = name;
    }
    if (n.actions.editFlags) {
      files = checkbox(labelled("Allow files"));
      files.checked = n.flags.includes("data");
      inputs["Allow files"] = files;
      first ??= files;
    }
    if (canEditFields(n)) {
      status = statusPicker(n.status ?? null, labelled("Status"));
      owner = textInput({ class: "details-input", ...labelled("Owner"), placeholder: UNKNOWN });
      owner.value = ownerOf(n) ?? "";
      inputs.Status = status.button;
      inputs.Owner = owner;
      first ??= owner;
      const outputField = textInput({ class: "details-input", ...labelled("Output"), placeholder: UNKNOWN });
      outputField.value = outputOf(n) ?? "";
      const pick = iconButton("Choose the output folder", "link", async () => {
        const picked = await pickInside(n, true);
        if (!picked) return;
        if (picked.relative === null) setStatus("warning", "Pick a folder inside the workboard");
        else outputField.value = picked.relative;
      });
      output = outputField;
      inputs.Output = el("div", { class: "details-name" }, outputField, pick);
    }

    const cancel = textButton("Cancel", undefined, stop);
    const save = textButton("Save", "btn-primary", async () => {
      // The store may have refreshed the node since the form was built.
      const current = selectedNode();
      stop();
      if (!current) return;
      if (files && files.checked !== current.flags.includes("data") && !(await setUserFlag(current, files.checked))) return;
      if (output && output.value.trim() !== (outputOf(current) ?? "") && !(await setOutput(current, output.value.trim() || null))) return;
      const fieldsChanged = (status?.value && status.value !== current.status) || (owner && owner.value.trim() !== (ownerOf(current) ?? ""));
      if (fieldsChanged && !(await setDetails(current, (status?.value || null) as WkbStatus | null, owner?.value ?? null))) return;
      // A details save re-selects the node, so the rename works on its fresh copy.
      const fresh = selectedNode();
      if (name && fresh && name.value.trim() !== fresh.editableName) await renameTo(fresh, name.value);
    });
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") stop();
      else if (ev.key === "Enter" && ev.target !== status?.button) save.click();
    };
    const fields: (HTMLElement | null)[] = [name, status?.button ?? null, owner, output, files];
    for (const field of fields) field?.addEventListener("keydown", onKey);
    queueMicrotask(() => first?.focus());
    inputs[""] = el("div", { class: "details-actions" }, cancel, save);
    return inputs;
  }

  function render(): void {
    const n = selectedNode();
    if (!n || (editing !== null && (editing !== keyOf(n.relParts) || !canEdit(n)))) editing = null;
    edit.hidden = !n || !canEdit(n);
    empty.hidden = n !== null;
    list.hidden = n === null;
    const ended = editing === null && form !== null;
    if (editing === null) form = null;
    else if (n && form?.key !== editing) form = { key: editing, inputs: controls(n) };
    const inputs = form?.inputs ?? {};
    const rows = (n ? nodeDetails(n) : []).map(([k, v]) =>
      k === "Path"
        ? detailField(el("dt", { id: termId(k) }, k), el("dd", { class: "path" }, ...pathSegments(v)))
        : detailField(el("dt", { id: termId(k) }, k), el("dd", k === "Status" && n?.status ? { class: `st-${n.status}` } : {}, inputs[k] ?? (k === "Status" && n?.status ? statusLabeled(n.status) : v))),
    );
    if (inputs["Allow files"]) rows.push(detailField(el("dt", { id: termId("Allow files") }, "Allow files"), el("dd", {}, inputs["Allow files"])));
    if (inputs[""]) rows.push(detailField(el("dd", {}, inputs[""])));
    for (const ext of detailFields(n)) {
      for (const f of ext.fields) {
        const term = el("dt", {}, f.label, el("span", { class: "ext-source" },));
        const value = el("dd", {}, f.value);
        if (f.action && n) {
          const { id } = f.action;
          value.append(textButton(f.action.label, "ext-action", () => runDetailsAction(ext.extId, id, n)));
        }
        rows.push(detailField(term, value));
      }
    }
    // Moving a field into the fresh rows blurs it, so focus goes back to it.
    const focused = document.activeElement;
    list.replaceChildren(...rows);
    if (focused instanceof HTMLElement && focused !== document.activeElement && list.contains(focused)) focused.focus();
    if (ended) onEditEnd();
  }

  function startEdit(): boolean {
    const n = selectedNode();
    if (!n || !canEdit(n)) return false;
    editing = keyOf(n.relParts);
    render();
    return true;
  }

  onDetailsChange(render);
  return { element, render, startEdit };
}
