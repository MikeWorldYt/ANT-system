// Asks before intake moves loose files out of a node the app has never managed: continue, or keep them with the data flag.
import { el } from "./dom";
import type { WaitingNode } from "../ipc/types";

export type WaitingChoice = "continue" | "data";

export interface WaitingDecision {
  relParts: string[];
  choice: WaitingChoice;
}

// Resolves to every node's decision, or null when the dialog is left with Later or Escape.
export function waitingIntakeDialog(nodes: WaitingNode[]): Promise<WaitingDecision[] | null> {
  return new Promise((resolve) => {
    const rows = nodes.map((node, i) => row(node, i));
    const table = el(
      "table",
      { class: "invalid-table", "aria-label": "Folders with loose files" },
      el(
        "thead",
        {},
        el("tr", {}, el("th", { scope: "col" }, "Folder"), el("th", { scope: "col" }, "Files"), el("th", { scope: "col" }, "Continue"), el("th", { scope: "col" }, "Set data flag")),
      ),
      el("tbody", {}, ...rows.map((r) => r.element)),
    );
    const later = el("button", { type: "submit", class: "btn", value: "cancel" }, "Later");
    const apply = el("button", { type: "submit", class: "btn btn-primary", value: "ok" }, "Apply");
    const form = el(
      "form",
      { method: "dialog" },
      el("h2", { id: "waiting-title" }, "Loose files in new folders"),
      el("p", { class: "dialog-text" }, "From now on the loose files of these folders move to their 98.* review folder. Set the data flag to keep them where they are, or continue to let the automation run."),
      el("div", { class: "invalid-scroll" }, table),
      el("div", { class: "dialog-buttons" }, later, apply),
    );
    const dialog = el("dialog", { class: "confirm invalid-dialog", "aria-labelledby": "waiting-title" }, form);

    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "ok" ? rows.map((r) => r.decision()) : null);
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    table.querySelector<HTMLInputElement>('input[type="radio"]')?.focus();
  });
}

function row(node: WaitingNode, index: number): { element: HTMLElement; decision: () => WaitingDecision } {
  const name = `waiting-${index}`;
  const cont = el("input", { type: "radio", name, value: "continue", checked: "", "aria-label": `Continue ${node.node}` });
  const flag = el("input", { type: "radio", name, value: "data", "aria-label": `Set data flag on ${node.node}` });
  const element = el("tr", {}, el("th", { scope: "row" }, node.node), el("td", {}, String(node.files)), el("td", {}, cont), el("td", {}, flag));
  return {
    element,
    decision: () => ({ relParts: node.relParts, choice: flag.checked ? "data" : "continue" }),
  };
}
