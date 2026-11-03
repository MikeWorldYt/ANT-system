// Commands on the marked items, shared by the side panel and the shortcuts so both read a selection the same way.
import { actedNodes, state } from "../store/state";
import { hideNode, move, remove, removeMany, setFavorite } from "./ops";
import type { MoveDirection, NodeActions } from "../ipc/types";

// Only delete has a batch form so far; every other command needs exactly one item.
export function canActOn(action: keyof NodeActions): boolean {
  const nodes = actedNodes();
  if (action === "delete" && nodes.length > 1) return true;
  return nodes.length === 1 && !!nodes[0]?.actions[action];
}

export function favoriteActed(): void {
  const node = actedNodes()[0];
  if (node && canActOn("favorite")) void setFavorite(node, !node.favorite);
}

export function moveActed(dir: MoveDirection): void {
  const node = actedNodes()[0];
  if (node && canActOn(dir === "up" ? "moveUp" : "moveDown") && !state.busy) void move(node, dir);
}

export function hideActed(): void {
  const node = actedNodes()[0];
  if (node && canActOn("hide")) void hideNode(node);
}

export function deleteActed(): void {
  const nodes = actedNodes();
  if (!canActOn("delete")) return;
  if (nodes.length > 1) void removeMany(nodes);
  else if (nodes[0]) void remove(nodes[0]);
}