// One table settles every folder that breaks the id format: a radio per action on each row.
import { el } from "./dom";
import type { InvalidFolder, Resolution } from "../ipc/types";

export type Choice = "ignore" | "review" | "node" | "workboard";

export interface Decision {
  relParts: string[];
  choice: Choice;
  destination: string[] | null;
  tracker: string;
}

// The names each pending resolution would take, null where the row keeps its own name.
export type PreviewNames = (rows: { relParts: string[]; resolution: Resolution }[]) => Promise<(string | null)[]>;

const ACTIONS: [Choice, string][] = [
  ["ignore", "Ignore"],
  ["review", "Review"],
  ["node", "Node"],
  ["workboard", "Workboard"],
];
const TRACKERS = ["PJ", "PR", "HW", "CR", "TK"];

// Resolves to every row's decision, or null when the dialog is left with Later or Escape.
export function invalidDialog(folders: InvalidFolder[], openFolder: (relParts: string[]) => void, preview: PreviewNames): Promise<Decision[] | null> {
  return new Promise((resolve) => {
    const rows = folders.map((folder, i) => row(folder, i, openFolder));
    let latest = 0;
    const showPreview = async (): Promise<void> => {
      const turn = ++latest;
      const pending = rows.flatMap((r, index) => {
        const resolution = r.resolution();
        return resolution ? [{ index, relParts: r.relParts, resolution }] : [];
      });
      const names = pending.length > 0 ? await preview(pending.map(({ relParts, resolution }) => ({ relParts, resolution }))) : [];
      if (turn !== latest) return;
      rows.forEach((r) => r.showName(null));
      pending.forEach((p, k) => rows[p.index]?.showName(names[k] ?? null));
    };
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

    form.addEventListener("change", () => void showPreview());
    dialog.addEventListener("close", () => {
      resolve(dialog.returnValue === "ok" ? rows.map((r) => r.decision()) : null);
      dialog.remove();
    });
    document.body.append(dialog);
    dialog.showModal();
    table.querySelector<HTMLInputElement>('input[type="radio"]:checked')?.focus();
  });
}

interface Row {
  element: HTMLElement;
  relParts: string[];
  decision: () => Decision;
  resolution: () => Resolution | null;
  showName: (name: string | null) => void;
}

function row(folder: InvalidFolder, index: number, openFolder: (relParts: string[]) => void): Row {
  const name = `invalid-${index}`;
  const rel = folder.relParts;
  const label = rel.join("/");
  const original = rel[rel.length - 1] ?? "";
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

  const current = (): Choice => ACTIONS.map(([c]) => c).find((c) => radios.get(c)?.checked) ?? "ignore";
  const shown = el("strong", {}, original);
  const element = el("tr", {}, el("th", { scope: "row" }, pathButton(rel, shown, openFolder)), ...cells);
  return {
    element,
    relParts: rel,
    decision: () => {
      const choice = current();
      const destination = choice === "review" ? (folder.review[Number(reviewPick?.value ?? 0)] ?? null) : null;
      return { relParts: rel, choice, destination, tracker: trackerPick.value };
    },
    resolution: () => {
      const choice = current();
      if (choice === "node") return { kind: "node" };
      if (choice === "workboard") return { kind: "workboard", tracker: trackerPick.value };
      return null;
    },
    showName: (next) => {
      shown.textContent = next ?? original;
    },
  };
}

// The parent path shows only its last folder, as "...\<dir>\"; the button opens the folder in the file explorer.
function pathButton(rel: string[], name: HTMLElement, openFolder: (relParts: string[]) => void): HTMLButtonElement {
  const parents = rel.slice(0, -1);
  const trail = parents.length > 1 ? `...\\${parents[parents.length - 1] ?? ""}\\` : parents.map((part) => `${part}\\`).join("");
  const button = el("button", { type: "button", class: "invalid-path", title: "Open in Explorer" }, trail, name);
  button.addEventListener("click", () => openFolder(rel));
  return button;
}
