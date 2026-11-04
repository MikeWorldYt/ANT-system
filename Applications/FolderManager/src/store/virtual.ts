// Virtual root entries list real folders from the whole Storage; their nodes are loaded into the tree without expanding it.
import { listChildren, listFavorites, listWorkboards } from "../ipc/api";
import { findNode, keyOf, state, type VirtualId } from "./state";
import type { TreeNode } from "../ipc/types";

const SOURCES: Record<VirtualId, () => Promise<string[][]>> = { favorites: listFavorites, board: listWorkboards };

// Loads every level above the folder, so findNode, the details panel and the actions reach it like any shown folder.
async function loadPath(parts: string[]): Promise<void> {
  let level = state.tree?.nodes ?? [];
  for (const part of parts.slice(0, -1)) {
    const n = level.find((x) => x.name === part);
    if (!n) return;
    if (n.children === null && n.hasChildren) n.children = await listChildren(n.relParts);
    level = n.children ?? [];
  }
}

export async function loadVirtual(): Promise<void> {
  const id = state.virtual;
  if (id === null) return;
  const paths = await SOURCES[id]();
  for (const parts of paths) await loadPath(parts);
  // Another entry may have been opened, or this one closed, while the list loaded.
  if (state.virtual === id) state.virtualKeys = paths.filter((p) => findNode(p) !== null).map(keyOf);
}

export const virtualNodes = (): TreeNode[] => state.virtualKeys.flatMap((key) => findNode(key.split("/")) ?? []);
