// Asks before intake moves loose files out of a node the app has never managed: continue, or keep them with the data flag.
import { el } from "../../../dom";
import { dialogButton } from "../../../atoms/button";
import { openDialog } from "../dialog-shell";
import { colHeader, dialogTable } from "./action-dialog";
import type { WaitingNode } from "../../../../services/ipc/types";

export type WaitingChoice = "continue" | "data";

export interface WaitingDecision {
  relParts: string[];
  choice: WaitingChoice;
}

// Resolves to every node's decision, or null when the dialog is left with Later or Escape.
export async function waitingIntakeDialog(nodes: WaitingNode[]): Promise<WaitingDecision[] | null> {
  const rows = nodes.map((node, i) => row(node, i));
  const headers = ["Folder", "Files", "Continue", "Set data flag"].map((label) => colHeader([label]));
  const { table, view } = dialogTable("Folders with loose files", headers, rows.map((r) => r.element));
  const { closed } = openDialog({
    title: "Loose files in new folders",
    wide: true,
    body: [el("p", { class: "dialog-text" }, "From now on the loose files of these folders move to their 98.* review folder. Set the data flag to keep them where they are, or continue to let the automation run."), view],
    buttons: [dialogButton("Later", "cancel"), dialogButton("Apply", "ok", "primary")],
  });
  table.querySelector<HTMLInputElement>('input[type="radio"]')?.focus();
  return (await closed) === "ok" ? rows.map((r) => r.decision()) : null;
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
