// Right column: quick actions, details and order of the selected node, collapsible to an icon rail.
import { el, setDisabled } from "../../dom";
import { iconButton } from "../../atoms/icon-button";
import { checkbox } from "../../atoms/checkbox";
import { allImportant, canActOn, canMarkImportant, deleteActed, favoriteActed, hideActed, importantActed, moveActed } from "../../../hooks/acted-nodes";
import { setReviewFilter } from "../../../hooks/ops";
import { readPref, savePref } from "../../../services/store/prefs";
import { favoritesShown, hiddenCategories, importantLabel, isMainCategory, isReviewRoot, nameWithoutIdentifier } from "../../../services/store/node-info";
import { quickActionsShown, railLayout, type RailId } from "../../../services/store/side-layout";
import { selectedNode, state, toggleHidingLinks } from "../../../services/store/state";
import { canEdit, createDetailsPanel } from "./details-panel";
import { createAutomationsPanel } from "./automations-panel";
import { createReviewAutomations } from "./review-automations";
import { toggleBoard } from "../../../hooks/virtual-nav";
import { sideAction } from "../../molecules/side-action";
import type { TreeNode } from "../../../services/ipc/types";

const PANEL_PREF = "fm.details";
// Function groups of the collapsed rail: view settings, item state, ordering, removal.
const RAIL_GROUP: Record<RailId, number> = { settings: 0, board: 0, favorite: 1, edit: 1, moveUp: 2, moveDown: 2, delete: 3 };

export interface SidePanelActions {
  openSelected(): void;
  openSettings(pageId?: string): void;
}

export interface SidePanel {
  elements: HTMLElement[];
  render(): void;
  toggleCollapsed(): void;
  editDetails(): void;
}

const favoriteLabel = (n: TreeNode | null) => (n?.favorite ? "Remove from favorites" : "Add to favorites");

export function createSidePanel({ openSelected, openSettings }: SidePanelActions): SidePanel {
  let collapsed = readPref(PANEL_PREF) === "hidden";
  // Set when the rail pencil expanded the panel, so the end of that edit returns to the rail.
  let collapseAfterEdit = false;

  const moveUp = () => moveActed("up");
  const moveDown = () => moveActed("down");

  const automations = createAutomationsPanel();
  const reviewTools = createReviewAutomations();
  const details = createDetailsPanel(() => {
    if (!collapseAfterEdit) return;
    collapseAfterEdit = false;
    const focusLost = document.activeElement === null || document.activeElement === document.body;
    show(true);
    if (focusLost) railEdit.focus();
  });

  // ─────────♦ EXPANDED PANEL ───────────────────────────────────────────────────
  const title = el("h2", { class: "side-title" });
  const settings = iconButton("Settings", "settings", () => openSettings());
  const board = iconButton("Board", "layout", () => void toggleBoard());
  const collapse = iconButton("Collapse side panel", "chevronsRight", () => {
    setCollapsed(true);
    railExpand.focus();
  });
  collapse.classList.add("side-collapse");
  const head = el("div", { class: "side-head" }, title, board, settings, collapse);

  const open = sideAction("Open", null, openSelected, "");
  const favorite = sideAction(favoriteLabel(null), "starred", favoriteActed);
  const important = sideAction(importantLabel(false), "important", importantActed);
  const quick = section("side-quick", "Quick actions", open.button, favorite.button, important.button);
  const up = sideAction("Move up", "moveUp", moveUp, "accent");
  const down = sideAction("Move down", "moveDown", moveDown, "accent");
  const order = section("side-order", "Order", up.button, down.button);
  const hideButton = sideAction("Hide element", "hide", hideActed);
  const remover = sideAction("Delete", "trash", deleteActed, "danger");
  const foot = el("div", { class: "side-foot" }, hideButton.button, remover.button);
  const categories = el("div", { class: "side-categories" });
  const hideLinks = el("a", { href: "#", class: "side-link" }, "Select elements to hide");
  hideLinks.addEventListener("click", (ev) => {
    ev.preventDefault();
    toggleHidingLinks();
  });
  const categoriesSection = section("side-categories", "Filters", categories, hideLinks);
  const panel = el("aside", { class: "details-panel", "aria-label": "Side panel" }, head, quick, categoriesSection, details.element, automations.element, reviewTools.element, order, foot);

  // ─────────♦ COLLAPSED RAIL ───────────────────────────────────────────────────
  const railExpand = iconButton("Expand side panel", "chevronsLeft", () => {
    setCollapsed(false);
    collapse.focus();
  });
  railExpand.classList.add("side-rail-expand");
  const railFavorite = iconButton(favoriteLabel(null), "starred", favoriteActed);
  const railEdit = iconButton("Edit details", "pencil", editDetails);
  const railUp = iconButton("Move up", "moveUp", moveUp);
  const railDown = iconButton("Move down", "moveDown", moveDown);
  const railBoard = iconButton("Board", "layout", () => void toggleBoard());
  const railDelete = iconButton("Delete", "trash", deleteActed);
  railDelete.classList.add("danger");
  const about = iconButton("About", "pageAbout", () => openSettings("about"));
  about.classList.add("side-rail-end");
  const railButtons: Record<RailId, HTMLButtonElement> = {
    settings: iconButton("Settings", "settings", () => openSettings()),
    board: railBoard,
    favorite: railFavorite,
    edit: railEdit,
    moveUp: railUp,
    moveDown: railDown,
    delete: railDelete,
  };
  const rail = el("aside", { class: "side-rail", "aria-label": "Side panel" });
  let railKey: string | null = null;

  // Rebuilt only when the layout changes, since replacing the buttons drops keyboard focus.
  function layoutRail(): void {
    const ids = railLayout()
      .filter((i) => i.shown)
      .map((i) => i.id);
    const key = ids.join(",");
    if (key === railKey) return;
    railKey = key;
    const children: HTMLElement[] = [railExpand];
    let previous: number | null = null;
    for (const id of ids) {
      // A separator marks each change of function, so a moved button may start its own group.
      if (previous !== null && RAIL_GROUP[id] !== previous) children.push(el("div", { class: "side-rail-sep", role: "separator" }));
      children.push(railButtons[id]);
      previous = RAIL_GROUP[id];
    }
    rail.replaceChildren(...children, about);
  }

  function section(id: string, label: string, ...actions: HTMLElement[]): HTMLElement {
    return el("section", { class: "side-section", "aria-labelledby": id }, el("h3", { id, class: "side-label" }, label), ...actions);
  }

  // Rebuilt only when the category set changes, so the box that was clicked keeps its focus.
  let categoryKey = "";
  let categoryInputs: HTMLInputElement[] = [];
  const favoritesInput = checkbox();
  favoritesInput.addEventListener("change", applyFilters);
  const favoritesRow = el("label", { class: "side-category" }, favoritesInput, "Favorites");

  function applyFilters(): void {
    const unchecked = categoryInputs.filter((i) => !i.checked).map((i) => i.value);
    void setReviewFilter(unchecked, favoritesInput.checked);
  }

  function syncCategories(reviewRoot: TreeNode): void {
    const mains = (state.tree?.nodes ?? []).filter(isMainCategory);
    const names = mains.map((c) => c.name);
    const hidden = hiddenCategories(reviewRoot);
    const key = names.join(",");
    if (key !== categoryKey) {
      categoryKey = key;
      categoryInputs = names.map((name) => checkbox({ value: name }));
      for (const input of categoryInputs) input.addEventListener("change", applyFilters);
      categories.replaceChildren(favoritesRow, ...categoryInputs.map((input, i) => el("label", { class: "side-category" }, input, nameWithoutIdentifier(mains[i]))));
    }
    favoritesInput.checked = favoritesShown(reviewRoot);
    for (const input of categoryInputs) input.checked = !hidden.includes(input.value);
  }

  function show(asCollapsed: boolean): void {
    collapsed = asCollapsed;
    panel.hidden = collapsed;
    rail.hidden = !collapsed;
  }

  function setCollapsed(on: boolean): void {
    collapseAfterEdit = false;
    savePref(PANEL_PREF, on ? "hidden" : "shown");
    show(on);
  }

  // Focus follows only when it sat in the half being hidden, so a shortcut from the views keeps it there.
  function toggleCollapsed(): void {
    const hadFocus = (collapsed ? rail : panel).contains(document.activeElement);
    setCollapsed(!collapsed);
    if (hadFocus) (collapsed ? railExpand : collapse).focus();
  }

  function editDetails(): void {
    if (!collapsed) {
      details.startEdit();
      return;
    }
    show(false);
    if (details.startEdit()) collapseAfterEdit = true;
    else show(true);
  }

  function render(): void {
    details.render();
    automations.render();
    reviewTools.render();
    layoutRail();
    const n = selectedNode();
    const idle = !state.busy;
    title.textContent = n ? nameWithoutIdentifier(n) : "Details";
    for (const b of [board, railBoard]) b.setAttribute("aria-pressed", String(state.virtual === "board"));
    // The review root keeps its filter but no details; its links show the same filter above their details.
    const review = n !== null && isReviewRoot(n);
    const reviewRoot = state.tree?.nodes.find(isReviewRoot) ?? null;
    const filtered = review || !!n?.linked;
    // Outcomes and review folders belong to the app: they show their automations instead of details.
    details.element.hidden = review || n?.nodeType?.base === "OUT" || n?.nodeType?.base === "REV";
    categoriesSection.hidden = !filtered || reviewRoot === null;
    if (filtered && reviewRoot) syncCategories(reviewRoot);
    hideLinks.classList.toggle("is-active", state.hidingLinks);
    // The review root and its links have no order or delete: their folders are the app's or the owners'.
    order.hidden = foot.hidden = n === null || review || n.linked;
    quick.hidden = n === null || !quickActionsShown();
    favorite.setLabel(favoriteLabel(n));
    const marked = allImportant();
    important.setLabel(importantLabel(marked));
    important.button.classList.toggle("is-important", marked);
    favorite.button.classList.toggle("is-favorite", !!n?.favorite);
    railFavorite.classList.toggle("is-favorite", !!n?.favorite);
    railFavorite.setAttribute("aria-label", favoriteLabel(n));
    railFavorite.title = favoriteLabel(n);
    const available: [HTMLButtonElement[], boolean][] = [
      [[open.button], n !== null],
      [[favorite.button, railFavorite], canActOn("favorite")],
      [[important.button], canMarkImportant()],
      [[railEdit], n !== null && canEdit(n)],
      [[up.button, railUp], canActOn("moveUp")],
      [[down.button, railDown], canActOn("moveDown")],
      [[remover.button, railDelete], canActOn("delete")],
      [[hideButton.button], canActOn("hide")],
    ];
    // An action the node does not offer is removed, not greyed out.
    for (const [buttons, on] of available) for (const b of buttons) b.hidden = !on;
    for (const b of [up.button, railUp, down.button, railDown]) setDisabled(b, !idle);
    order.hidden ||= !(canActOn("moveUp") || canActOn("moveDown"));
    foot.hidden ||= !(canActOn("delete") || canActOn("hide"));
  }

  layoutRail();
  panel.hidden = collapsed;
  rail.hidden = !collapsed;
  return {
    elements: [panel, rail],
    render,
    toggleCollapsed,
    editDetails,
  };
}