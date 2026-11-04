// Manage mode: rows of the managed folder get a checkbox that hides them, and a header row confirms the mode.
import { setChildHidden, stopManaging } from "../../hooks/ops";
import { el } from "../dom";
import { sortedForColumns } from "../../services/store/sort";
import { keyOf, sortOf, state } from "../../services/store/state";
import type { HiddenChoice, TreeNode } from "../../services/ipc/types";

export interface ManageSnapshot {
  choices: HiddenChoice[];
  selected: string | null;
}

export const isManagedFolder = (key: string): boolean => state.managing !== null && keyOf(state.managing) === key;

export const isHiddenChild = (n: TreeNode): boolean => state.hiddenChoices.some((c) => c.node.name === n.name && c.hidden);

// The managed folder's children, hidden ones included, or null until they are loaded.
export function managedNodes(): TreeNode[] | null {
  if (state.managing === null || state.hiddenChoices.length === 0) return null;
  return sortedForColumns(state.hiddenChoices.map((c) => c.node), sortOf(keyOf(state.managing)), state.settings.pinFavorites);
}

// Records what the list was built from, so a render with the same choices and selection keeps the rows in place.
export const manageSnapshot = (): ManageSnapshot | null => (state.managing === null ? null : { choices: state.hiddenChoices, selected: state.selectedKey });

export const sameManage = (built: ManageSnapshot | null): boolean =>
  built !== null && manageSnapshot() !== null && built.choices === state.hiddenChoices && built.selected === state.selectedKey;

// A row of the managed folder gets its checkbox; toggling it updates that row only.
export function manageBox(n: TreeNode, row: HTMLElement): HTMLElement[] {
  if (state.managing === null || keyOf(n.relParts.slice(0, -1)) !== keyOf(state.managing)) return [];
  const choice = state.hiddenChoices.find((c) => c.node.name === n.name);
  const box = el("input", { type: "checkbox", class: "hide-box", "aria-label": `Hide ${n.name}` });
  box.checked = isHiddenChild(n);
  box.disabled = choice?.fixed ?? false;
  row.classList.toggle("is-hidden", box.checked);
  for (const type of ["click", "dblclick", "pointerdown"]) box.addEventListener(type, (ev) => ev.stopPropagation());
  box.addEventListener("change", async () => {
    const on = box.checked;
    row.classList.toggle("is-hidden", on);
    if (!(await setChildHidden(n.name, on))) {
      box.checked = !on;
      row.classList.toggle("is-hidden", !on);
    }
  });
  return [box];
}

export function manageHeader(className: string): HTMLElement {
  const done = el("button", { type: "button", class: "btn manage-done" }, "Done");
  done.addEventListener("click", stopManaging);
  return el("div", { class: `manage-header ${className}` }, el("span", { class: "manage-name" }, "Manage hidden nodes"), done);
}
