// One group of the reorder dialog: rows to select like the views, moved as a block by pointer or Alt+Arrow; HTML5 drag events never reach the webview.
import { countAbove, pastThreshold, pointOf, released, type Point } from "../../../../hooks/pointer";
import { el } from "../../../dom";
import { folderIcon, kindIcon } from "../../../atoms/icons";
import { fitBlock } from "../../../../services/store/id-fit";
import { idRow, paintRow, rowKey, type IdGroup, type RowResult } from "./reorder-row";
import type { TreeNode } from "../../../../services/ipc/types";

export type { IdGroup, RowResult } from "./reorder-row";

const LIFTED = "is-lifted";
const SORTING = "is-sorting";
const DROP_BEFORE = "drop-before";
const DROP_AFTER = "drop-after";

interface Drag {
  name: string;
  block: string[];
  start: Point;
  lifted: boolean;
  slot: number | null;
  // A press on a row of a larger selection narrows it to that row, unless it turns into a drag.
  narrow: boolean;
}

let nextId = 0;

export function idList(group: IdGroup, icons: Map<string, TreeNode>, onChange: () => void, announce: (text: string) => void): { element: HTMLElement; update: (results: Map<string, RowResult>) => void } {
  const headingId = `reorder-group-${++nextId}`;
  const list = el("ul", { class: "reorder-list", "aria-labelledby": headingId });
  let drag: Drag | null = null;
  let last = new Map<string, RowResult>();
  let selected = new Set<string>();
  let anchor: string | null = null;

  const rows = (): HTMLElement[] => Array.from(list.children) as HTMLElement[];
  const rowOf = (target: EventTarget | null): HTMLElement | null => (target instanceof Element ? target.closest<HTMLElement>("li[data-name]") : null);
  const inControl = (target: EventTarget | null): boolean => target instanceof Element && target.closest("input, select") !== null;
  const nameOf = (li: HTMLElement): string => li.dataset.name ?? "";
  const rowNamed = (name: string): HTMLElement | undefined => rows()[group.order.indexOf(name)];
  const blockOf = (name: string): string[] => (selected.has(name) ? group.order.filter((n) => selected.has(n)) : [name]);

  function selectOnly(name: string): void {
    selected = new Set([name]);
    anchor = name;
  }

  function selectRange(name: string): void {
    const [a, b] = [group.order.indexOf(anchor ?? name), group.order.indexOf(name)];
    selected = new Set(group.order.slice(Math.min(a, b), Math.max(a, b) + 1));
  }

  function toggle(name: string): void {
    if (!selected.delete(name)) selected.add(name);
    anchor = name;
  }

  function update(results: Map<string, RowResult>): void {
    last = results;
    rows().forEach((li, i) => {
      const name = group.order[i] ?? "";
      const res = results.get(name);
      if (res) paintRow(li, group, name, res, i, selected.has(name));
    });
  }

  const hooks = {
    onId: (name: string, digits: string) => {
      group.ids.set(name, digits);
      onChange();
    },
    // With several worknodes selected, the tracker picked on one of them applies to all.
    onTracker: (name: string, tracker: string) => {
      for (const n of blockOf(name)) group.trackers.set(n, tracker);
      render();
      onChange();
      rowNamed(name)?.querySelector("select")?.focus();
    },
    announce,
  };

  function render(): void {
    list.replaceChildren(...group.order.map((name, i) => {
      const node = icons.get(name);
      return idRow(group, name, i, headingId, node ? folderIcon(node) : kindIcon(group.kind === "worknode" ? "wkb" : "folder"), hooks);
    }));
    update(last);
  }

  const placed = (block: string[], at: number): string[] => {
    const rest = group.order.filter((n) => !block.includes(n));
    rest.splice(at, 0, ...block);
    return rest;
  };
  const sameOrder = (order: string[]) => order.every((n, i) => n === group.order[i]);

  function moveBlock(block: string[], at: number, focus: string): void {
    const order = placed(block, at);
    if (sameOrder(order)) return;
    group.order = order;
    group.ids = fitBlock({ ...group, key: (name, digits) => rowKey(group, name, digits) }, block);
    render();
    onChange();
    // The rows are rebuilt, so focus follows the row it was on.
    rowNamed(focus)?.focus();
    const where = group.order.indexOf(block[0] ?? "") + 1;
    announce(`${block.length > 1 ? `${block.length} folders` : (block[0] ?? "")} moved to position ${where} of ${group.order.length}.`);
  }

  // Alt+Arrow moves the block one row past its neighbour; a scattered selection gathers where its first row is.
  function step(name: string, up: boolean): void {
    const block = blockOf(name);
    const top = group.order.indexOf(block[0] ?? "");
    const bottom = group.order.indexOf(block[block.length - 1] ?? "") - (block.length - 1);
    moveBlock(block, up ? Math.max(0, top - 1) : Math.min(group.order.length - block.length, bottom + 1), name);
  }

  // The gap under the pointer, counted in rows above it; a drop that changes nothing has none.
  function slotAt(d: Drag, y: number): number | null {
    const slot = countAbove(rows(), y);
    const at = slot - d.block.filter((n) => group.order.indexOf(n) < slot).length;
    return sameOrder(placed(d.block, at)) ? null : slot;
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
    for (const r of rows()) r.classList.remove(LIFTED);
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
    const li = ev.button === 0 && !inControl(ev.target) ? rowOf(ev.target) : null;
    if (!li) return;
    const name = nameOf(li);
    if (ev.ctrlKey || ev.metaKey || ev.shiftKey) {
      if (ev.shiftKey) selectRange(name);
      else toggle(name);
      return update(last);
    }
    const narrow = selected.has(name) && selected.size > 1;
    if (!selected.has(name)) selectOnly(name);
    update(last);
    drag = { name, block: blockOf(name), start: pointOf(ev), lifted: false, slot: null, narrow };
  });
  list.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    if (released(ev)) return end();
    if (!drag.lifted) {
      if (!pastThreshold(drag.start, pointOf(ev))) return;
      drag.lifted = true;
      for (const n of drag.block) rowNamed(n)?.classList.add(LIFTED);
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
    if (!done) return;
    if (done.lifted && done.slot !== null) {
      const slot = done.slot;
      return moveBlock(done.block, slot - done.block.filter((n) => group.order.indexOf(n) < slot).length, done.name);
    }
    if (!done.lifted && done.narrow) {
      selectOnly(done.name);
      update(last);
    }
  });
  list.addEventListener("pointercancel", end);
  list.addEventListener("keydown", (ev) => {
    const li = rowOf(ev.target);
    if (!li || ev.ctrlKey || ev.metaKey) return;
    const name = nameOf(li);
    const arrow = ev.key === "ArrowUp" || ev.key === "ArrowDown";
    if (ev.altKey && !ev.shiftKey && arrow) {
      ev.preventDefault();
      return step(name, ev.key === "ArrowUp");
    }
    // Inside a field the keys type and move the caret, so only the row itself selects.
    if (ev.target !== li || ev.altKey) return;
    if (ev.key === " ") {
      ev.preventDefault();
      toggle(name);
      return update(last);
    }
    if (!arrow) return;
    ev.preventDefault();
    const next = group.order[group.order.indexOf(name) + (ev.key === "ArrowUp" ? -1 : 1)];
    if (next === undefined) return;
    if (ev.shiftKey) {
      if (anchor === null) anchor = name;
      selectRange(next);
    } else selectOnly(next);
    update(last);
    rowNamed(next)?.focus();
  });

  render();
  return { element: el("section", { class: "reorder-group" }, el("h3", { id: headingId }, group.title), list), update };
}
