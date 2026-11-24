// Virtual root entries list real folders from the whole Storage; their nodes are loaded into the tree without expanding it.
import { listChildren, listFavorites, listReview, listWorknodes } from "../ipc/api";
import { arrangeReview } from "./review-list";
import { sortedForColumns } from "./sort";
import { findNode, isVirtualShown, keyOf, sortOf, state, virtualRoot, type VirtualId } from "./state";
import type { TreeNode } from "../ipc/types";

// Rereading keeps what was already loaded below a folder that is still there, so an open path is not cut.
export const keepChildren = (root: string, fresh: TreeNode[]): TreeNode[] => fresh.map((n) => ({ ...n, children: state.system[root]?.find((o) => o.name === n.name)?.children ?? null }));

async function listSystemTop(root: string): Promise<string[][]> {
  const nodes = keepChildren(root, await listChildren([root]));
  state.system[root] = nodes;
  return nodes.map((n) => n.relParts);
}

const SOURCES: Partial<Record<VirtualId, () => Promise<string[][]>>> = { favorites: listFavorites, review: listReview, board: listWorknodes };

// Loads every level above the folder, so findNode, the details panel and the actions reach it like any shown folder.
export async function loadPath(parts: string[]): Promise<void> {
  let level = state.tree?.nodes ?? [];
  for (const part of parts.slice(0, -1)) {
    const n = level.find((x) => x.name === part);
    if (!n) return;
    if (n.children === null && n.hasChildren) n.children = await listChildren(n.relParts);
    level = n.children ?? [];
  }
}

export async function virtualKeys(id: VirtualId): Promise<string[]> {
  const root = virtualRoot(id);
  const paths = await (root !== null ? listSystemTop(root) : (SOURCES[id]?.() ?? Promise.resolve([])));
  for (const parts of paths) await loadPath(parts);
  return paths.filter((p) => findNode(p) !== null).map(keyOf);
}

export async function loadVirtual(): Promise<void> {
  const id = state.virtual;
  if (id === null) return;
  const keys = await virtualKeys(id);
  // Another entry may have been opened, or this one closed, while the list loaded.
  if (state.virtual === id) state.virtualKeys = keys;
}

// An entry's folders in the order its list shows them.
function arrangeList(id: VirtualId, keys: string[]): TreeNode[] {
  const nodes = keys.flatMap((key) => findNode(key.split("/")) ?? []);
  if (id === "review") return arrangeReview(nodes);
  const root = virtualRoot(id);
  return root !== null ? sortedForColumns(nodes, sortOf(root), state.settings.pinFavorites) : nodes;
}

export const virtualNodes = (): TreeNode[] => (state.virtual === null ? [] : arrangeList(state.virtual, state.virtualKeys));

// Entries unfolded in the navigation pane with the folders each lists, null while loading; apart from the entry open in the views.
const unfolded = new Map<VirtualId, string[] | null>();

export const isUnfolded = (id: VirtualId): boolean => unfolded.has(id);

export const unfoldLoading = (id: VirtualId): boolean => unfolded.get(id) === null;

export const unfoldedNodes = (id: VirtualId): TreeNode[] => arrangeList(id, unfolded.get(id) ?? []);

export const unfoldedRoots = (): string[] => [...unfolded.keys()].flatMap((id) => virtualRoot(id) ?? []);

export function setUnfolded(id: VirtualId, on: boolean): boolean {
  if (on === unfolded.has(id)) return false;
  if (on) unfolded.set(id, null);
  else unfolded.delete(id);
  return true;
}

export async function loadUnfolded(id: VirtualId): Promise<void> {
  const keys = await virtualKeys(id);
  // The entry may have been folded while its list loaded.
  if (unfolded.has(id)) unfolded.set(id, keys);
}

export async function loadUnfoldedLists(): Promise<void> {
  for (const id of [...unfolded.keys()]) if (isVirtualShown(id)) await loadUnfolded(id);
}
