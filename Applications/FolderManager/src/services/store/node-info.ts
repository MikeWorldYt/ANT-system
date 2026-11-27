// Type and flag text for a node, shown in the details block and as row tooltips.
import type { EditableType, TreeNode, WkbStatus } from "../ipc/types";
import { state } from "./state";
import { extensionsShown } from "./view-prefs";

export const TYPES: [EditableType, string][] = [["NODE", "Node"], ["WKB", "Worknode"]];
const TYPE_LABELS: [string, string][] = [...TYPES, ["OUT", "Outcomes"], ["RES", "Resource Node"], ["REV", "Review Node"], ["SS", "Screenshots Node"], ["REC", "Recordings Node"], ["TPL", "Templates Node"]];
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
const GLOBAL_PREFIX = /^ANT\./i;
const SUPPORT_LABELS: Record<string, string> = { REC: "Recordings", SS: "Screenshots", REV: "Review", RES: "Resources" };

// Folder names keep their prefix on disk; the app reads them as words: "REC.WORK" is "Work Recordings".
function readable(text: string): string {
  return text
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((word) => (word === word.toUpperCase() ? word.charAt(0) + word.slice(1).toLowerCase() : word))
    .join(" ");
}

function supportLabel(n: TreeNode): string | null {
  const match = /^(REC|SS|REV|RES)\.(.*)$/i.exec(n.name);
  if (!match || n.file || n.linked) return null;
  const kind = (match[1] ?? "").toUpperCase();
  const rest = match[2] ?? "";
  const label = SUPPORT_LABELS[kind] ?? "";
  // A resources folder may already say what it holds ("RES.edition-resources"), so only the default one takes its parent's name.
  if (kind === "RES" && rest.toLowerCase() !== label.toLowerCase()) return readable(rest);
  const owner = kind === "RES" ? (n.relParts[n.relParts.length - 2] ?? "").replace(/^[^.]*\./, "") : rest;
  return `${readable(owner)} ${label}`.trim();
}

export const displayNodeName = (n: TreeNode): string => {
  if (n.relParts.length === 1 && GLOBAL_PREFIX.test(n.name)) return n.name.replace(GLOBAL_PREFIX, "");
  const support = supportLabel(n);
  if (support) return support;
  if (n.file && !extensionsShown()) return n.name.lastIndexOf(".") > 0 ? n.name.slice(0, n.name.lastIndexOf(".")) : n.name;
  return n.linked || state.settings.hideDirectoryIdentifiers ? nameWithoutIdentifier(n) : n.name;
};
const OWNER_KEYS = ["owner", "holder", "client"];

export const isWorknode = (n: TreeNode) => n.nodeType?.base === "WKB";
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
export function fileTypeName(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? `${name.slice(dot + 1).toUpperCase()} file` : "File";
}

// A level 1 node heads one area of the storage, so it reads as a main node; deeper nodes say their level.
export const typeName = (n: TreeNode): string => (n.file ? fileTypeName(n.name) : n.nodeType?.base === "NODE" ? (n.relParts.length === 1 ? "Main Node" : `Node (Level ${n.relParts.length})`) : TYPE_LABELS.find(([v]) => v === n.nodeType?.base)?.[1] ?? n.nodeType?.raw ?? "—");

export const typeLabel = (n: TreeNode): string => {
  const flags = nodeFlags(n);
  return flags.length > 0 ? `${typeName(n)}\n(${flags.join(", ")})` : typeName(n);
};

// A tooltip is one line in plain words, so the internal type names give way to what the folder is for.
const SUPPORT_SUMMARY: Record<string, (owner: string) => string> = {
  REV: (o) => `Review: files waiting to be sorted into ${o}`,
  RES: (o) => `Resources: reference files shared by ${o}`,
  REC: (o) => `Recordings of ${o}`,
  SS: (o) => `Screenshots of ${o}`,
  TPL: (o) => `Templates for ${o}`,
  CFG: (o) => `Settings files of ${o}`,
};

const baseSummary = (n: TreeNode): string => {
  const support = n.file ? undefined : SUPPORT_SUMMARY[n.nodeType?.base ?? ""];
  if (support) return support(n.relParts.length > 1 ? readable(n.relParts[n.relParts.length - 2].replace(/^[^.]*\./, "")) : "the Storage");
  if (!n.file && n.nodeType?.base === "NODE" && n.flags.includes("data") && nodeFlags(n).length === 1) return "Node with free files";
  if (!n.file && n.nodeType?.base === "NODE" && n.flags.includes("main") && nodeFlags(n).length === 1) return "Main Node";
  return typeLabel(n).replace(/\s*\n\s*/, " ");
};

export const nodeSummary = (n: TreeNode): string => (n.concealed ? `${baseSummary(n)} · Hidden, shown faded because hidden items are on` : baseSummary(n));

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

export function nodeDetails(n: TreeNode): [string, string][] {
  const rows: [string, string][] = [["Name", n.name]];
  const wkb = isWorknode(n);
  if (!n.linked && !isReviewRoot(n) && n.relParts.length > 1) rows.push(["Type", typeName(n)]);
  if (wkb) {
    const output = outputOf(n);
    rows.push(["Output", output === null ? UNKNOWN : `/${output.replace(/^\/+/, "")}`]);
  }
  if (wkb) rows.push(["Status", n.status ? statusLabel(n.status) : UNKNOWN]);
  if (wkb) rows.push(["Owner", ownerOf(n) ?? UNKNOWN]);
  if (n.info?.error) rows.push(["Info", n.info.error]);
  for (const f of n.info?.fields ?? []) {
    if (f.key.toLowerCase() === "breadcrumb") continue;
    // Status, owner and output of a worknode already have their own rows.
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
  if (n.stamps.modified !== null) {
    const created = rows.findIndex(([key]) => key.toLowerCase() === "created");
    const d = new Date(n.stamps.modified);
    const modified = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    rows.splice(created < 0 ? rows.length : created + 1, 0, ["Modified", modified]);
  }
  const created = rows.find(([key]) => key.toLowerCase() === "created");
  if (created) created[1] = formatDate(created[1]);
  return rows;
}

function formatDate(value: string): string {
  const match = /^(\d{1,4})[-/.](\d{1,2})[-/.](\d{1,4})$/.exec(value.trim());
  if (!match) return value;
  const dayFirst = match[1].length < 3 && match[3].length === 4;
  const yearPart = dayFirst ? match[3] : match[1];
  const year = yearPart.length === 2 ? 2000 + Number(yearPart) : Number(yearPart);
  const month = Number(match[2]);
  const day = Number(dayFirst ? match[1] : match[3]);
  if (month < 1 || month > 12 || day < 1 || day > 31) return value;
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}
