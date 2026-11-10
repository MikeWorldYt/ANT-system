// Tools that tidy a review folder: the boxes pick which run, and Batch mode shows their dry run in place of the views before anything changes.
import { el } from "../../dom";
import { checkbox } from "../../atoms/checkbox";
import { textButton } from "../../atoms/button";
import { guiIcon } from "../../atoms/gui-icons";
import { confirmAutomation } from "../dialogs/action-dialog/automation-warning";
import { batch, closeBatch, openBatch, replanBatch, runBatch } from "../../../hooks/automations/batch";
import { reviewScope, type ReviewScope } from "../../../hooks/automations/scope/review-scope";
import { readPref, savePref } from "../../../services/store/prefs";
import { state } from "../../../services/store/state";

function pick(kind: string, label: string, hint: string): { element: HTMLElement; box: HTMLInputElement } {
  const id = `review-auto-${kind}`;
  const pref = `fm.automation.${kind}`;
  const box = checkbox({ id });
  box.checked = readPref(pref) === "1";
  box.addEventListener("change", () => savePref(pref, box.checked ? "1" : "0"));
  return { element: el("label", { class: "side-action automation-row", for: id, title: hint }, box, el("span", { class: "side-action-label" }, label)), box };
}

function action(label: string, icon: "play" | "layout", onClick: () => void): HTMLButtonElement {
  const button = textButton("", "side-action accent automation-run", onClick);
  button.append(el("span", { class: "side-action-icon" }, guiIcon(icon)), el("span", { class: "side-action-label" }, label));
  return button;
}

export function createReviewAutomations(): { element: HTMLElement; render(): void } {
  let scope: ReviewScope | null = null;
  const reduce = pick("reduce", "Reduce name", "Strip hashes, site ids and other noise from file names");
  const sort = pick("sort", "Sort by projects", "Group files that look like one project into a folder");
  const auto = checkbox({ id: "review-auto-always" });
  const autoRow = el("label", { class: "side-action automation-row", for: "review-auto-always", title: "Run the ticked automations, without a dry run, on every file that lands in this folder" }, auto, el("span", { class: "side-action-label" }, "Run on every new file"));
  const enter = action("Batch mode", "layout", () => {
    if (scope) void openBatch(scope, reduce.box.checked, sort.box.checked);
  });
  const run = action("Run", "play", () => void runBatch(reduce.box.checked, sort.box.checked, auto.checked));
  const leave = textButton("Leave batch mode", "side-action", closeBatch);
  const element = el("section", { class: "side-section", "aria-labelledby": "review-automations-title" }, el("h3", { id: "review-automations-title", class: "side-label" }, "Automations"), leave, reduce.element, sort.element, autoRow, enter, run);

  for (const box of [reduce.box, sort.box]) box.addEventListener("change", () => (batch ? void replanBatch(reduce.box.checked, sort.box.checked) : render()));
  auto.addEventListener("change", async () => {
    if (auto.checked && !(await confirmAutomation())) auto.checked = false;
  });

  function render(): void {
    scope = reviewScope();
    if (batch && scope?.review?.relParts.join("/") !== batch.review.relParts.join("/")) closeBatch();
    element.hidden = scope === null;
    const inBatch = batch !== null;
    const none = !reduce.box.checked && !sort.box.checked;
    reduce.element.hidden = sort.element.hidden = autoRow.hidden = run.hidden = leave.hidden = !inBatch;
    enter.hidden = inBatch;
    enter.disabled = state.busy || !scope?.review;
    run.disabled = state.busy || none;
    if (!inBatch) {
      const flags = scope?.review?.flags ?? [];
      auto.checked = flags.includes("auto-reduce") || flags.includes("auto-sort");
    }
  }

  return { element, render };
}
