// Direction toggle that sits in the top padding of a Columns view column.
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { setColumnSort, sortOf } from "../store/state";

export function columnSortToggle(columnKey: string): HTMLButtonElement {
  const { dir } = sortOf(columnKey);
  const label = dir === "asc" ? "Sort descending" : "Sort ascending";
  // The view keeps a single tab stop; keyboard sorting stays on the toolbar.
  const btn = el("button", { type: "button", class: "col-sort", tabindex: "-1", "aria-label": label, title: label }, guiIcon(dir === "asc" ? "moveUp" : "moveDown"));
  btn.addEventListener("click", (ev) => {
    // The column background would otherwise step back to its on-path row.
    ev.stopPropagation();
    setColumnSort(columnKey, { dir: dir === "asc" ? "desc" : "asc" });
  });
  return btn;
}
