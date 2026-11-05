// Tools that tidy a review folder; the selection decides whether they reach the whole folder, one folder's files, or only what is marked.
import { el } from "../../dom";
import { textButton } from "../../atoms/button";
import { reduceNames } from "../../../hooks/automations/reduce-name";
import { reviewScope, type ReviewScope } from "../../../hooks/automations/scope/review-scope";
import { state } from "../../../services/store/state";

export function createReviewAutomations(): { element: HTMLElement; render(): void } {
  let scope: ReviewScope | null = null;
  const reduce = textButton("Reduce name", "side-action", () => {
    if (scope) void reduceNames(scope);
  });
  reduce.title = "Shorten long hexadecimal file names to their first 4 characters";
  const reach = el("p", { class: "details-empty" });
  const element = el("section", { class: "side-section", "aria-labelledby": "review-automations-title" }, el("h3", { id: "review-automations-title", class: "side-label" }, "Automations"), reach, reduce);

  function render(): void {
    scope = reviewScope();
    element.hidden = scope === null;
    reduce.disabled = state.busy;
    reach.textContent = scope?.label ?? "";
  }

  return { element, render };
}
