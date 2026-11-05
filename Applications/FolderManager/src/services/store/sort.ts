// Pure list ordering shared by the store and the columns view.
import { STATUSES } from "./node-info";
import type { TreeNode } from "../ipc/types";

export type SortBy = "track" | "name" | "status";
export interface ColumnSort {
  by: SortBy;
  dir: "asc" | "desc";
}

export const SORT_BYS: readonly SortBy[] = ["track", "name", "status"];

// The 90-99 folders and 998/999 stay at the end of a list whatever the sort direction.
export function isTail(name: string): boolean {
  const n = Number(/^(\d+)\./.exec(name)?.[1]);
  return (n >= 90 && n <= 99) || n === 998 || n === 999;
}

const statusRank = (n: TreeNode): number => STATUSES.findIndex(([v]) => v === n.status) + 1 || STATUSES.length + 1;

const sortValue: Record<SortBy, (n: TreeNode) => string | number> = {
  // The backend already lists each level by its id or tracker number.
  track: () => 0,
  name: (n) => (n.editableName || n.name).toLocaleLowerCase(),
  status: statusRank,
};

const isPortfolio = (n: TreeNode): boolean => n.nodeType?.base === "OUT";

// Portfolios head every list, even above favorites; pinned favorites go above the rest, and the tail folders still close it.
export function favoritesFirst(nodes: TreeNode[], pin: boolean): TreeNode[] {
  const heads = nodes.filter(isPortfolio);
  const rest = nodes.filter((n) => !isPortfolio(n));
  if (!pin) return [...heads, ...rest];
  const body = rest.filter((n) => !isTail(n.name));
  return [...heads, ...body.filter((n) => n.favorite), ...body.filter((n) => !n.favorite), ...rest.filter((n) => isTail(n.name))];
}

// Folders come before files in every list, whatever the sort direction.
export function foldersFirst(nodes: TreeNode[]): TreeNode[] {
  return [...nodes.filter((n) => !n.file), ...nodes.filter((n) => n.file)];
}

// Ties keep the incoming name order, and the tail folders close the list in either direction.
export function sortedForColumns(nodes: TreeNode[], { by, dir }: ColumnSort, pin: boolean): TreeNode[] {
  const head = nodes.filter((n) => !isTail(n.name));
  const key = sortValue[by];
  const ordered = [...head].sort((a, b) => {
    const [x, y] = [key(a), key(b)];
    return typeof x === "string" && typeof y === "string" ? x.localeCompare(y, undefined, { numeric: true }) : x < y ? -1 : x > y ? 1 : 0;
  });
  if (dir === "desc") ordered.reverse();
  return foldersFirst(favoritesFirst([...ordered, ...nodes.filter((n) => isTail(n.name))], pin));
}

// Board cards come from many folders, so the track is read from each name instead of the backend's per-level order.
export function sortedCards(nodes: TreeNode[], { by, dir }: ColumnSort): TreeNode[] {
  const text = (n: TreeNode) => (by === "name" ? n.editableName || n.name : n.name);
  const ordered = [...nodes].sort((a, b) =>
    by === "status" ? statusRank(a) - statusRank(b) : text(a).localeCompare(text(b), undefined, { numeric: true, sensitivity: "base" }),
  );
  return dir === "desc" ? ordered.reverse() : ordered;
}
