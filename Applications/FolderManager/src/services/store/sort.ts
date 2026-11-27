// Pure list ordering shared by the store and the columns view.
import { STATUSES } from "./node-info";
import { folderSizeOf } from "./folder-size";
import { inSystem, type TreeNode } from "../ipc/types";

export type SortBy = "track" | "name" | "status" | "modified" | "created" | "size";
export interface ColumnSort {
  by: SortBy;
  dir: "asc" | "desc";
}

export const SORT_BYS: readonly SortBy[] = ["track", "name", "status", "modified", "created", "size"];

// Support folders (REV., RES., TPL., SS., REC. and the 90/97/98 ids of older storages) and 998/999 stay at the end of a list whatever the sort direction.
export function isTail(name: string): boolean {
  return /^(REV|RES|TPL|SS|REC|ANT)\./i.test(name) || /^(90|97|98|998|999)\./.test(name);
}

// Downloads is not the storage, so its names follow no tail rule.
export const isTailNode = (n: TreeNode): boolean => !inSystem(n.relParts) && isTail(n.name);

const statusRank = (n: TreeNode): number => STATUSES.findIndex(([v]) => v === n.status) + 1 || STATUSES.length + 1;

const sortValue: Record<SortBy, (n: TreeNode) => string | number> = {
  // The backend already lists each level by its id or tracker number.
  track: () => 0,
  name: (n) => (n.editableName || n.name).toLocaleLowerCase(),
  status: statusRank,
  // Ghosts and links have no size or stamp, and a folder has one only once its total is known, so they sort as the smallest.
  modified: (n) => n.stamps.modified ?? -1,
  created: (n) => n.stamps.created ?? -1,
  size: (n) => n.stamps.size ?? folderSizeOf(n.relParts.join("/")) ?? -1,
};

const isPortfolio = (n: TreeNode): boolean => n.nodeType?.base === "OUT";

// Portfolios head every list, even above favorites; pinned favorites go above the rest, and the tail folders still close it.
export function favoritesFirst(nodes: TreeNode[], pin: boolean): TreeNode[] {
  const heads = nodes.filter(isPortfolio);
  const rest = nodes.filter((n) => !isPortfolio(n));
  if (!pin) return [...heads, ...rest];
  const body = rest.filter((n) => !isTailNode(n));
  return [...heads, ...body.filter((n) => n.favorite), ...body.filter((n) => !n.favorite), ...rest.filter(isTailNode)];
}

// Folders come before files in every list, whatever the sort direction.
export function foldersFirst(nodes: TreeNode[]): TreeNode[] {
  return [...nodes.filter((n) => !n.file), ...nodes.filter((n) => n.file)];
}

// Ties keep the incoming name order, and the tail folders close the list in either direction, below the files too: they support the content, they are not part of it.
export function sortedForColumns(nodes: TreeNode[], { by, dir }: ColumnSort, pin: boolean): TreeNode[] {
  const head = nodes.filter((n) => !isTailNode(n));
  const key = sortValue[by];
  const ordered = [...head].sort((a, b) => {
    const [x, y] = [key(a), key(b)];
    return typeof x === "string" && typeof y === "string" ? x.localeCompare(y, undefined, { numeric: true }) : x < y ? -1 : x > y ? 1 : 0;
  });
  if (dir === "desc") ordered.reverse();
  const listed = foldersFirst(favoritesFirst([...ordered, ...nodes.filter(isTailNode)], pin));
  return [...listed.filter((n) => !isTailNode(n)), ...listed.filter(isTailNode)];
}

// Board cards come from many folders, so the track is read from each name instead of the backend's per-level order.
export function sortedCards(nodes: TreeNode[], { by, dir }: ColumnSort): TreeNode[] {
  const text = (n: TreeNode) => (by === "name" ? n.editableName || n.name : n.name);
  const ordered = [...nodes].sort((a, b) =>
    by === "status" || by === "modified" || by === "created" || by === "size" ? Number(sortValue[by](a)) - Number(sortValue[by](b)) : text(a).localeCompare(text(b), undefined, { numeric: true, sensitivity: "base" }),
  );
  return dir === "desc" ? ordered.reverse() : ordered;
}
