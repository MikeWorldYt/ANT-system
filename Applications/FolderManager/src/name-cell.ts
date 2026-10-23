// Name column of a row: plain text, or the in-place editor when this node is being renamed.
import { el } from "./dom";
import { renameTo } from "./ops";
import { keyOf, state, stopEdit } from "./state";
import type { TreeNode } from "./types";

export function nameCell(n: TreeNode): HTMLElement {
  const key = keyOf(n.relParts);
  const edit = state.editing;
  if (!edit || edit.key !== key) return el("span", { class: "name" }, n.name);

  const input = el("input", { class: "name-input", type: "text", spellcheck: "false", autocomplete: "off", "aria-label": `Rename ${n.name}` });
  input.value = edit.draft;
  const finish = (commit: boolean) => {
    if (state.editing?.key !== key) return;
    const text = input.value;
    stopEdit();
    if (commit) void renameTo(n, text);
  };
  input.addEventListener("input", () => {
    edit.draft = input.value;
  });
  input.addEventListener("keydown", (ev) => {
    ev.stopPropagation();
    if (ev.key === "Enter") finish(true);
    else if (ev.key === "Escape") finish(false);
    else return;
    ev.preventDefault();
  });
  // A re-render swaps the input and blurs the old one; the new one has focus by the time this runs.
  input.addEventListener("blur", () =>
    queueMicrotask(() => {
      if (!document.activeElement?.classList.contains("name-input")) finish(true);
    }),
  );
  for (const type of ["click", "dblclick", "pointerdown"]) input.addEventListener(type, (ev) => ev.stopPropagation());
  queueMicrotask(() => {
    input.focus();
    if (edit.focused) input.setSelectionRange(input.value.length, input.value.length);
    else input.select();
    edit.focused = true;
  });
  return el("span", { class: "name editing" }, el("span", { class: "name-fixed" }, n.fixedPrefix), input);
}
