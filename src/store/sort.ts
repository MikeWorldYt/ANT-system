// Pure list ordering shared by the store and the columns view.
import { STATUSES } from "./node-info";
import type { TreeNode } from "../ipc/types";

export type SortBy = "name" | "type" | "status";
export interface ColumnSort {
  by: SortBy;
  dir: "asc" | "desc";
}

export const SORT_BYS: readonly SortBy[] = ["name", "type", "status"];

// The 90-99 folders and 998/999 stay at the end of a list whatever the sort direction.
export function isTail(name: string): boolean {
  const n = Number(/^(\d+)\./.exec(name)?.[1]);
  return (n >= 90 && n <= 99) || n === 998 || n === 999;
}

const sortValue: Record<SortBy, (n: TreeNode) => string | number> = {
  name: () => 0,
  type: (n) => n.nodeType?.raw ?? "",
  status: (n) => STATUSES.findIndex(([v]) => v === n.status) + 1 || STATUSES.length + 1,
};

// Favorites go above the rest of their list when pinned; the tail folders still close it.
export function favoritesFirst(nodes: TreeNode[], pin: boolean): TreeNode[] {
  if (!pin) return nodes;
  const body = nodes.filter((n) => !isTail(n.name));
  return [...body.filter((n) => n.favorite), ...body.filter((n) => !n.favorite), ...nodes.filter((n) => isTail(n.name))];
}

// Ties keep the incoming name order, and the tail folders close the list in either direction.
export function sortedForColumns(nodes: TreeNode[], { by, dir }: ColumnSort, pin: boolean): TreeNode[] {
  const head = nodes.filter((n) => !isTail(n.name));
  const key = sortValue[by];
  const ordered = [...head].sort((a, b) => (key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : 0));
  if (dir === "desc") ordered.reverse();
  return favoritesFirst([...ordered, ...nodes.filter((n) => isTail(n.name))], pin);
}
