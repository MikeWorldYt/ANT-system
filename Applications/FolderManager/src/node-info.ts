// Type and flag text for a node, shown in the details block and as row tooltips.
import type { TreeNode, WkbStatus } from "./types";

export const TYPES: [string, string][] = [["NODE", "Node"], ["WKB", "Workboard"], ["PORTF", "Portfolio"]];
export const STATUSES: [WkbStatus, string][] = [
  ["in-progress", "In progress"],
  ["for-review", "For review"],
  ["hold", "Hold"],
  ["done", "Done"],
];

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
  if (n.status) rows.push(["Status", STATUSES.find(([v]) => v === n.status)?.[1] ?? n.status]);
  const flags = nodeFlags(n);
  if (flags.length > 0) rows.push(["Flags", flags.join(", ")]);
  if (n.info?.error) rows.push(["Info", n.info.error]);
  for (const f of n.info?.fields ?? []) {
    // A recognised status already has its own row.
    if (f.key.toLowerCase() === "status" && n.status) continue;
    rows.push([f.key.charAt(0).toUpperCase() + f.key.slice(1), f.value]);
  }
  return rows;
}
