// REVIEW lists real review folders from the whole Storage, so their staircase, groups and filter are worked out here.
import { readPref, savePref } from "./prefs";
import { keyOf, state } from "./state";
import type { TreeNode } from "../ipc/types";

interface ReviewFilter {
  hidden: string[];
  favorites: boolean;
}

const PREF = "fm.reviewFilter";
const prefKey = () => `${PREF}:${state.tree?.root ?? ""}`;

export function reviewFilter(): ReviewFilter {
  const saved = readPref(prefKey());
  if (saved === null) return { hidden: [], favorites: true };
  try {
    return JSON.parse(saved) as ReviewFilter;
  } catch (e) {
    console.warn("Ignoring a broken review filter", e);
    return { hidden: [], favorites: true };
  }
}

export const saveReviewFilter = (filter: ReviewFilter) => savePref(prefKey(), JSON.stringify(filter));

export const inReviewList = (n: TreeNode) => state.virtual === "review" && state.virtualKeys.includes(keyOf(n.relParts));
export const reviewLevel = (n: TreeNode) => n.relParts.length - 1;
export const reviewName = (n: TreeNode) => n.name.replace(/^[^.]*\./, "");
const pinned = (n: TreeNode) => n.favorite && state.settings.pinFavorites;
// Pinned favorites form their own group above the categories.
export const reviewGroup = (n: TreeNode) => (pinned(n) ? "" : (n.relParts[0] ?? ""));

// Comparing the owners' paths puts every folder right below its parent, whatever the sort of the columns.
function byOwner(a: TreeNode, b: TreeNode): number {
  const [x, y] = [a.relParts.slice(0, -1), b.relParts.slice(0, -1)];
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const order = (x[i] ?? "").localeCompare(y[i] ?? "", undefined, { numeric: true });
    if (order !== 0) return order;
  }
  return x.length - y.length;
}

export function arrangeReview(nodes: TreeNode[]): TreeNode[] {
  const { hidden, favorites } = reviewFilter();
  const shown = nodes.filter((n) => !hidden.includes(n.relParts[0] ?? "") || (favorites && n.favorite)).sort(byOwner);
  return [...shown.filter(pinned), ...shown.filter((n) => !pinned(n))];
}
