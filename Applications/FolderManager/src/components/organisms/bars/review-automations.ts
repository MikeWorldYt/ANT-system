// Tools that tidy a review folder: the boxes pick which run, and Batch mode shows their dry run in place of the views before anything changes.
import { el } from "../../dom";
import { checkbox } from "../../atoms/checkbox";
import { textButton } from "../../atoms/button";
import { guiIcon } from "../../atoms/gui-icons";
import { confirmAutomation } from "../dialogs/action-dialog/automation-warning";
import { batch, closeBatch, openBatch, replanBatch, runBatch } from "../../../hooks/automations/batch";
import { reviewScope, type ReviewScope } from "../../../hooks/automations/scope/review-scope";
import { openDictionaryWindow } from "../../../services/ipc/api";
import { reportError } from "./status-line";
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

function action(label: string, icon: "play" | "layout" | "pencil", onClick: () => void): HTMLButtonElement {
  const button = textButton("", "side-action accent automation-run", onClick);
  button.append(el("span", { class: "side-action-icon" }, guiIcon(icon)), el("span", { class: "side-action-label" }, label));
  return button;
}

export function createReviewAutomations(): { element: HTMLElement; render(): void } {
  let scope: ReviewScope | null = null;
  const reduce = pick("reduce", "Clean up names", "Strip hashes, site ids and other noise from file names");
  const addDate = pick("addDate", "Add date", "Put a date in front of each name: the one the name holds, or else when the file was created");
  const removeDate = pick("removeDate", "Remove date", "Take every date out of the names; it wins over Add date");
  const sort = pick("sort", "Sort by projects", "Group files that look like one project into a folder");
  const group = pick("group", "Group by dictionary", "Move files into the folders the nearest dictionary.md names");
  const removeSymbols = pick("removeSymbols", "Remove symbols", "Take the brackets ( ) [ ] { } out of the names");
  const all = [reduce, addDate, removeDate, sort, group, removeSymbols];
  const picks = () => ({ reduce: reduce.box.checked, addDate: addDate.box.checked, removeDate: removeDate.box.checked, sort: sort.box.checked, group: group.box.checked, removeSymbols: removeSymbols.box.checked });
  const auto = checkbox({ id: "review-auto-always" });
  const autoRow = el("label", { class: "side-action automation-row", for: "review-auto-always", title: "Run the ticked automations, without a dry run, on every file that lands in this folder" }, auto, el("span", { class: "side-action-label" }, "Run on every new file"));
  const enter = action("Batch mode", "layout", () => {
    if (scope) void openBatch(scope, picks());
  });
  const run = action("Run", "play", () => void runBatch(picks(), auto.checked));
  const leave = textButton("Leave batch mode", "side-action", closeBatch);
  // The dictionary belongs to the node that holds the review folder, so it reaches that node's other folders too.
  const dictionary = action("Edit dictionary", "pencil", () => {
    const review = scope?.review;
    if (review) openDictionaryWindow(review.relParts.slice(0, -1)).catch((e: unknown) => reportError("error", e, "Dictionary window"));
  });
  const element = el("section", { class: "side-section", "aria-labelledby": "review-automations-title" }, el("h3", { id: "review-automations-title", class: "side-label" }, "Automations"), leave, ...all.map((p) => p.element), autoRow, enter, run, dictionary);

  for (const { box } of all) box.addEventListener("change", () => (batch ? void replanBatch(picks()) : render()));
  auto.addEventListener("change", async () => {
    if (auto.checked && !(await confirmAutomation())) auto.checked = false;
  });

  function render(): void {
    scope = reviewScope();
    if (batch && scope?.review?.relParts.join("/") !== batch.review.relParts.join("/")) closeBatch();
    element.hidden = scope === null;
    const inBatch = batch !== null;
    const none = all.every((p) => !p.box.checked);
    for (const p of all) p.element.hidden = !inBatch;
    autoRow.hidden = run.hidden = leave.hidden = !inBatch;
    enter.hidden = inBatch;
    enter.disabled = state.busy || !scope?.review;
    run.disabled = state.busy || none;
    dictionary.disabled = !scope?.review;
    if (!inBatch) {
      const flags = scope?.review?.flags ?? [];
      auto.checked = ["auto-reduce", "auto-add-date", "auto-remove-date", "auto-sort", "auto-group", "auto-remove-symbols"].some((f) => flags.includes(f));
    }
  }

  return { element, render };
}
