// Tools that tidy a review folder; the selection decides whether they reach the whole folder, one folder's files, or only what is marked.
import { el } from "../../dom";
import { textButton } from "../../atoms/button";
import { reduceNames } from "../../../hooks/automations/reduce-name";
import { sortProjects } from "../../../hooks/automations/sort-projects";
import { reviewScope, type ReviewScope } from "../../../hooks/automations/scope/review-scope";
import { state } from "../../../services/store/state";

export function createReviewAutomations(): { element: HTMLElement; render(): void } {
  let scope: ReviewScope | null = null;
  const reduce = textButton("Reduce name", "side-action", () => {
    if (scope) void reduceNames(scope);
  });
  reduce.title = "Shorten long hexadecimal file names to their first 4 characters";
  const sort = textButton("Sort by projects", "side-action", () => {
    if (scope) void sortProjects(scope);
  });
  sort.title = "Group files that look like one project into a folder, after a preview";
  const reach = el("p", { class: "details-empty" });
  const element = el("section", { class: "side-section", "aria-labelledby": "review-automations-title" }, el("h3", { id: "review-automations-title", class: "side-label" }, "Automations"), reach, reduce, sort);

  function render(): void {
    scope = reviewScope();
    element.hidden = scope === null;
    reduce.disabled = sort.disabled = state.busy;
    sort.hidden = scope === null || scope.folders.length !== 1 || scope.files.length > 0;
    reach.textContent = scope?.label ?? "";
  }

  return { element, render };
}
