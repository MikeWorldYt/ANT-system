// Ctrl+A marks every item of the folder the selection stands in, as Explorer does, instead of selecting the page's text.
import { activeColumnKey, findNode, keyOf, listed, selectKeys, state } from "../services/store/state";
import { virtualNodes } from "../services/store/virtual";
import { DOWNLOADS_ROOT, type TreeNode } from "../services/ipc/types";

function siblings(): TreeNode[] {
  const parent = activeColumnKey();
  if (parent === DOWNLOADS_ROOT) return state.virtual === "downloads" ? virtualNodes() : state.downloads;
  if (parent === "") return state.virtual !== null ? virtualNodes() : (state.tree?.nodes ?? []);
  return findNode(parent.split("/"))?.children ?? [];
}

export function selectAll(): void {
  const keys = listed(siblings()).map((n) => keyOf(n.relParts));
  if (keys.length > 0) selectKeys(keys);
}
