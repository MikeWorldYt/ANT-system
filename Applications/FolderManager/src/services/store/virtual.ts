// Virtual root entries list real folders from the whole Storage; their nodes are loaded into the tree without expanding it.
import { listChildren, listFavorites, listWorkboards } from "../ipc/api";
import { findNode, keyOf, state, type VirtualId } from "./state";
import { DOWNLOADS_ROOT, type TreeNode } from "../ipc/types";

// Rereading keeps what was already loaded below a folder that is still there, so an open path is not cut.
async function listDownloadsTop(): Promise<string[][]> {
  const before = state.downloads;
  const fresh = await listChildren([DOWNLOADS_ROOT]);
  state.downloads = fresh.map((n) => ({ ...n, children: before.find((o) => o.name === n.name)?.children ?? null }));
  return state.downloads.map((n) => n.relParts);
}

const SOURCES: Record<VirtualId, () => Promise<string[][]>> = { favorites: listFavorites, downloads: listDownloadsTop, board: listWorkboards };

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

export async function loadVirtual(): Promise<void> {
  const id = state.virtual;
  if (id === null) return;
  const paths = await SOURCES[id]();
  for (const parts of paths) await loadPath(parts);
  // Another entry may have been opened, or this one closed, while the list loaded.
  if (state.virtual === id) state.virtualKeys = paths.filter((p) => findNode(p) !== null).map(keyOf);
}

export const virtualNodes = (): TreeNode[] => state.virtualKeys.flatMap((key) => findNode(key.split("/")) ?? []);
