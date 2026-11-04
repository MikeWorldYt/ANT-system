// One table settles every folder that breaks the id format: a radio per action on each row.
import { el } from "./dom";
import { guiIcon } from "./gui-icons";
import { dialogButton, openDialog } from "./dialog/shell";
import { colHeader, dialogTable } from "./dialog/table";
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

// Two-line headers keep the table narrow.
const ACTIONS: [Choice, string[]][] = [
  ["ignore", ["Ignore"]],
  ["review", ["Move to", "Review"]],
  ["node", ["Convert to", "Node"]],
  ["workboard", ["Convert to", "Workboard"]],
];
const TRACKERS = ["PJ", "PR", "HW", "CR", "TK"];
// Matches --name-cycle in invalid-dialog.css.
const NAME_CYCLE_MS = 8000;

// Settles one row at once and says whether it went through.
export type ApplyOne = (decision: Decision) => Promise<boolean>;

// Sends one folder to the Recycle Bin after asking, and says whether it went.
export type DeleteOne = (relParts: string[]) => Promise<boolean>;

const isInside = (child: string[], parent: string[]): boolean => child.length > parent.length && parent.every((part, i) => part === child[i]);

// Resolves to the decisions of the rows still waiting, or null when the dialog is left with Later or Escape.
export function invalidDialog(folders: InvalidFolder[], openFolder: (relParts: string[]) => void, preview: PreviewNames, applyOne: ApplyOne, deleteOne: DeleteOne): Promise<Decision[] | null> {
  return new Promise((resolve) => {
    const rows = folders.map((folder, i) => row(folder, i, openFolder));
    let latest = 0;
    let working = false;
    // A folder with others inside it waits for them, as renaming or moving it would change their paths.
    const refreshApply = (): void => {
      for (const r of rows) r.setBlocked(working || rows.some((o) => o !== r && isInside(o.relParts, r.relParts)));
      later.disabled = apply.disabled = working;
    };
    const settleRow = async (r: Row, act: () => Promise<boolean>): Promise<void> => {
      working = true;
      refreshApply();
      const done = await act();
      working = false;
      if (!done) {
        refreshApply();
        return;
      }
      rows.splice(rows.indexOf(r), 1);
      r.element.remove();
      if (rows.length === 0) {
        dialog.close("ok");
        return;
      }
      refreshApply();
      void showPreview();
    };
    const applyRow = (r: Row): Promise<void> => settleRow(r, () => applyOne(r.decision()));
    const deleteRow = (r: Row): Promise<void> => settleRow(r, () => deleteOne(r.relParts));
    const showPreview = async (): Promise<void> => {
      const turn = ++latest;
      const pending = rows.flatMap((r, index) => {
        const resolution = r.resolution();
        return resolution ? [{ index, relParts: r.relParts, resolution }] : [];
      });
      const names = pending.length > 0 ? await preview(pending.map(({ relParts, resolution }) => ({ relParts, resolution }))) : [];
      if (turn !== latest) return;
      const renamed = new Map<number, string | null>(pending.map((p, k): [number, string | null] => [p.index, names[k] ?? null]));
      rows.forEach((r, i) => r.showName(renamed.get(i) ?? null));
    };
    const headers = [colHeader(["Path"]), ...ACTIONS.map(([, lines]) => colHeader(lineBreaks(lines))), colHeader([], { "aria-label": "Delete or apply this folder only" })];
    const { table, view } = dialogTable("Folders outside the id format", headers, rows.map((r) => r.element));
    const later = dialogButton("Later", "cancel");
    const apply = dialogButton("Apply", "ok", "primary");
    const { dialog, form, closed } = openDialog({
      title: "Format validations",
      wide: true,
      body: [el("p", { class: "dialog-text" }, "ANT detects some invalid formats that have reached your storage. Choose what each one become."), view],
      buttons: [later, apply],
    });

    for (const r of rows) {
      r.applyButton.addEventListener("click", () => void applyRow(r));
      r.deleteButton.addEventListener("click", () => void deleteRow(r));
    }
    refreshApply();
    form.addEventListener("change", () => void showPreview());
    void closed.then((value) => resolve(value === "ok" ? rows.map((r) => r.decision()) : null));
    void showPreview();
    table.querySelector<HTMLInputElement>('input[type="radio"]:checked')?.focus();
  });
}

interface Row {
  element: HTMLElement;
  relParts: string[];
  applyButton: HTMLButtonElement;
  deleteButton: HTMLButtonElement;
  setBlocked: (blocked: boolean) => void;
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
  trackerPick.value = folder.tracker ?? "PJ";
  const radios = new Map<Choice, HTMLInputElement>();

  const cells = ACTIONS.map(([choice, action]) => {
    const enabled = choice === "ignore" || choice === "node" || (choice === "review" && hasReview) || (choice === "workboard" && rel.length >= 2);
    const attrs: Record<string, string> = { type: "radio", name, value: choice, "aria-label": `${action.join(" ")} ${label}` };
    if (!enabled) attrs.disabled = "";
    const input = el("input", attrs);
    radios.set(choice, input);
    const extra = choice === "review" ? reviewPick : choice === "workboard" ? trackerPick : null;
    return el("td", {}, input, ...(extra ? [extra] : []));
  });
  const start: Choice = folder.workboard && rel.length >= 2 ? "workboard" : hasReview ? "review" : "ignore";
  const startRadio = radios.get(start);
  if (startRadio) startRadio.checked = true;

  const current = (): Choice => ACTIONS.map(([c]) => c).find((c) => radios.get(c)?.checked) ?? "ignore";
  const shown = el("strong", {}, original);
  let applied = "";
  const nameFor = (next: string | null, choice: Choice): HTMLElement | string => {
    if (next === null || next === original) return choice === "ignore" ? el("span", { class: "invalid-name-kept" }, original) : original;
    // Every row reads the same clock, so rows that open at different times still fade together.
    const style = `animation-delay: -${Date.now() % NAME_CYCLE_MS}ms`;
    return el(
      "span",
      { class: "invalid-name-cycle" },
      el("span", { class: "invalid-name-old", style }, original),
      el("span", { class: "invalid-name-new", style }, next),
    );
  };
  const applyButton = el("button", { type: "button", class: "btn invalid-apply", "aria-label": `Apply only ${label}` }, "Apply");
  const deleteButton = el("button", { type: "button", class: "invalid-delete", "aria-label": `Delete ${original}`, title: `Delete ${original}` }, guiIcon("trash"));
  const element = el("tr", {}, el("th", { scope: "row" }, pathButton(rel, shown, openFolder)), ...cells, el("td", {}, deleteButton, applyButton));
  return {
    element,
    relParts: rel,
    applyButton,
    deleteButton,
    setBlocked: (blocked) => {
      applyButton.disabled = deleteButton.disabled = blocked;
    },
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
      // A redraw would restart the fade, so a row keeps its element while its name and action stay the same.
      const choice = current();
      const key = `${next ?? ""}|${choice}`;
      if (key === applied) return;
      applied = key;
      shown.replaceChildren(nameFor(next, choice));
    },
  };
}

function lineBreaks(lines: string[]): (Node | string)[] {
  return lines.flatMap((line, i) => (i === 0 ? [line] : [el("br"), line]));
}

// The parent path shows only its last folder, as "...\<dir>\"; the button opens the folder in the file explorer.
function pathButton(rel: string[], name: HTMLElement, openFolder: (relParts: string[]) => void): HTMLButtonElement {
  const parents = rel.slice(0, -1);
  const trail = parents.length > 1 ? `...\\${parents[parents.length - 1] ?? ""}\\` : parents.map((part) => `${part}\\`).join("");
  const button = el("button", { type: "button", class: "invalid-path", title: "Open in Explorer" }, trail, name);
  button.addEventListener("click", () => openFolder(rel));
  return button;
}