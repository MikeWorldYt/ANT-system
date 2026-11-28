// Name column of a row: plain text, or the in-place editor when this node is being renamed.
import { el, keepClicks } from "../dom";
import { guiIcon } from "../atoms/gui-icons";
import { keepExtensionInSight } from "../atoms/fit-name";
import { hideCheckbox } from "../atoms/checkbox";
import { textInput } from "../atoms/text-input";
import { extOf, growToFit, keepExtension, selectStem } from "./keep-extension";
import { renameTo, setLinkHidden } from "../../hooks/ops";
import { keyOf, state, stopEdit } from "../../services/store/state";
import type { TreeNode } from "../../services/ipc/types";
import { inReviewList, reviewLevel, reviewName } from "../../services/store/review-list";
import { displayNodeName, isTopLink, linkDashes } from "../../services/store/node-info";

// The view the user last pressed in; a rename edits the name only there, not in every view that shows the node.
let pressed: Element | null = null;
document.addEventListener("pointerdown", (ev) => {
  if (ev.target instanceof Element && !ev.target.closest("[role=menu]")) pressed = ev.target.closest(".tree, .columns, .graph") ?? ev.target.closest("[role=listbox]");
}, true);

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

// `staircase` adds the review-link dashes; the grid leaves them out because they read badly in tiles. `wrap` edits the name over several lines, as a tile shows it.
export function nameCell(n: TreeNode, inlineStar = true, staircase = false, wrap = false): HTMLElement {
  const key = keyOf(n.relParts);
  const edit = state.editing;
  if (edit && edit.key === key && edit.view === undefined) edit.view = pressed;
  if (!edit || edit.key !== key) return plainName(n, inlineStar, staircase, wrap);
  // Queued before the editor focuses itself, so other views showing the same node swap back to the plain name first.
  queueMicrotask(() => {
    if (edit.view && cell.isConnected && !edit.view.contains(cell)) cell.replaceWith(plainName(n, inlineStar, staircase, wrap));
  });
  const cell = editor(n, edit, key, wrap);
  return cell;
}

function plainName(n: TreeNode, inlineStar: boolean, staircase: boolean, wrap: boolean): HTMLElement {
  const review = inReviewList(n);
  const text = review ? (staircase ? "-".repeat(reviewLevel(n) - 1) : "") + reviewName(n) : (staircase ? linkDashes(n) : "") + displayNodeName(n);
  const shown = el("span", { class: "name-text" }, text);
  if (wrap && n.file) keepExtensionInSight(shown, text);
  return el("span", { class: isTopLink(n) || (review && reviewLevel(n) === 1) ? "name top-link" : "name" }, shown, ...(n.favorite && inlineStar ? [favoriteMark()] : []), ...(n.important && inlineStar ? [importantMark()] : []), ...(runsAuto(n) ? [autoMark()] : []));
}

function editor(n: TreeNode, edit: NonNullable<typeof state.editing>, key: string, wrap: boolean): HTMLElement {
  const input = wrap ? el("textarea", { class: "name-input wrap", "aria-label": `Rename ${n.name}`, spellcheck: "false", rows: "1" }) : textInput({ class: "name-input", "aria-label": `Rename ${n.name}` });
  input.value = edit.draft;
  const ext = n.file ? extOf(n.name) : "";
  keepExtension(input, ext);
  const fit = input instanceof HTMLTextAreaElement ? growToFit(input) : () => undefined;
  const finish = (commit: boolean) => {
    if (state.editing?.key !== key) return;
    const text = input.value.replace(/\s*\n\s*/g, " ");
    stopEdit();
    if (commit) void renameTo(n, text);
  };
  input.addEventListener("input", () => {
    edit.draft = input.value;
  });
  (input as HTMLElement).addEventListener("keydown", (ev) => {
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
    if (!input.isConnected) return;
    fit();
    input.focus();
    if (edit.focused) input.setSelectionRange(input.value.length, input.value.length);
    else if (ext) selectStem(input, ext);
    else input.select();
    edit.focused = true;
  });
  return el("span", { class: wrap ? "name editing wrap" : "name editing" }, el("span", { class: "name-fixed" }, n.fixedPrefix), input);
}
