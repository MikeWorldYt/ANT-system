// Gives a folder's numbered children and worknodes new ids: a moved row takes a free id that fits its place, or a typed one.
import { el } from "../../../dom";
import { cancelButton, dialogButton } from "../../../atoms/button";
import { dialogText, openDialog } from "../dialog-shell";
import { idList, type IdGroup, type RowResult } from "./reorder-list";
import { idOf, idProblem, keyFor, padId, takenKey, withId, type IdKind } from "../../../../services/store/id-order";
import type { IdTarget, ReorderPlan, TreeNode } from "../../../../services/ipc/types";

export interface ReorderAnswer {
  targets: IdTarget[];
  renames: Map<string, string>;
}

const INTRO = "Drag a folder, or focus it and press Alt+Up or Alt+Down: it takes a free id that fits its new place, and the others keep theirs when they can. Or type its new id: digits only, 01 to 99 for nodes and 001 or more for worknodes.";

const groupOf = (kind: IdKind, title: string, names: string[], root: boolean, taken: Set<string>): IdGroup =>
  ({ kind, title, root, order: [...names], ids: new Map(names.map((n) => [n, idOf(n, kind)])), typed: new Map(), taken });

// The same checks the backend repeats on save: a usable id, held by no other folder here.
function results(groups: IdGroup[], taken: Set<string>): Map<string, RowResult> {
  const rows = groups.flatMap((g) =>
    g.order.map((name) => {
      const digits = g.ids.get(name) ?? idOf(name, g.kind);
      const problem = idProblem(digits, g.kind, g.root);
      return { g, name, problem, id: problem === null ? padId(digits, g.kind, g.root) : digits, key: keyFor(name, g.kind, digits) };
    }),
  );
  const counts = new Map<string, number>();
  for (const r of rows) if (r.problem === null) counts.set(r.key, (counts.get(r.key) ?? 0) + 1);
  const out = new Map<string, RowResult>();
  for (const r of rows) {
    const clash = taken.has(r.key) ? "Another folder here has this id" : (counts.get(r.key) ?? 0) > 1 ? "Two folders would share this id" : null;
    const problem = r.problem ?? clash;
    out.set(r.name, { name: problem === null ? withId(r.name, r.g.kind, r.id) : r.name, id: r.id, problem });
  }
  return out;
}

// `plan` lists each group in id order; resolves to every row's new id, or null when cancelled.
export async function reorderDialog(folder: string, plan: ReorderPlan, icons: Map<string, TreeNode>, root: boolean): Promise<ReorderAnswer | null> {
  const taken = new Set(plan.taken.map(takenKey));
  const groups = [groupOf("node", "Nodes", plan.nodes, root, taken), groupOf("worknode", "Worknodes", plan.worknodes, root, taken)].filter((g) => g.order.length > 0);
  const status = el("div", { class: "sr-only", role: "status", "aria-live": "polite" });
  const save = dialogButton("Save", "ok", "primary");
  let current = new Map<string, RowResult>();

  function refresh(): void {
    current = results(groups, taken);
    for (const list of lists) list.update(current);
    const unchanged = groups.every((g) => g.order.every((name) => current.get(name)?.name === name));
    save.disabled = unchanged || [...current.values()].some((r) => r.problem !== null);
  }

  const lists = groups.map((g) => idList(g, icons, refresh, (text) => (status.textContent = text)));
  refresh();
  const { closed } = openDialog({
    title: `Reorder by id: ${folder}`,
    wide: true,
    body: [dialogText(INTRO), el("div", { class: "reorder-scroll" }, ...lists.map((l) => l.element)), status],
    buttons: [cancelButton(), save],
    focus: () => lists[0]?.element.querySelector<HTMLElement>("li")?.focus(),
  });
  if ((await closed) !== "ok") return null;
  const targets = groups.flatMap((g) => g.order.map((name) => ({ name, newId: current.get(name)?.id ?? idOf(name, g.kind) })));
  const renames = new Map([...current].filter(([old, res]) => res.name !== old).map(([old, res]) => [old, res.name]));
  return { targets, renames };
}
