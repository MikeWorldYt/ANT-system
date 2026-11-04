// Name column of a row: plain text, or the in-place editor when this node is being renamed.
import { el } from "../dom";
import { guiIcon } from "../atoms/gui-icons";
import { renameTo, setLinkHidden } from "../../hooks/ops";
import { keyOf, state, stopEdit } from "../../services/store/state";
import type { TreeNode } from "../../services/ipc/types";
import { displayNodeName, isTopLink, linkDashes } from "../../services/store/node-info";

export const favoriteMark = () => el("span", { class: "fav-mark", title: "Favorite" }, guiIcon("starred"));

export const importantMark = () => el("span", { class: "imp-mark", title: "Important" }, guiIcon("important"), el("span", { class: "sr-only" }, "Important"));

// Checked means the link is hidden; the box sits beside the icon and keeps its clicks from selecting the row.
export function hideBox(n: TreeNode): HTMLElement[] {
  if (!state.hidingLinks || !n.linked) return [];
  const box = el("input", { type: "checkbox", class: "hide-box", "aria-label": `Hide ${n.name}` });
  box.checked = n.hidden;
  for (const type of ["click", "dblclick", "pointerdown"]) box.addEventListener(type, (ev) => ev.stopPropagation());
  box.addEventListener("change", () => void setLinkHidden(n, box.checked));
  return [box];
}

// `staircase` adds the review-link dashes; the grid leaves them out because they read badly in tiles.
export function nameCell(n: TreeNode, inlineStar = true, staircase = false): HTMLElement {
  const key = keyOf(n.relParts);
  const edit = state.editing;
  if (!edit || edit.key !== key) {
    const text = (staircase ? linkDashes(n) : "") + displayNodeName(n);
    return el("span", { class: isTopLink(n) ? "name top-link" : "name" }, el("span", { class: "name-text" }, text), ...(n.favorite && inlineStar ? [favoriteMark()] : []), ...(n.important && inlineStar ? [importantMark()] : []));
  }

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
