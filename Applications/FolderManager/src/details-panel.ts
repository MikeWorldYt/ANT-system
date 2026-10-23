// Details of the selected node; status and owner of a workboard are edited here and saved to its info.json.
import { el } from "./dom";
import { guiIcon } from "./gui-icons";
import { isWorkboard, nodeDetails, ownerOf, STATUSES, UNKNOWN } from "./node-info";
import { setDetails } from "./ops";
import { keyOf, selectedNode } from "./state";
import type { TreeNode, WkbStatus } from "./types";

const canEdit = (n: TreeNode) => isWorkboard(n) && !n.ghost && n.actions.changeType && !n.info?.error;

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
    const status = el("select", { class: "details-input", "aria-label": "Status" });
    if (!n.status) status.append(el("option", { value: "", disabled: "", selected: "" }, UNKNOWN));
    const tint = () => {
      status.className = `details-input${status.value ? ` st-${status.value}` : ""}`;
    };
    status.addEventListener("change", tint);
    for (const [value, label] of STATUSES) {
      const option = el("option", { value }, label);
      option.selected = value === n.status;
      status.append(option);
    }
    tint();
    const owner = el("input", { class: "details-input", type: "text", "aria-label": "Owner", placeholder: UNKNOWN, spellcheck: "false", autocomplete: "off" });
    owner.value = ownerOf(n) ?? "";
    const save = el("button", { type: "button", class: "btn btn-primary" }, "Save");
    const cancel = el("button", { type: "button", class: "btn" }, "Cancel");
    save.addEventListener("click", () => {
      stop();
      void setDetails(n, (status.value || null) as WkbStatus | null, owner.value);
    });
    cancel.addEventListener("click", stop);
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") stop();
      else if (ev.key === "Enter") save.click();
    };
    status.addEventListener("keydown", onKey);
    owner.addEventListener("keydown", onKey);
    queueMicrotask(() => owner.focus());
    return { Status: status, Owner: owner, "": el("div", { class: "details-actions" }, save, cancel) };
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
