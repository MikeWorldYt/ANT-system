// Asks before intake moves loose files out of a folder the app has never managed: sort them automatically, or keep them where they are.
import { el } from "../../../dom";
import { textButton } from "../../../atoms/button";
import { radio } from "../../../atoms/checkbox";
import { actionDialog, colHeader, dialogTable } from "./action-dialog";
import { pathButton } from "./invalid-dialog";
import type { WaitingNode } from "../../../../services/ipc/types";

export type WaitingChoice = "continue" | "data";

export interface WaitingDecision {
  relParts: string[];
  choice: WaitingChoice;
}

// Settles one row at once and says whether it went through.
export type ApplyWaiting = (decision: WaitingDecision) => Promise<boolean>;

interface Row {
  element: HTMLElement;
  applyButton: HTMLButtonElement;
  decision: () => WaitingDecision;
}

// Resolves to the decisions of the rows still waiting, or null when the dialog is left with Later or Escape.
export function waitingIntakeDialog(nodes: WaitingNode[], showFolder: (relParts: string[]) => void, applyOne: ApplyWaiting): Promise<WaitingDecision[] | null> {
  return new Promise((resolve) => {
    const rows = nodes.map((node, i) => row(node, i, showFolder));
    const headers = [colHeader(["Folder"]), colHeader(["Files"]), colHeader(["Sort automatically"]), colHeader(["Keep unsorted"]), colHeader([], { "aria-label": "Apply this folder only" })];
    const { table, view } = dialogTable("File sorting needs you permission", headers, rows.map((r) => r.element));
    const intro = [
      el("p", { class: "dialog-text" }, "ANT detects new folders have files lying loose and needs to know what to do with them."),
      el(
        "p",
        { class: "dialog-text" },
        el("strong", {}, "Sort automatically"),
        ": moves them to a review folder (REV.*), now and from now on.",
        el("br"),
        el("strong", {}, "Keep unsorted"),
        ": leaves them where they are so you can sort them manually.",
      ),
    ];
    const { dialog, closed, later, apply } = actionDialog("File sorting needs you permission", intro, view, () => table.querySelector<HTMLInputElement>('input[type="radio"]')?.focus());
    const setWorking = (working: boolean): void => {
      for (const r of rows) r.applyButton.disabled = working;
      later.disabled = apply.disabled = working;
    };
    for (const r of rows) {
      r.applyButton.addEventListener("click", () => {
        setWorking(true);
        void applyOne(r.decision()).then((done) => {
          setWorking(false);
          if (!done) return;
          rows.splice(rows.indexOf(r), 1);
          r.element.remove();
          if (rows.length === 0) dialog.close("ok");
        });
      });
    }
    void closed.then((value) => resolve(value === "ok" ? rows.map((r) => r.decision()) : null));
  });
}

function row(node: WaitingNode, index: number, showFolder: (relParts: string[]) => void): Row {
  const name = `waiting-${index}`;
  const sort = radio({ name, value: "continue", checked: "", "aria-label": `Sort the files of ${node.node} automatically` });
  const keep = radio({ name, value: "data", "aria-label": `Keep the files of ${node.node} unsorted` });
  const applyButton = textButton("Apply", "invalid-apply");
  applyButton.setAttribute("aria-label", `Apply only ${node.node}`);
  const folder = pathButton(node.relParts, el("strong", {}, node.relParts[node.relParts.length - 1] ?? node.node), showFolder);
  const element = el("tr", {}, el("th", { scope: "row" }, folder), el("td", {}, String(node.files)), el("td", {}, sort), el("td", {}, keep), el("td", {}, applyButton));
  return {
    element,
    applyButton,
    decision: () => ({ relParts: node.relParts, choice: keep.checked ? "data" : "continue" }),
  };
}
