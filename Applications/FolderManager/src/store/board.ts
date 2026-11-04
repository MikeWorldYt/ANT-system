// Task board placement: the column and order a workboard was given, kept in its info.json with the status it had then.
import type { BoardCard, BoardColumn, TreeNode, WkbStatus } from "../ipc/types";

export const BOARD_COLUMNS: [BoardColumn, string][] = [
  ["backlog", "Backlog"],
  ["todo", "Todo"],
  ["progress", "In progress"],
  ["done", "Done"],
];

// Dropping on a column sets this status; Backlog keeps whatever the card has.
export const COLUMN_STATUS: Partial<Record<BoardColumn, WkbStatus>> = { todo: "hold", progress: "in-progress", done: "done" };

export type BoardLists = Record<BoardColumn, TreeNode[]>;

const field = (n: TreeNode, key: string): string | undefined => n.info?.fields.find((f) => f.key.toLowerCase() === key)?.value;
const storedColumn = (n: TreeNode): BoardColumn | null => BOARD_COLUMNS.find(([c]) => c === field(n, "board"))?.[0] ?? null;
const sameStatus = (n: TreeNode): boolean => field(n, "boardstatus") === (n.statusRaw ?? undefined);
const keyOf = (n: TreeNode): string => n.relParts.join("/");

export function columnOf(n: TreeNode): BoardColumn {
  const stored = storedColumn(n);
  if (stored !== null && sameStatus(n)) return stored;
  if (n.status === "hold") return "todo";
  if (n.status === "done") return "done";
  // A card never placed starts in Backlog; one whose status changed elsewhere, back from hold or done, goes to In progress.
  return stored === null ? "backlog" : "progress";
}

// Cards never ordered follow the ordered ones, in the path order the list came in.
function orderOf(n: TreeNode): number {
  const order = Number(field(n, "boardorder") ?? Number.NaN);
  return Number.isFinite(order) ? order : Number.POSITIVE_INFINITY;
}

const byOrder = (a: TreeNode, b: TreeNode): number => (orderOf(a) === orderOf(b) ? 0 : orderOf(a) < orderOf(b) ? -1 : 1);

// Important cards head their column; each group keeps its own manual order.
export function boardLists(nodes: TreeNode[]): BoardLists {
  const lists: BoardLists = { backlog: [], todo: [], progress: [], done: [] };
  for (const n of nodes) lists[columnOf(n)].push(n);
  for (const list of Object.values(lists)) list.sort((a, b) => Number(b.important) - Number(a.important) || byOrder(a, b));
  return lists;
}

// The moved card and every card of the column whose stored place would no longer match; `index` counts the others.
export function placement(lists: BoardLists, card: TreeNode, to: BoardColumn, index: number): BoardCard[] {
  const list = lists[to].filter((c) => keyOf(c) !== keyOf(card));
  // A card only moves inside its own group, so the important ones stay on top.
  const pinned = list.filter((c) => c.important).length;
  const [min, max] = card.important ? [0, pinned] : [pinned, list.length];
  list.splice(Math.max(min, Math.min(index, max)), 0, card);
  return list.flatMap((c, order) => {
    const placed = c !== card && storedColumn(c) === to && sameStatus(c) && field(c, "boardorder") === String(order);
    return placed ? [] : [{ relParts: c.relParts, column: to, order }];
  });
}
