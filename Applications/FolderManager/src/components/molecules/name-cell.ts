// Name column of a row: plain text, or the in-place editor when this node is being renamed.
import { el, keepClicks } from "../dom";
import { guiIcon } from "../atoms/gui-icons";
import { hideCheckbox } from "../atoms/checkbox";
import { textInput } from "../atoms/text-input";
import { renameTo, setLinkHidden } from "../../hooks/ops";
import { keyOf, state, stopEdit } from "../../services/store/state";
import type { TreeNode } from "../../services/ipc/types";
import { inReviewList, reviewLevel, reviewName } from "../../services/store/review-list";
import { displayNodeName, isTopLink, linkDashes } from "../../services/store/node-info";

export const favoriteMark = () => el("span", { class: "fav-mark", title: "Favorite" }, guiIcon("starred"));

// A review folder that changes every file landing in it says so in the tree, not only in its panel.
const autoMark = () => el("span", { class: "auto-mark", title: "Automations run on new files" }, guiIcon("play"), el("span", { class: "sr-only" }, "Automations run on new files"));
const runsAuto = (n: TreeNode): boolean => n.flags.some((f) => f.startsWith("auto-"));

export const importantMark = () => el("span", { class: "imp-mark", title: "Important" }, guiIcon("important"), el("span", { class: "sr-only" }, "Important"));

// Checked means the link is hidden; the box sits beside the icon and keeps its clicks from selecting the row.
export function hideBox(n: TreeNode): HTMLElement[] {
  if (!state.hidingLinks || !n.linked) return [];
  const box = hideCheckbox(n.name, n.hidden);
  box.addEventListener("change", () => void setLinkHidden(n, box.checked));
  return [box];
}

// `staircase` adds the review-link dashes; the grid leaves them out because they read badly in tiles.
export function nameCell(n: TreeNode, inlineStar = true, staircase = false): HTMLElement {
  const key = keyOf(n.relParts);
  const edit = state.editing;
  if (!edit || edit.key !== key) {
    const review = inReviewList(n);
    const text = review ? (staircase ? "-".repeat(reviewLevel(n) - 1) : "") + reviewName(n) : (staircase ? linkDashes(n) : "") + displayNodeName(n);
    return el("span", { class: isTopLink(n) || (review && reviewLevel(n) === 1) ? "name top-link" : "name" }, el("span", { class: "name-text" }, text), ...(n.favorite && inlineStar ? [favoriteMark()] : []), ...(n.important && inlineStar ? [importantMark()] : []), ...(runsAuto(n) ? [autoMark()] : []));
  }

  const input = textInput({ class: "name-input", "aria-label": `Rename ${n.name}` });
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
  keepClicks(input);
  queueMicrotask(() => {
    input.focus();
    if (edit.focused) input.setSelectionRange(input.value.length, input.value.length);
    else input.select();
    edit.focused = true;
  });
  return el("span", { class: "name editing" }, el("span", { class: "name-fixed" }, n.fixedPrefix), input);
}
