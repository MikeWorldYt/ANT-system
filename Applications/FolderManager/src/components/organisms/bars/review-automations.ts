// Tools that tidy a review folder: the boxes pick which run, and Batch mode shows their dry run in place of the views before anything changes.
import { el } from "../../dom";
import { checkbox } from "../../atoms/checkbox";
import { textButton } from "../../atoms/button";
import { guiIcon } from "../../atoms/gui-icons";
import { confirmAutomation } from "../dialogs/action-dialog/automation-warning";
import { AUTOMATIONS, autoKinds, batch, closeBatch, labelsOf, openBatch, replanBatch, runBatch, setAuto } from "../../../hooks/automations/batch";
import { reviewScope, type ReviewScope } from "../../../hooks/automations/scope/review-scope";
import { openDictionaryWindow } from "../../../services/ipc/api";
import { reportError, setStatus } from "./status-line";
import { readPref, savePref } from "../../../services/store/prefs";
import { state } from "../../../services/store/state";
import type { Picks } from "../../../services/ipc/types";

const HINTS: Record<keyof Picks, string> = {
  reduce: "Strip hashes, site ids and other noise from file names",
  addDate: "Put a date in front of each name: the one the name holds, or else when the file was created",
  removeDate: "Take every date out of the names; it wins over Add date",
  sort: "Group files that look like one project into a folder",
  group: "Move files into the folders the nearest dictionary.md names",
  removeSymbols: "Take the brackets ( ) [ ] { } out of the names",
};

function pick(kind: keyof Picks, label: string): { kind: keyof Picks; element: HTMLElement; box: HTMLInputElement } {
  const id = `review-auto-${kind}`;
  const pref = `fm.automation.${kind}`;
  const box = checkbox({ id });
  box.checked = readPref(pref) === "1";
  box.addEventListener("change", () => savePref(pref, box.checked ? "1" : "0"));
  return { kind, element: el("label", { class: "side-action automation-row", for: id, title: HINTS[kind] }, box, el("span", { class: "side-action-label" }, label)), box };
}

function action(label: string, icon: "play" | "layout" | "pencil", onClick: () => void): HTMLButtonElement {
  const button = textButton("", "side-action accent automation-run", onClick);
  button.append(el("span", { class: "side-action-icon" }, guiIcon(icon)), el("span", { class: "side-action-label" }, label));
  return button;
}

export function createReviewAutomations(): { element: HTMLElement; render(): void } {
  let scope: ReviewScope | null = null;
  const all = AUTOMATIONS.map((a) => pick(a.kind, a.label));
  const ticked = (): (keyof Picks)[] => all.filter((p) => p.box.checked).map((p) => p.kind);
  const picks = (): Picks => Object.fromEntries(all.map((p) => [p.kind, p.box.checked])) as unknown as Picks;
  const tick = (kinds: (keyof Picks)[]): void => {
    for (const p of all) {
      p.box.checked = kinds.includes(p.kind);
      savePref(`fm.automation.${p.kind}`, p.box.checked ? "1" : "0");
    }
  };

  // What already runs by itself stays in sight outside Batch mode, with a way to stop or change it.
  const autoState = el("strong", { class: "automation-auto-state" }, "Auto (Run on new files):");
  const autoList = el("ul", { class: "automation-auto-list" });
  const stop = textButton("Stop", "side-action automation-auto-stop", () => void switchAuto([]));
  const editAuto = textButton("Edit", "side-action", () => {
    const review = scope?.review;
    if (!scope || !review) return;
    tick(autoKinds(review));
    void openBatch(scope, picks());
  });
  const autoBox = el("div", { class: "automation-auto", role: "group", "aria-label": "Automatic run" }, autoState, autoList, el("div", { class: "automation-auto-actions" }, stop, editAuto));


  const enter = action("Open batch", "layout", () => {
    const review = scope?.review;
    if (!scope || !review) return;
    // A folder that already runs automations opens with those ticked, so the dry run shows what really happens to new files.
    const running = autoKinds(review);
    if (running.length > 0) tick(running);
    void openBatch(scope, picks());
  });
  const run = action("Run now", "play", () => void runBatch());
  // Unlike Run now, this keeps the ticked automations running on every file that lands later.
  const runAuto = action("Run automatically", "play", () => void startAuto());
  runAuto.classList.add("automation-run-auto");
  runAuto.title = "Run the ticked automations, without a dry run, on every file that lands in this folder";
  const leave = textButton("", "side-action", closeBatch);
  leave.append(el("span", { class: "side-action-icon" }, guiIcon("layout")), el("span", { class: "side-action-label" }, "Close batch"));
  // The dictionary belongs to the node that holds the review folder, so it reaches that node's other folders too.
  const dictionary = action("Edit dictionary", "pencil", () => {
    const review = scope?.review;
    if (review) openDictionaryWindow(review.relParts.slice(0, -1)).catch((e: unknown) => reportError("error", e, "Dictionary window"));
  });
  dictionary.classList.add("automation-dictionary");
  // The picks and the run buttons sit in one bordered box, so it reads which automations a run applies.
  const batchBox = el("div", { class: "automation-batch", role: "group", "aria-label": "Automations to run" }, ...all.map((p) => p.element), run, runAuto);
  run.classList.add("automation-run-small");
  runAuto.classList.add("automation-run-small");
  const element = el(
    "section",
    { class: "side-section", "aria-labelledby": "review-automations-title" },
    el("h3", { id: "review-automations-title", class: "side-label" }, "Automations"),
    dictionary,
    leave,
    batchBox,
    enter,
    autoBox,
  );

  async function switchAuto(kinds: (keyof Picks)[]): Promise<void> {
    const review = scope?.review;
    if (!review) return;
    try {
      await setAuto(review, kinds);
      setStatus("info", kinds.length > 0 ? `${review.name} now runs ${labelsOf(kinds)} on every new file.` : `Automations stopped in ${review.name}.`);
    } catch (e) {
      reportError("error", e, "Automations");
    }
  }

  async function startAuto(): Promise<void> {
    if (ticked().length === 0 || !(await confirmAutomation())) return;
    await switchAuto(ticked());
  }

  for (const { box } of all) box.addEventListener("change", () => (batch ? void replanBatch(picks()) : render()));

  function render(): void {
    scope = reviewScope();
    if (batch && scope?.review?.relParts.join("/") !== batch.review.relParts.join("/")) closeBatch();
    element.hidden = scope === null;
    const inBatch = batch !== null;
    const running = scope?.review ? autoKinds(scope.review) : [];
    autoBox.hidden = running.length === 0;
    autoList.replaceChildren(...AUTOMATIONS.filter((a) => running.includes(a.kind)).map((a) => el("li", {}, a.label)));
    stop.disabled = state.busy;
    editAuto.hidden = inBatch;
    batchBox.hidden = leave.hidden = !inBatch;
    const chosen = ticked();
    runAuto.disabled = state.busy || chosen.length === 0;
    enter.hidden = inBatch;
    enter.disabled = state.busy || !scope?.review;
    run.disabled = state.busy || chosen.length === 0;
    dictionary.disabled = !scope?.review;
  }

  return { element, render };
}
