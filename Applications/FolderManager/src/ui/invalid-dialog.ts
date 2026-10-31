// One table settles every folder that breaks the id format: a radio per action on each row.
import { el } from "./dom";
import type { InvalidFolder } from "../ipc/types";

export type Choice = "ignore" | "review" | "node" | "workboard";

export interface Decision {
  relParts: string[];
  choice: Choice;
  destination: string[] | null;
  tracker: string;
}

const ACTIONS: [Choice, string][] = [
  ["ignore", "Ignore"],
  ["review", "Review"],
  ["node", "Node"],
  ["workboard", "Workboard"],
];
const TRACKERS = ["PJ", "PR", "HW", "CR", "TK"];

// Resolves to every row's decision, or null when the dialog is left with Later or Escape.
export function invalidDialog(folders: InvalidFolder[], openFolder: (relParts: string[]) => void): Promise<Decision[] | null> {
  return new Promise((resolve) => {
    const rows = folders.map((folder, i) => row(folder, i, openFolder));
    const table = el(
      "table",
      { class: "invalid-table", "aria-label": "Folders outside the id format" },
      el("thead", {}, el("tr", {}, el("th", { scope: "col" }, "Path"), ...ACTIONS.map(([, label]) => el("th", { scope: "col" }, label)))),
      el("tbody", {}, ...rows.map((r) => r.element)),
    );
    const later = el("button", { type: "submit", class: "btn", value: "cancel" }, "Later");
    const apply = el("button", { type: "submit", class: "btn btn-primary", value: "ok" }, "Apply");
    const form = el(
      "form",
      { method: "dialog" },
      el("h2", { id: "invalid-title" }, "Invalid directories found"),
      el("p", { class: "dialog-text" }, "These folders do not follow the id format. Choose what each one becomes."),
      el("div", { class: "invalid-scroll" }, table),
      el("div", { class: "dialog-buttons" }, later, apply),
    );
    const dialog = el("dialog", { class: "confirm invalid-dialog", "aria-labelledby": "invalid-title" }, form);

    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "ok" ? rows.map((r) => r.decision()) : null);
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    table.querySelector<HTMLInputElement>('input[type="radio"]:checked')?.focus();
  });
}

function row(folder: InvalidFolder, index: number, openFolder: (relParts: string[]) => void): { element: HTMLElement; decision: () => Decision } {
  const name = `invalid-${index}`;
  const rel = folder.relParts;
  const label = rel.join("/");
  const hasReview = folder.review.length > 0;
  const reviewPick = folder.review.length > 1 ? el("select", { class: "dialog-input invalid-pick", "aria-label": `Review folder for ${label}` }, ...folder.review.map((parts, i) => el("option", { value: String(i) }, parts.join("/")))) : null;
  const trackerPick = el("select", { class: "dialog-input invalid-pick", "aria-label": `Tracker for ${label}` }, ...TRACKERS.map((t) => el("option", { value: t }, t)));
  const radios = new Map<Choice, HTMLInputElement>();

  const cells = ACTIONS.map(([choice, action]) => {
    const enabled = choice === "ignore" || choice === "node" || (choice === "review" && hasReview) || (choice === "workboard" && rel.length >= 2);
    const attrs: Record<string, string> = { type: "radio", name, value: choice, "aria-label": `${action} ${label}` };
    if (!enabled) attrs.disabled = "";
    const input = el("input", attrs);
    radios.set(choice, input);
    const extra = choice === "review" ? reviewPick : choice === "workboard" ? trackerPick : null;
    return el("td", {}, input, ...(extra ? [extra] : []));
  });
  const start: Choice = hasReview ? "review" : "ignore";
  const startRadio = radios.get(start);
  if (startRadio) startRadio.checked = true;

  const element = el("tr", {}, el("th", { scope: "row" }, pathButton(rel, openFolder)), ...cells);
  return {
    element,
    decision: () => {
      const choice = ACTIONS.map(([c]) => c).find((c) => radios.get(c)?.checked) ?? "ignore";
      const destination = choice === "review" ? folder.review[Number(reviewPick?.value ?? 0)] ?? null : null;
      return { relParts: rel, choice, destination, tracker: trackerPick.value };
    },
  };
}

// The parent path is shortened to its last two folders; the button opens the folder in the file explorer.
function pathButton(rel: string[], openFolder: (relParts: string[]) => void): HTMLButtonElement {
  const parents = rel.slice(0, -1);
  const prefix = parents.length > 2 ? "...\\" : "";
  const trail = parents
    .slice(-2)
    .map((part) => `${part}\\`)
    .join("");
  const button = el("button", { type: "button", class: "invalid-path", title: "Open in Explorer" }, `${prefix}${trail}`, el("strong", {}, rel[rel.length - 1] ?? ""));
  button.addEventListener("click", () => openFolder(rel));
  return button;
}
