// Type and flag text for a node, shown in the details block and as row tooltips.
import type { TreeNode, WkbStatus } from "./types";

export const TYPES: [string, string][] = [["NODE", "Node"], ["WKB", "Workboard"], ["PORTF", "Portfolio"]];
export const STATUSES: [WkbStatus, string][] = [
  ["hold", "Hold"],
  ["in-progress", "In progress"],
  ["for-review", "For review"],
  ["done", "Done"],
];

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
