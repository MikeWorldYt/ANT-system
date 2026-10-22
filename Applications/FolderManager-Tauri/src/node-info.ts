// Type and flag text for a node, shown in the details block and as row tooltips.
import type { TreeNode } from "./types";

export function nodeFlags(n: TreeNode): string[] {
  const flags: string[] = [];
  if (n.ghost) flags.push("missing on disk");
  if (n.locked) flags.push("locked");
  if (n.reserved) flags.push("reserved");
  if (n.isGit) flags.push("git project");
  return flags;
}

export const nodeSummary = (n: TreeNode): string => [n.nodeType?.raw, ...nodeFlags(n)].filter(Boolean).join(" · ");

// Both nodeType and flags can be empty, so the caller decides whether to hide the block.
export function nodeDetails(n: TreeNode): [string, string][] {
  const rows: [string, string][] = [["Type", n.nodeType?.raw ?? "—"]];
  const flags = nodeFlags(n);
  if (flags.length > 0) rows.push(["Flags", flags.join(", ")]);
  return rows;
}
