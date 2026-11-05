// Which columns the columns view shows for the selected path, real or symbolic through FAVORITES.
import { sortedForColumns } from "../services/store/sort";
import { isReviewRoot } from "../services/store/node-info";
import { findNode, keyOf, listed, sortOf, state } from "../services/store/state";
import { virtualNodes } from "../services/store/virtual";
import type { TreeNode } from "../services/ipc/types";

export interface Column {
  nodes: TreeNode[];
  loading: boolean;
  // The folder listed; null for the storage root and for a virtual list.
  parent: TreeNode | null;
  listing: boolean;
  // The row of this column on the selected path.
  pathKey: string | null;
}

const sorted = (nodes: TreeNode[], parent: TreeNode | null) => sortedForColumns(listed(nodes), sortOf(parent ? keyOf(parent.relParts) : ""), state.settings.pinFavorites);

// The keys along the selected path, one per column; inside a virtual entry they start at the entry it went through.
function chainOf(): string[] {
  const parts = state.selectedKey?.split("/") ?? [];
  const from = state.virtual === null ? 1 : state.via === null ? parts.length + 1 : state.via.split("/").length;
  return parts.slice(from - 1).map((_, i) => keyOf(parts.slice(0, from + i)));
}

// `opened` is the selected folder whose child column the user opened; selecting alone shows none.
export function buildColumns(opened: string | null): Column[] {
  const chain = chainOf();
  const rootNodes = state.tree?.nodes ?? [];
  const cols: Column[] = [{ nodes: sorted(rootNodes, null), loading: false, parent: null, listing: false, pathKey: state.virtual === null ? (chain[0] ?? null) : null }];
  // An open virtual entry lists its folders beside the root, wherever they live, and its path continues from there.
  if (state.virtual !== null) cols.push({ nodes: virtualNodes(), loading: false, parent: null, listing: true, pathKey: chain[0] ?? null });
  for (const [i, key] of chain.entries()) {
    const open = findNode(key.split("/"));
    if (!open || !open.hasChildren) break;
    // The review folder's list stays open while it is selected, so its filter legend has a column to sit in.
    if (key === state.selectedKey && key !== opened && !isReviewRoot(open)) break;
    if (open.children === null) {
      cols.push({ nodes: [], loading: true, parent: open, listing: false, pathKey: null });
      break;
    }
    cols.push({ nodes: sorted(open.children, open), loading: false, parent: open, listing: false, pathKey: chain[i + 1] ?? null });
    if (open.children.length === 0) break;
  }
  return cols;
}
