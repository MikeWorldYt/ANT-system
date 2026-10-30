// Type and flag text for a node, shown in the details block and as row tooltips.
import type { EditableType, TreeNode, WkbStatus } from "../ipc/types";
import { state } from "./state";

export const TYPES: [EditableType, string][] = [["NODE", "Node"], ["WKB", "Workboard"], ["PORTF", "Portfolio"]];
export const STATUSES: [WkbStatus, string][] = [
  ["hold", "Hold"],
  ["in-progress", "In progress"],
  ["for-review", "For review"],
  ["done", "Done"],
];

export const statusLabel = (status: WkbStatus): string => STATUSES.find(([v]) => v === status)?.[1] ?? status;

export const UNKNOWN = "unknown";
export const nameWithoutIdentifier = (n: TreeNode): string => {
  const isDirectoryIdentifier = /^\d+\.$/.test(n.fixedPrefix) || /^[A-Za-z]+\.\d+_$/.test(n.fixedPrefix);
  return isDirectoryIdentifier ? n.name.slice(n.fixedPrefix.length) : n.name;
};
export const displayNodeName = (n: TreeNode): string => (state.settings.hideDirectoryIdentifiers ? nameWithoutIdentifier(n) : n.name);
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
  flags.push(...n.flags);
  return flags;
}

// Non-editable types (reviews, resources, templates) keep their stored label.
export const typeName = (n: TreeNode): string => TYPES.find(([v]) => v === n.nodeType?.base)?.[1] ?? n.nodeType?.raw ?? "—";

// Flags sit in parentheses after the type, e.g. "Node (reserved)".
export const typeLabel = (n: TreeNode): string => {
  const flags = nodeFlags(n);
  return flags.length > 0 ? `${typeName(n)} (${flags.join(", ")})` : typeName(n);
};

export const nodeSummary = (n: TreeNode): string => typeLabel(n);

export function nodeDetails(n: TreeNode): [string, string][] {
  const rows: [string, string][] = [["Name", n.name], ["Type", typeLabel(n)]];
  const wkb = isWorkboard(n);
  if (wkb) rows.push(["Status", n.status ? statusLabel(n.status) : UNKNOWN]);
  if (wkb) rows.push(["Owner", ownerOf(n) ?? UNKNOWN]);
  if (n.info?.error) rows.push(["Info", n.info.error]);
  for (const f of n.info?.fields ?? []) {
    // Status and owner of a workboard already have their own rows.
    if (wkb && (f.key.toLowerCase() === "status" || OWNER_KEYS.includes(f.key.toLowerCase()))) continue;
    // Favorite is toggled from the quick actions, never edited as a field.
    if (f.key.toLowerCase() === "favorite") continue;
    rows.push([f.key.charAt(0).toUpperCase() + f.key.slice(1), f.value]);
  }
  return rows;
}
