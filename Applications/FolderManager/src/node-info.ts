// Type and flag text for a node, shown in the details block and as row tooltips.
import type { ColumnSort, SortBy } from "./state";
import type { TreeNode, WkbStatus } from "./types";

export const TYPES: [string, string][] = [["NODE", "Node"], ["WKB", "Workboard"], ["PORTF", "Portfolio"]];
export const STATUSES: [WkbStatus, string][] = [
  ["hold", "Hold"],
  ["in-progress", "In progress"],
  ["for-review", "For review"],
  ["done", "Done"],
];

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

// Ties keep the incoming name order, and the tail folders close the list in either direction.
export function sortedForColumns(nodes: TreeNode[], { by, dir }: ColumnSort): TreeNode[] {
  const head = nodes.filter((n) => !isTail(n.name));
  const key = sortValue[by];
  const ordered = [...head].sort((a, b) => (key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : 0));
  if (dir === "desc") ordered.reverse();
  return [...ordered, ...nodes.filter((n) => isTail(n.name))];
}

export const UNKNOWN = "unknown";
const OWNER_KEYS = ["owner", "holder", "client"];

export const isWorkboard = (n: TreeNode) => n.nodeType?.base === "WKB";
export const ownerOf = (n: TreeNode): string | null =>
  n.info?.fields.find((f) => OWNER_KEYS.includes(f.key.toLowerCase()))?.value ?? null;

export function nodeFlags(n: TreeNode): string[] {
  const flags: string[] = [];
  if (n.ghost) flags.push("missing on disk");
  if (n.locked) flags.push("locked");
  if (n.reserved) flags.push("reserved");
  if (n.isGit) flags.push("git project");
  return flags;
}

export const nodeSummary = (n: TreeNode): string => [n.nodeType?.raw, ...nodeFlags(n)].filter(Boolean).join(" · ");

export function nodeDetails(n: TreeNode): [string, string][] {
  const rows: [string, string][] = [["Type", n.nodeType?.raw ?? "—"]];
  const wkb = isWorkboard(n);
  if (wkb) rows.push(["Status", STATUSES.find(([v]) => v === n.status)?.[1] ?? UNKNOWN]);
  if (wkb) rows.push(["Owner", ownerOf(n) ?? UNKNOWN]);
  const flags = nodeFlags(n);
  if (flags.length > 0) rows.push(["Flags", flags.join(", ")]);
  if (n.info?.error) rows.push(["Info", n.info.error]);
  for (const f of n.info?.fields ?? []) {
    // Status and owner of a workboard already have their own rows.
    if (wkb && (f.key.toLowerCase() === "status" || OWNER_KEYS.includes(f.key.toLowerCase()))) continue;
    rows.push([f.key.charAt(0).toUpperCase() + f.key.slice(1), f.value]);
  }
  return rows;
}
