// Moving a card on the task board: its new column sets the status first, then the column's order is saved.
import { placeOnBoard, setDetails } from "./ops";
import { boardLists, COLUMN_STATUS, placement } from "../store/board";
import { virtualNodes } from "../store/virtual";
import type { BoardColumn, TreeNode } from "../ipc/types";

// The status goes through the details flow, so finishing on the board asks about the output like the status picker does.
export async function moveCard(card: TreeNode, to: BoardColumn, index: number): Promise<void> {
  const cards = placement(boardLists(virtualNodes()), card, to, index);
  const status = COLUMN_STATUS[to];
  if (status && !(await setDetails(card, status, null))) return;
  await placeOnBoard(cards);
}
