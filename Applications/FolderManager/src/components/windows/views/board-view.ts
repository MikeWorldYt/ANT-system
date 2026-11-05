// Task board of every workboard, shown for the virtual BOARD entry in place of whichever view is active.
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { SORT_KEYS } from "../../organisms/bars/toolbar";
import { sortedCards, type ColumnSort } from "../../../services/store/sort";
import { statusMark } from "../../atoms/status-mark";
import { importantMark } from "../../molecules/name-cell";
import { displayNodeName, statusLabel } from "../../../services/store/node-info";
import { BOARD_COLUMNS, boardLists, type BoardLists } from "../../../services/store/board";
import { virtualNodes } from "../../../services/store/virtual";
import { isSelected, keyOf, select, state } from "../../../services/store/state";
import { goToReal } from "../../../hooks/virtual-nav";
import { moveCard } from "../../../hooks/board";
import { installBoardDrag } from "../../../hooks/board-drag";
import { cardDetails } from "../../molecules/board-card";
import { openFolder } from "../../../hooks/path-actions";
import { boardCardMenu } from "../../organisms/menus/board-card-menu";
import { isMenuKey, openMenuWith } from "../../organisms/menus/context-menu";
import type { BoardColumn, TreeNode } from "../../../services/ipc/types";

export function createBoardView(host: HTMLElement, editDetails: () => void): { element: HTMLElement; render(): void } {
  const columns = el("div", { class: "board-columns" });
  const element = el("section", { class: "board", "aria-label": "Board" }, columns);
  element.hidden = true;
  let lists: BoardLists = boardLists([]);
  const collapsed = new Set<BoardColumn>();
  // A column without a sort keeps the order the cards were dragged into.
  const sorts = new Map<BoardColumn, ColumnSort>();
  let shown = false;

  const cardNode = (key: string): TreeNode | undefined => Object.values(lists).flat().find((n) => keyOf(n.relParts) === key);
  installBoardDrag(columns, (key, to, index) => {
    const card = cardNode(key);
    if (card) void moveCard(card, to, index);
  });

  // Cards are rebuilt on every change, so which ones are expanded lives here.
  const expanded = new Set<string>();

  function openCardMenu(n: TreeNode, at: BoardColumn, x: number, y: number): void {
    const key = keyOf(n.relParts);
    const act = {
      edit: () => {
        select(n.relParts);
        editDetails();
      },
      moveTo: (to: BoardColumn) => void moveCard(n, to, lists[to].length),
    };
    openMenuWith((close) => boardCardMenu(n, at, act, close), `Actions for ${displayNodeName(n)}`, x, y, () => columns.querySelector<HTMLElement>(`[data-card="${CSS.escape(key)}"]`)?.focus());
  }

  function buildCard(n: TreeNode, at: BoardColumn, index: number): HTMLElement {
    const key = keyOf(n.relParts);
    const parent = n.relParts.slice(0, -1).join(" › ");
    const name = displayNodeName(n);
    const { toggle, panel } = cardDetails(n, name, `board-details-${at}-${index}`, expanded.has(key), (open) => (open ? expanded.add(key) : expanded.delete(key)), (node) => void openFolder(node));
    const label = [name, n.important ? "important" : null, n.status ? statusLabel(n.status) : null, `in ${parent}`].filter(Boolean).join(", ");
    const head = el("div", { class: "board-card-head" }, ...(n.status ? [statusMark(n.status)] : []), el("span", { class: "board-name" }, name), ...(n.important ? [importantMark()] : []), toggle);
    const card = el("li", { class: "board-card", tabindex: "0", "aria-label": label, title: n.relParts.join("/") }, head, panel);
    card.dataset.card = key;
    card.classList.toggle("important", n.important);
    if (isSelected(key)) {
      card.classList.add("selected");
      card.setAttribute("aria-current", "true");
    }
    const fromButton = (ev: Event) => ev.target instanceof Element && ev.target.closest("button") !== null;
    card.addEventListener("click", (ev) => {
      if (!fromButton(ev)) select(n.relParts);
    });
    card.addEventListener("dblclick", (ev) => {
      if (!fromButton(ev)) void goToReal(n.relParts);
    });
    card.addEventListener("keydown", (ev) => {
      if (fromButton(ev)) return;
      if (isMenuKey(ev)) {
        const r = card.getBoundingClientRect();
        openCardMenu(n, at, r.left + 24, r.bottom);
      } else if (ev.ctrlKey || ev.altKey || ev.metaKey) return;
      else if (ev.key === "Enter") void goToReal(n.relParts);
      else if (ev.key === " ") select(n.relParts);
      else if (!focusNeighbor(card, ev.key)) return;
      ev.preventDefault();
    });
    card.addEventListener("contextmenu", (ev) => {
      ev.preventDefault();
      // Keyboard-originated contextmenu events carry button 0 and are handled on keydown.
      if (ev.button === 2) openCardMenu(n, at, ev.clientX, ev.clientY);
    });
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

  // Each click on a key steps ascending, descending, then back to the board order.
  function sortButtons(id: BoardColumn, label: string): HTMLElement[] {
    const sort = sorts.get(id);
    return SORT_KEYS.map(([key, text, icon]) => {
      const on = sort?.by === key;
      const mode = on ? (sort?.dir === "desc" ? "descending" : "ascending") : "off";
      const button = el("button", { type: "button", class: `board-col-toggle board-col-sort${on ? " active" : ""}`, "aria-pressed": String(on), "aria-label": `Sort ${label} by ${text}, ${mode}`, title: on ? `${text}, ${mode}` : `Sort by ${text}` }, guiIcon(icon));
      button.dataset.sort = key;
      button.addEventListener("click", () => {
        if (!on) sorts.set(id, { by: key, dir: "asc" });
        else if (sort?.dir === "asc") sorts.set(id, { by: key, dir: "desc" });
        else sorts.delete(id);
        render();
        columns.querySelector<HTMLElement>(`[data-column="${id}"] [data-sort="${key}"]`)?.focus();
      });
      return button;
    });
  }

  function buildColumn(id: BoardColumn, label: string): HTMLElement {
    const headingId = `board-col-${id}`;
    const sort = sorts.get(id);
    const cards = sort ? sortedCards(lists[id], sort) : lists[id];
    const list = el("ul", { class: "board-list", "aria-labelledby": headingId }, ...cards.map((n, i) => buildCard(n, id, i)));
    if (cards.length === 0) list.append(el("li", { class: "board-empty" }, "No cards"));
    const shut = collapsed.has(id);
    list.hidden = shut;
    const toggle = el("button", { type: "button", class: "board-col-toggle", "aria-expanded": String(!shut), "aria-controls": `${headingId}-list`, "aria-label": `${shut ? "Expand" : "Collapse"} ${label}`, title: shut ? "Expand" : "Collapse" }, guiIcon("chevronDown"));
    list.id = `${headingId}-list`;
    toggle.addEventListener("click", () => {
      if (collapsed.has(id)) collapsed.delete(id);
      else collapsed.add(id);
      render();
      columns.querySelector<HTMLElement>(`[data-column="${id}"] .board-col-toggle`)?.focus();
    });
    const head = el("h3", { id: headingId, class: "board-col-title" }, label, el("span", { class: "board-col-tools" }, ...sortButtons(id, label), toggle, el("span", { class: "board-count" }, String(cards.length))));
    const col = el("section", { class: `board-col${shut ? " collapsed" : ""}`, "aria-labelledby": headingId }, head, list);
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
    // The board replaces the views, so the keyboard lands on it.
    if (again) again.focus();
    else if (opening) columns.querySelector<HTMLElement>(".board-card")?.focus();
  }

  return { element, render };
}
