// One group of the reorder dialog: rows that move by pointer or Alt+Arrow, each with its id to type; HTML5 drag events never reach the webview.
import { countAbove, pastThreshold, pointOf, released, type Point } from "../../../../hooks/pointer";
import { el } from "../../../dom";
import { folderIcon, kindIcon } from "../../../atoms/icons";
import { idOf, padId, type IdKind } from "../../../../services/store/id-order";
import { fitMoved } from "../../../../services/store/id-fit";
import type { TreeNode } from "../../../../services/ipc/types";

const LIFTED = "is-lifted";
const SORTING = "is-sorting";
const DROP_BEFORE = "drop-before";
const DROP_AFTER = "drop-after";
const BAD_KEY = "is-bad-key";
const BAD_KEY_MS = 600;

export interface IdGroup {
  kind: IdKind;
  title: string;
  root: boolean;
  order: string[];
  // Every row's new id as digits; starts as its current id.
  ids: Map<string, string>;
  typed: Map<string, string>;
  taken: Set<string>;
}

export interface RowResult {
  name: string;
  id: string;
  problem: string | null;
}

interface Drag {
  row: HTMLElement;
  from: number;
  start: Point;
  lifted: boolean;
  slot: number | null;
}

let nextId = 0;

export function idList(group: IdGroup, icons: Map<string, TreeNode>, onChange: () => void, announce: (text: string) => void): { element: HTMLElement; update: (results: Map<string, RowResult>) => void } {
  const headingId = `reorder-group-${++nextId}`;
  const list = el("ul", { class: "reorder-list", "aria-labelledby": headingId });
  let drag: Drag | null = null;
  let last = new Map<string, RowResult>();

  const rows = (): HTMLElement[] => Array.from(list.children) as HTMLElement[];
  const rowOf = (target: EventTarget | null): HTMLElement | null => (target instanceof Element ? target.closest<HTMLElement>("li[data-index]") : null);
  const titleOf = (name: string): string => {
    const id = idOf(name, group.kind);
    return name.slice(name.indexOf(id) + id.length).replace(/^[._]/, "");
  };

  function rejectKey(input: HTMLInputElement): void {
    input.classList.add(BAD_KEY);
    window.setTimeout(() => input.classList.remove(BAD_KEY), BAD_KEY_MS);
    announce("Ids take digits only.");
  }

  function idInput(name: string): HTMLInputElement {
    const input = el("input", { class: "reorder-input", type: "text", inputmode: "numeric", maxlength: "6", "aria-label": `New id for ${name}`, value: group.typed.get(name) ?? idOf(name, group.kind) });
    input.addEventListener("beforeinput", (ev) => {
      if (ev.data === null || /^\d*$/.test(ev.data)) return;
      ev.preventDefault();
      rejectKey(input);
    });
    // Drops and autofill can skip beforeinput, so whatever still slips in is taken out.
    input.addEventListener("input", () => {
      if (/\D/.test(input.value)) {
        input.value = input.value.replace(/\D/g, "");
        rejectKey(input);
      }
      group.typed.set(name, input.value);
      group.ids.set(name, input.value);
      onChange();
    });
    const commit = () => {
      if (input.value === "" || !group.typed.has(name)) return;
      input.value = padId(input.value, group.kind, group.root);
      group.typed.set(name, input.value);
      group.ids.set(name, input.value);
      onChange();
    };
    input.addEventListener("blur", commit);
    // Enter would submit the dialog form, so it only commits the id.
    input.addEventListener("keydown", (ev) => {
      if (ev.key !== "Enter") return;
      ev.preventDefault();
      commit();
    });
    return input;
  }

  function row(name: string, index: number): HTMLElement {
    const node = icons.get(name);
    const problemId = `${headingId}-p${index}`;
    const input = idInput(name);
    input.setAttribute("aria-describedby", problemId);
    return el(
      "li",
      { tabindex: "0", "data-index": String(index), "data-name": name },
      node ? folderIcon(node) : kindIcon(group.kind === "worknode" ? "wkb" : "folder"),
      input,
      el("span", { class: "reorder-title" }, titleOf(name)),
      el("span", { class: "reorder-arrow", "aria-hidden": "true" }, "→"),
      el("span", { class: "reorder-new" }),
      el("span", { class: "reorder-problem", id: problemId }),
    );
  }

  function update(results: Map<string, RowResult>): void {
    last = results;
    const count = group.order.length;
    rows().forEach((li, i) => {
      const name = group.order[i] ?? "";
      const res = results.get(name);
      if (!res) return;
      const same = res.name === name && res.problem === null;
      li.classList.toggle("is-unchanged", same);
      li.setAttribute("aria-label", `${name}, ${i + 1} of ${count}, ${res.problem ?? (same ? "unchanged" : `new id ${res.id}`)}`);
      li.querySelector(".reorder-input")?.setAttribute("aria-invalid", String(res.problem !== null));
      const shown = li.querySelector(".reorder-new");
      if (shown) shown.textContent = same ? "unchanged" : res.id;
      const problem = li.querySelector(".reorder-problem");
      if (problem) problem.textContent = res.problem ?? "";
    });
  }

  function render(): void {
    list.replaceChildren(...group.order.map(row));
    update(last);
  }

  function move(from: number, to: number): void {
    const name = group.order[from];
    if (name === undefined || to === from || to < 0 || to >= group.order.length) return;
    group.order = group.order.filter((_, i) => i !== from);
    group.order.splice(to, 0, name);
    group.ids = fitMoved(group, name);
    // A shifted row that had a typed id shows the id it now takes.
    for (const [typed, digits] of group.typed) if (group.ids.get(typed) !== digits) group.typed.set(typed, group.ids.get(typed) ?? digits);
    render();
    onChange();
    // The rows are rebuilt, so focus follows the moved folder to its new row.
    rows()[to]?.focus();
    announce(`${name} moved to position ${to + 1} of ${group.order.length}.`);
  }

  // The gap under the pointer, counted in rows above it; both edges of the dragged row leave it in place.
  function slotAt(d: Drag, y: number): number | null {
    const slot = countAbove(rows(), y);
    return slot === d.from || slot === d.from + 1 ? null : slot;
  }

  function mark(slot: number | null): void {
    const all = rows();
    for (const r of all) r.classList.remove(DROP_BEFORE, DROP_AFTER);
    if (slot === null) return;
    if (slot < all.length) all[slot]?.classList.add(DROP_BEFORE);
    else all[all.length - 1]?.classList.add(DROP_AFTER);
  }

  function end(): void {
    if (!drag) return;
    drag.row.classList.remove(LIFTED);
    list.classList.remove(SORTING);
    mark(null);
    document.removeEventListener("keydown", onEscape, true);
    window.removeEventListener("blur", end);
    drag = null;
  }

  // Captured so Escape cancels the drag without also closing the dialog.
  function onEscape(ev: KeyboardEvent): void {
    if (ev.key !== "Escape") return;
    ev.preventDefault();
    ev.stopPropagation();
    end();
  }

  list.addEventListener("pointerdown", (ev) => {
    end();
    const inField = ev.target instanceof HTMLInputElement;
    const target = ev.button === 0 && !inField ? rowOf(ev.target) : null;
    if (target) drag = { row: target, from: rows().indexOf(target), start: pointOf(ev), lifted: false, slot: null };
  });
  list.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    if (released(ev)) return end();
    if (!drag.lifted) {
      if (!pastThreshold(drag.start, pointOf(ev))) return;
      drag.lifted = true;
      drag.row.classList.add(LIFTED);
      list.classList.add(SORTING);
      if (!list.hasPointerCapture(ev.pointerId)) list.setPointerCapture(ev.pointerId);
      document.addEventListener("keydown", onEscape, true);
      window.addEventListener("blur", end);
    }
    drag.slot = slotAt(drag, ev.clientY);
    mark(drag.slot);
  });
  list.addEventListener("pointerup", () => {
    const done = drag;
    end();
    if (done?.lifted && done.slot !== null) move(done.from, done.slot > done.from ? done.slot - 1 : done.slot);
  });
  list.addEventListener("pointercancel", end);
  list.addEventListener("keydown", (ev) => {
    if (!ev.altKey || ev.ctrlKey || ev.metaKey || ev.shiftKey || (ev.key !== "ArrowUp" && ev.key !== "ArrowDown")) return;
    const target = rowOf(ev.target);
    if (!target) return;
    ev.preventDefault();
    const from = rows().indexOf(target);
    move(from, ev.key === "ArrowUp" ? from - 1 : from + 1);
  });

  render();
  return { element: el("section", { class: "reorder-group" }, el("h3", { id: headingId }, group.title), list), update };
}
