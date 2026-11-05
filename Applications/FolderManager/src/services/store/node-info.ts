// Type and flag text for a node, shown in the details block and as row tooltips.
import type { EditableType, TreeNode, WkbStatus } from "../ipc/types";
import { state } from "./state";

export const TYPES: [EditableType, string][] = [["NODE", "Node"], ["WKB", "Workboard"]];
const TYPE_LABELS: [string, string][] = [...TYPES, ["OUT", "Outcomes"]];
export const STATUSES: [WkbStatus, string][] = [
  ["hold", "Hold"],
  ["in-progress", "In progress"],
  ["for-review", "For review"],
  ["done", "Done"],
];

export const statusLabel = (status: WkbStatus): string => STATUSES.find(([v]) => v === status)?.[1] ?? status;

export const importantLabel = (important: boolean): string => (important ? "Is not important" : "Is important");

export const UNKNOWN = "unknown";
export const nameWithoutIdentifier = (n: TreeNode): string => {
  // A review link's name carries its owner's id slots, so it shows the owner folder's own name instead.
  if (n.linked && n.linkPath) {
    const segments = n.linkPath.split("/");
    return (segments[segments.length - 2] ?? n.name).replace(/^[^.]*\./, "");
  }
  const isDirectoryIdentifier = /^\d+\.$/.test(n.fixedPrefix) || /^[A-Za-z]+\.\d+_$/.test(n.fixedPrefix);
  return isDirectoryIdentifier ? n.name.slice(n.fixedPrefix.length) : n.name;
};
// Review links always drop their identifier: their numbers repeat the category's, which reads as noise.
export const displayNodeName = (n: TreeNode): string => (n.linked || state.settings.hideDirectoryIdentifiers ? nameWithoutIdentifier(n) : n.name);
const OWNER_KEYS = ["owner", "holder", "client"];

export const isWorkboard = (n: TreeNode) => n.nodeType?.base === "WKB";
export const ownerOf = (n: TreeNode): string | null =>
  n.info?.fields.find((f) => OWNER_KEYS.includes(f.key.toLowerCase()))?.value ?? null;
export const outputOf = (n: TreeNode): string | null => n.info?.fields.find((f) => f.key.toLowerCase() === "output")?.value ?? null;

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
export const typeName = (n: TreeNode): string => (n.file ? "File" : TYPE_LABELS.find(([v]) => v === n.nodeType?.base)?.[1] ?? n.nodeType?.raw ?? "—");

// Flags sit in parentheses after the type, e.g. "Node (reserved)".
export const typeLabel = (n: TreeNode): string => {
  const flags = nodeFlags(n);
  return flags.length > 0 ? `${typeName(n)}\n(${flags.join(", ")})` : typeName(n);
};

// A tooltip is one line in plain words, so the internal type names give way to what the folder is for.
export const nodeSummary = (n: TreeNode): string => {
  if (!n.file && n.nodeType?.base === "REV") return "Review Folder";
  if (!n.file && n.nodeType?.base === "NODE" && n.flags.includes("data") && nodeFlags(n).length === 1) return "Node with free files unsorted";
  return typeLabel(n).replace(/\s*\n\s*/, " ");
};

export const isReviewRoot = (n: TreeNode) => n.relParts.length === 1 && n.name.toLowerCase() === "998.review";
export const isMainCategory = (n: TreeNode) => n.depth === 0 && !n.ghost && /^[1-8]00\./.test(n.name);

// A review link's path is its owner's folders plus the 98.* folder, so the owner's level is the segments before it.
export const linkLevel = (n: TreeNode): number | null => (n.linked && n.linkPath ? n.linkPath.split("/").length - 1 : null);
export const isTopLink = (n: TreeNode): boolean => linkLevel(n) === 1;
// One dash per level below the first, so the review list reads as a staircase.
export const linkDashes = (n: TreeNode): string => "-".repeat(Math.max(0, (linkLevel(n) ?? 1) - 1));
export const linkCategory = (n: TreeNode): string | null => n.linkPath?.split("/")[0] ?? null;

// The 998.Review filter is stored by the backend as fields of its info.json.
const filterField = (n: TreeNode, key: string): string | undefined =>
  n.info?.fields.find((f) => f.key.toLowerCase() === key)?.value;

// Categories the filter hides, as one comma-separated field.
export const hiddenCategories = (n: TreeNode): string[] =>
  (filterField(n, "hidden") ?? "")
    .split(",")
    .map((c) => c.trim())
    .filter((c) => c !== "");

// Favorites are on unless the backend stored "off".
export const favoritesShown = (n: TreeNode): boolean => filterField(n, "favorites") !== "off";

export function nodeDetails(n: TreeNode): [string, string][] {
  const rows: [string, string][] = [["Name", n.name]];
  const wkb = isWorkboard(n);
  if (wkb) rows.push(["Output", outputOf(n) ?? UNKNOWN]);
  if (wkb) rows.push(["Status", n.status ? statusLabel(n.status) : UNKNOWN]);
  if (n.linked) rows.push(["Path", n.linkPath ?? UNKNOWN]);
  else if (!isReviewRoot(n)) rows.push(["Type", typeLabel(n)]);
  if (wkb) rows.push(["Owner", ownerOf(n) ?? UNKNOWN]);
  if (n.info?.error) rows.push(["Info", n.info.error]);
  for (const f of n.info?.fields ?? []) {
    // Status, owner and output of a workboard already have their own rows.
    if (wkb && (f.key.toLowerCase() === "status" || f.key.toLowerCase() === "output" || OWNER_KEYS.includes(f.key.toLowerCase()))) continue;
    // Favorite is toggled from the quick actions, never edited as a field.
    if (f.key.toLowerCase() === "favorite") continue;
    // Important is toggled from the quick actions, like favorite.
    if (f.key.toLowerCase() === "important") continue;
    // The board placement changes by moving the card, never as a field.
    if (["board", "boardorder", "boardstatus"].includes(f.key.toLowerCase())) continue;
    // The review filter has its own checkboxes, so its stored values are not detail rows.
    if (isReviewRoot(n) && (f.key.toLowerCase() === "hidden" || f.key.toLowerCase() === "favorites")) continue;
    rows.push([f.key.charAt(0).toUpperCase() + f.key.slice(1), f.value]);
  }
  return rows;
}
