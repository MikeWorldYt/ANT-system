// Ctrl+A marks every item of the folder the selection stands in, as Explorer does, instead of selecting the page's text.
import { activeColumnKey, findNode, keyOf, listed, selectKeys, state, virtualRoot } from "../services/store/state";
import { virtualNodes } from "../services/store/virtual";
import { systemRootOf, type TreeNode } from "../services/ipc/types";

function siblings(): TreeNode[] {
  const parent = activeColumnKey();
  if (systemRootOf([parent]) === parent) return virtualRoot(state.virtual) === parent ? virtualNodes() : (state.system[parent] ?? []);
  if (parent === "") return state.virtual !== null ? virtualNodes() : (state.tree?.nodes ?? []);
  return findNode(parent.split("/"))?.children ?? [];
}

export function selectAll(): void {
  const keys = listed(siblings()).map((n) => keyOf(n.relParts));
  if (keys.length > 0) selectKeys(keys);
}
