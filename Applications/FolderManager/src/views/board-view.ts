// Task board of every workboard, shown for the virtual BOARD entry in place of whichever view is active.
import { el } from "../ui/dom";
import { iconButton } from "../ui/icon-button";
import { attachMenu, type MenuChoice } from "../ui/anchored-menu";
import { statusMark } from "../ui/status-mark";
import { displayNodeName, statusLabel } from "../store/node-info";
import { BOARD_COLUMNS, boardLists, type BoardLists } from "../store/board";
import { virtualNodes } from "../store/virtual";
import { isSelected, keyOf, select, state } from "../store/state";
import { goToReal } from "../actions/virtual-nav";
import { moveCard } from "../actions/board";
import { installBoardDrag } from "./board-drag";
import type { BoardColumn, TreeNode } from "../ipc/types";

export function createBoardView(host: HTMLElement): { element: HTMLElement; render(): void } {
  const columns = el("div", { class: "board-columns" });
  const back = iconButton("Back to the Storage", "back", () => select(null));
  const element = el("section", { class: "board", "aria-labelledby": "board-title" }, el("header", { class: "board-head" }, back, el("h2", { id: "board-title", class: "board-title" }, "BOARD")), columns);
  element.hidden = true;
  let lists: BoardLists = boardLists([]);
  let shown = false;

  const cardNode = (key: string): TreeNode | undefined => Object.values(lists).flat().find((n) => keyOf(n.relParts) === key);
  installBoardDrag(columns, (key, to, index) => {
    const card = cardNode(key);
    if (card) void moveCard(card, to, index);
  });

  // The keyboard way to do what a drag does: pick a column, or step the card up or down in its own.
  function moveChoices(card: TreeNode, at: BoardColumn, index: number): MenuChoice[] {
    const choices: MenuChoice[] = BOARD_COLUMNS.map(([id, label]) => ({ label: `Move to ${label}`, checked: id === at, run: () => void moveCard(card, id, id === at ? index : lists[id].length) }));
    const steps: MenuChoice[] = [
      ...(index > 0 ? [{ label: "Move up", run: () => void moveCard(card, at, index - 1) }] : []),
      ...(index < lists[at].length - 1 ? [{ label: "Move down", run: () => void moveCard(card, at, index + 1) }] : []),
    ];
    return [...choices, ...steps.map((s, i) => ({ ...s, divider: i === 0 }))];
  }

  function buildCard(n: TreeNode, at: BoardColumn, index: number): HTMLElement {
    const key = keyOf(n.relParts);
    const parent = n.relParts.slice(0, -1).join(" › ");
    const name = displayNodeName(n);
    const move = iconButton(`Move ${name}`, "chevronDown");
    move.classList.add("board-move");
    attachMenu(move, `Move ${name}`, () => moveChoices(n, at, index));
    const label = [name, n.status ? statusLabel(n.status) : null, `in ${parent}`].filter(Boolean).join(", ");
    const head = el("div", { class: "board-card-head" }, ...(n.status ? [statusMark(n.status)] : []), el("span", { class: "board-name" }, name), move);
    const card = el("li", { class: "board-card", tabindex: "0", "aria-label": label, title: n.relParts.join("/") }, head, el("span", { class: "board-path" }, parent));
    card.dataset.card = key;
    if (isSelected(key)) {
      card.classList.add("selected");
      card.setAttribute("aria-current", "true");
    }
    const fromMove = (ev: Event) => ev.target instanceof Element && ev.target.closest(".board-move") !== null;
    card.addEventListener("click", (ev) => {
      if (!fromMove(ev)) select(n.relParts);
    });
    card.addEventListener("dblclick", (ev) => {
      if (!fromMove(ev)) void goToReal(n.relParts);
    });
    card.addEventListener("keydown", (ev) => {
      if (fromMove(ev) || ev.ctrlKey || ev.altKey || ev.metaKey) return;
      if (ev.key === "Enter") void goToReal(n.relParts);
      else if (ev.key === " ") select(n.relParts);
      else if (!focusNeighbor(card, ev.key)) return;
      ev.preventDefault();
    });
    // The view's menu acts through views the board hides, so a card offers none.
    card.addEventListener("contextmenu", (ev) => ev.preventDefault());
    return card;
  }

  // Up and Down walk a column; Left and Right jump to the first card of the next column that has one.
  function focusNeighbor(card: HTMLElement, key: string): boolean {
    const inColumn = [...(card.parentElement?.querySelectorAll<HTMLElement>(".board-card") ?? [])];
    const i = inColumn.indexOf(card);
    const cols = [...columns.querySelectorAll<HTMLElement>(".board-col")];
    const at = cols.findIndex((c) => c.contains(card));
    const firstIn = (list: HTMLElement[]) => list.map((c) => c.querySelector<HTMLElement>(".board-card")).find((c) => c !== null);
    // null means the key is not a move; undefined, a move with nowhere to go.
    const target =
      key === "ArrowDown" ? inColumn[i + 1]
      : key === "ArrowUp" ? inColumn[i - 1]
      : key === "ArrowRight" ? firstIn(cols.slice(at + 1))
      : key === "ArrowLeft" ? firstIn(cols.slice(0, at).reverse())
      : null;
    if (target === null) return false;
    target?.focus();
    return true;
  }

  function buildColumn(id: BoardColumn, label: string): HTMLElement {
    const headingId = `board-col-${id}`;
    const cards = lists[id];
    const list = el("ul", { class: "board-list", "aria-labelledby": headingId }, ...cards.map((n, i) => buildCard(n, id, i)));
    if (cards.length === 0) list.append(el("li", { class: "board-empty" }, "No cards"));
    const col = el("section", { class: "board-col", "aria-labelledby": headingId }, el("h3", { id: headingId, class: "board-col-title" }, label, " ", el("span", { class: "board-count" }, String(cards.length))), list);
    col.dataset.column = id;
    return col;
  }

  function render(): void {
    const open = state.virtual === "board";
    element.hidden = !open;
    host.hidden = open;
    const opening = open && !shown;
    shown = open;
    if (!open) return;
    const focused = document.activeElement instanceof HTMLElement ? document.activeElement.closest<HTMLElement>(".board-card")?.dataset.card : undefined;
    lists = boardLists(virtualNodes());
    columns.replaceChildren(...BOARD_COLUMNS.map(([id, label]) => buildColumn(id, label)));
    const again = focused ? columns.querySelector<HTMLElement>(`[data-card="${CSS.escape(focused)}"]`) : null;
    // The entry that opened the board is hidden with the views, so the keyboard lands on the board.
    if (again) again.focus();
    else if (opening) back.focus();
  }

  return { element, render };
}
