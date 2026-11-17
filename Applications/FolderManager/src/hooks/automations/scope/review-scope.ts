// What a review-folder automation acts on, read from the selection: a node or review folder, a folder inside it, one item, or several.
import * as api from "../../../services/ipc/api";
import { findNode, selectedNode, selectedNodes, state } from "../../../services/store/state";
import { DOWNLOADS_ROOT, inDownloads, type TreeNode } from "../../../services/ipc/types";

export interface ReviewScope {
  // A folder gives the files it holds, not what its subfolders hold.
  folders: string[][];
  files: TreeNode[];
  label: string;
  // The one review folder the selection sits in, whose automations can be switched on.
  review: TreeNode | null;
}

// Review, screenshot and recording folders run automations by default; any other folder once it carries "autom".
const AUTOMATED = ["REV", "SS", "REC"];
const isReview = (n: TreeNode | null | undefined): n is TreeNode => !!n && !n.file && !n.ghost && (AUTOMATED.includes(n.nodeType?.base ?? "") || n.flags.includes("autom"));

// The Downloads folder is no tree node, so it stands in as one: its batch and dictionary run without the storage, and it has no automatic run.
export const DOWNLOADS = { name: "Downloads", relParts: [DOWNLOADS_ROOT], flags: [], file: false, ghost: false, nodeType: null } as unknown as TreeNode;
export const isDownloads = (n: TreeNode | null | undefined): boolean => n === DOWNLOADS;

// The review folder the item sits in, at any depth, or the item itself when it is one.
function reviewAbove(n: TreeNode): TreeNode | null {
  if (inDownloads(n.relParts)) return DOWNLOADS;
  for (let i = n.relParts.length; i > 0; i--) {
    const up = findNode(n.relParts.slice(0, i));
    if (isReview(up)) return up;
  }
  return null;
}

export function reviewScope(): ReviewScope | null {
  const picked = selectedNodes();
  if (picked.length > 1) {
    const reviews = picked.map(reviewAbove);
    if (!reviews.every((r) => r)) return null;
    const same = (r: TreeNode | null) => r?.relParts.join("/") === reviews[0]?.relParts.join("/");
    const one = reviews.every(same) ? (reviews[0] ?? null) : null;
    return { folders: picked.filter((n) => !n.file).map((n) => n.relParts), files: picked.filter((n) => n.file), label: `${picked.length} selected items`, review: one };
  }
  const n = selectedNode();
  if (!n && state.virtual === "downloads") return { folders: [[DOWNLOADS_ROOT]], files: [], label: "Files in Downloads", review: DOWNLOADS };
  const review = n ? reviewAbove(n) : null;
  if (!n || !review) return null;
  if (!n.file) return { folders: [n.relParts], files: [], label: `Files in ${n.name}`, review };
  return { folders: [n.relParts.slice(0, -1)], files: [], label: `Files beside ${n.name}`, review };
}

// Every file the scope reaches, listed fresh from disk so nothing the tree has not loaded yet is missed.
export async function scopeFiles(scope: ReviewScope): Promise<TreeNode[]> {
  const files = [...scope.files];
  for (const folder of scope.folders) files.push(...(await api.listChildren(folder)).filter((c) => c.file));
  return files;
}
