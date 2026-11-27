// Right column: quick actions and details of the selected node, collapsible to an icon rail.
import { el } from "../../dom";
import { iconButton } from "../../atoms/icon-button";
import { checkbox } from "../../atoms/checkbox";
import { allImportant, canActOn, canMarkImportant, deleteActed, favoriteActed, hideActed, importantActed } from "../../../hooks/acted-nodes";
import { setReviewFilter } from "../../../hooks/ops";
import { reviewFilter } from "../../../services/store/review-list";
import { readPref, savePref } from "../../../services/store/prefs";
import { importantLabel, isMainCategory, isReviewRoot, nameWithoutIdentifier } from "../../../services/store/node-info";
import { quickActionsShown, railLayout, type RailId } from "../../../services/store/side-layout";
import { selectedNode, state, toggleHidingLinks } from "../../../services/store/state";
import { shownEntry } from "../../molecules/virtual-entry";
import { canEdit, createDetailsPanel } from "./details-panel";
import { createAutomationsPanel } from "./automations-panel";
import { createPreviewPanel } from "./preview-panel";
import { createReviewAutomations } from "./review-automations";
import { toggleBoard } from "../../../hooks/virtual-nav";
import { aiConnectDialog } from "../dialogs/ai-connect-dialog/ai-connect-dialog";
import { sideAction } from "../../molecules/side-action";
import { tourSeen } from "../tour/onboarding-tour";
import type { TreeNode } from "../../../services/ipc/types";

const PANEL_PREF = "fm.details";
// Function groups of the collapsed rail: view settings, item state, ordering, removal.
const RAIL_GROUP: Record<RailId, number> = { settings: 0, board: 0, favorite: 1, edit: 1, delete: 2 };

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

  const automations = createAutomationsPanel();
  const reviewTools = createReviewAutomations();
  const preview = createPreviewPanel();
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
  const head = el("div", { class: "side-head" }, title, board, collapse);

  const open = sideAction("Open", null, openSelected, "");
  const favorite = sideAction(favoriteLabel(null), "starred", favoriteActed);
  const important = sideAction(importantLabel(false), "important", importantActed);
  const quick = section("side-quick", "Quick actions", open.button, favorite.button, important.button);
  const hideButton = sideAction("Hide element", "hide", hideActed);
  const remover = sideAction("Delete", "trash", deleteActed, "danger");
  // App-wide entries close the panel's foot at the far right, apart from the item tools, as on the collapsed rail.
  const footTools = el("div", { class: "side-foot-tools" }, hideButton.button, remover.button);
  const appRow = el("div", { class: "side-foot-app" },
    iconButton("Connect AI", "aiConnect", () => void aiConnectDialog()),
    iconButton("About", "pageAbout", () => openSettings("about")),
    settings);
  const foot = el("div", { class: "side-foot" }, footTools, appRow);
  const categories = el("div", { class: "side-categories" });
  const hideLinks = el("a", { href: "#", class: "side-link" }, "Select elements to hide");
  hideLinks.addEventListener("click", (ev) => {
    ev.preventDefault();
    toggleHidingLinks();
  });
  const categoriesSection = section("side-categories", "Filters", categories, hideLinks);
  const panel = el("aside", { class: "details-panel", "aria-label": "Side panel" }, head, details.welcome, quick, categoriesSection, reviewTools.element, details.element, automations.element, preview.element, foot);

  // ─────────♦ COLLAPSED RAIL ───────────────────────────────────────────────────
  const railExpand = iconButton("Expand side panel", "chevronsLeft", () => {
    setCollapsed(false);
    collapse.focus();
  });
  railExpand.classList.add("side-rail-expand");
  const railFavorite = iconButton(favoriteLabel(null), "starred", favoriteActed);
  const railEdit = iconButton("Edit details", "pencil", editDetails);
  const railBoard = iconButton("Board", "layout", () => void toggleBoard());
  const railDelete = iconButton("Delete", "trash", deleteActed);
  railDelete.classList.add("danger");
  const about = iconButton("About", "pageAbout", () => openSettings("about"));
  const connectAi = iconButton("Connect AI", "aiConnect", () => void aiConnectDialog());
  connectAi.classList.add("side-rail-end");
  const railButtons: Record<RailId, HTMLButtonElement> = {
    settings: iconButton("Settings", "settings", () => openSettings()),
    board: railBoard,
    favorite: railFavorite,
    edit: railEdit,
    delete: railDelete,
  };

  const rail = el("aside", { class: "side-rail", "aria-label": "Side panel" });
  let railKey: string | null = null;

  // Rebuilt only when the layout changes, since replacing the buttons drops keyboard focus.
  function layoutRail(): void {
    const shown = railLayout()
      .filter((i) => i.shown)
      .map((i) => i.id);
    // Settings is app-wide, so it sits with About at the rail's foot instead of among the item actions.
    const ids = shown.filter((id) => id !== "settings");
    const foot = shown.includes("settings") ? [railButtons.settings] : [];
    const key = shown.join(",");
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
    rail.replaceChildren(...children, connectAi, about, ...foot);
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
    setReviewFilter(unchecked, favoritesInput.checked);
  }

  function syncCategories(): void {
    const mains = (state.tree?.nodes ?? []).filter(isMainCategory);
    const names = mains.map((c) => c.name);
    const { hidden, favorites } = reviewFilter();
    const key = names.join(",");
    if (key !== categoryKey) {
      categoryKey = key;
      categoryInputs = names.map((name) => checkbox({ value: name }));
      for (const input of categoryInputs) input.addEventListener("change", applyFilters);
      categories.replaceChildren(favoritesRow, ...categoryInputs.map((input, i) => el("label", { class: "side-category" }, input, nameWithoutIdentifier(mains[i]))));
    }
    favoritesInput.checked = favorites;
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
    about.classList.toggle("tour-pulse", !tourSeen());
    details.render();
    automations.render();
    reviewTools.render();
    preview.render();
    layoutRail();
    const n = selectedNode();
    title.textContent = n ? (n.depth === 0 ? n.name.replace(/^[^.]+\./, "") : n.fixedPrefix ? n.fixedPrefix.replace(/[._]+$/, "") : nameWithoutIdentifier(n)) : (shownEntry()?.label ?? "Details");
    for (const b of [board, railBoard]) b.setAttribute("aria-pressed", String(state.virtual === "board"));
    // The review root keeps its filter but no details; its links show the same filter above their details.
    const review = n !== null && isReviewRoot(n);
    const filtered = state.virtual === "review";
    // Outcomes and review folders belong to the app: they show their automations instead of details.
    details.element.hidden = review || n?.nodeType?.base === "OUT" || n?.nodeType?.base === "REV";
    categoriesSection.hidden = !filtered;
    if (filtered) syncCategories();
    hideLinks.classList.toggle("is-active", state.hidingLinks);
    // The review root and its links have no delete: their folders are the app's or the owners'.
    footTools.hidden = n === null || review || n.linked;
    quick.hidden = n === null || !quickActionsShown();
    open.setLabel(n?.file ? "Open" : "Open folder location");
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
      [[remover.button, railDelete], canActOn("delete")],
      [[hideButton.button], canActOn("hide")],
    ];
    // An action the node does not offer is removed, not greyed out.
    for (const [buttons, on] of available) for (const b of buttons) b.hidden = !on;
    footTools.hidden ||= !(canActOn("delete") || canActOn("hide"));
    tidySeparators();
  }

  // A group whose buttons are all hidden leaves no separator behind, so two never stand together.
  function tidySeparators(): void {
    let shownSince = false;
    let pending: HTMLElement | null = null;
    for (const child of rail.children) {
      if (!(child instanceof HTMLElement) || child === railExpand) continue;
      if (child.classList.contains("side-rail-sep")) {
        child.hidden = true;
        if (shownSince) pending = child;
        shownSince = false;
      } else if (!child.hidden) {
        if (pending) pending.hidden = false;
        pending = null;
        shownSince = true;
      }
    }
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
