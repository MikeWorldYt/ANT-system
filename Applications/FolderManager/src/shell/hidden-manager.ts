// Side panel list that shows a folder's children with a checkbox each; a checked folder stays hidden.
import { setChildHidden, stopManaging } from "../actions/ops";
import { el } from "../ui/dom";
import { state } from "../store/state";

export interface HiddenManager {
  element: HTMLElement;
  render(): void;
}

export function createHiddenManager(): HiddenManager {
  const hint = el("p", { class: "dialog-text" });
  const list = el("div", { class: "hidden-list" });
  const done = el("button", { type: "button", class: "btn btn-primary" }, "Done");
  done.addEventListener("click", stopManaging);
  const element = el(
    "section",
    { class: "side-section", "aria-labelledby": "hidden-manage-title" },
    el("h3", { id: "hidden-manage-title", class: "side-label" }, "Manage hidden nodes"),
    hint,
    list,
    el("div", { class: "details-actions" }, done),
  );

  function render(): void {
    const folder = state.managing;
    element.hidden = folder === null;
    if (folder === null) return;
    hint.textContent = `Checked folders in ${folder.join("/") || "the storage"} stay hidden.`;
    const rows = state.hiddenChoices.map((choice) => {
      const box = el("input", { type: "checkbox", value: choice.name });
      box.checked = choice.hidden;
      box.disabled = choice.fixed;
      box.addEventListener("change", () => void setChildHidden(choice.name, box.checked));
      return el("label", { class: "hidden-choice" }, box, el("span", {}, choice.name));
    });
    list.replaceChildren(...(rows.length > 0 ? rows : [el("p", { class: "dialog-text" }, "This folder has no folders inside.")]));
  }

  return { element, render };
}
