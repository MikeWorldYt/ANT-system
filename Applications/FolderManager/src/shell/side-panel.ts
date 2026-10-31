// Right column: quick actions, details and order of the selected node, collapsible to an icon rail.
import { el } from "../ui/dom";
import { iconButton } from "../ui/icon-button";
import { move, remove, setFavorite } from "../actions/ops";
import { readPref, savePref } from "../store/prefs";
import { nameWithoutIdentifier } from "../store/node-info";
import { quickActionsShown, railLayout, type RailId } from "../store/side-layout";
import { selectedNode, state } from "../store/state";
import { canEdit, createDetailsPanel } from "./details-panel";
import { openSettings } from "./settings-dialog";
import { setDisabled, sideAction } from "./side-action";
import type { TreeNode } from "../ipc/types";

const PANEL_PREF = "fm.details";
// Function groups of the collapsed rail: view settings, item state, ordering, removal.
const RAIL_GROUP: Record<RailId, number> = { settings: 0, favorite: 1, edit: 1, moveUp: 2, moveDown: 2, delete: 3 };

export interface SidePanel {
  elements: HTMLElement[];
  render(): void;
  toggleCollapsed(): void;
  editDetails(): void;
}

const favoriteLabel = (n: TreeNode | null) => (n?.favorite ? "Remove from favorites" : "Add to favorites");

export function createSidePanel(openSelected: () => void): SidePanel {
  let collapsed = readPref(PANEL_PREF) === "hidden";
  // Set when the rail pencil expanded the panel, so the end of that edit returns to the rail.
  let collapseAfterEdit = false;

  const act = (fn: (n: TreeNode) => void) => () => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const toggleFavorite = act((n) => void setFavorite(n, !n.favorite));
  const moveUp = act((n) => void move(n, "up"));
  const moveDown = act((n) => void move(n, "down"));
  const del = act((n) => void remove(n));

  const details = createDetailsPanel(() => {
    if (!collapseAfterEdit) return;
    collapseAfterEdit = false;
    const focusLost = document.activeElement === null || document.activeElement === document.body;
    show(true);
    if (focusLost) railEdit.focus();
  });

  // ─────────♦ EXPANDED PANEL ───────────────────────────────────────────────────
  const title = el("h2", { class: "side-title" });
  const settings = iconButton("Settings", "settings", () => void openSettings());
  const collapse = iconButton("Collapse side panel", "chevronsRight", () => {
    setCollapsed(true);
    railExpand.focus();
  });
  collapse.classList.add("side-collapse");
  const head = el("div", { class: "side-head" }, title, settings, collapse);

  const open = sideAction("Open", null, openSelected, "");
  const favorite = sideAction(favoriteLabel(null), "starred", toggleFavorite);
  const quick = section("side-quick", "Quick actions", open.button, favorite.button);
  const up = sideAction("Move up", "moveUp", moveUp, "accent");
  const down = sideAction("Move down", "moveDown", moveDown, "accent");
  const order = section("side-order", "Order", up.button, down.button);
  const remover = sideAction("Delete", "trash", del, "danger");
  const foot = el("div", { class: "side-foot" }, remover.button);
  const panel = el("aside", { class: "details-panel", "aria-label": "Side panel" }, head, quick, details.element, order, foot);

  // ─────────♦ COLLAPSED RAIL ───────────────────────────────────────────────────
  const railExpand = iconButton("Expand side panel", "chevronsLeft", () => {
    setCollapsed(false);
    collapse.focus();
  });
  railExpand.classList.add("side-rail-expand");
  const railFavorite = iconButton(favoriteLabel(null), "starred", toggleFavorite);
  const railEdit = iconButton("Edit details", "pencil", editDetails);
  const railUp = iconButton("Move up", "moveUp", moveUp);
  const railDown = iconButton("Move down", "moveDown", moveDown);
  const railDelete = iconButton("Delete", "trash", del);
  railDelete.classList.add("danger");
  const about = iconButton("About", "pageAbout", () => void openSettings("about"));
  about.classList.add("side-rail-end");
  const railButtons: Record<RailId, HTMLButtonElement> = {
    settings: iconButton("Settings", "settings", () => void openSettings()),
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
    layoutRail();
    const n = selectedNode();
    const a = n?.actions;
    const idle = !state.busy;
    title.textContent = n ? nameWithoutIdentifier(n) : "Details";
    order.hidden = foot.hidden = n === null;
    quick.hidden = n === null || !quickActionsShown();
    favorite.setLabel(favoriteLabel(n));
    favorite.button.classList.toggle("is-favorite", !!n?.favorite);
    railFavorite.classList.toggle("is-favorite", !!n?.favorite);
    railFavorite.setAttribute("aria-label", favoriteLabel(n));
    railFavorite.title = favoriteLabel(n);
    const enabled: [HTMLButtonElement[], boolean][] = [
      [[open.button], n !== null],
      [[favorite.button, railFavorite], !!a?.favorite],
      [[railEdit], n !== null && canEdit(n)],
      [[up.button, railUp], !!(a?.moveUp && idle)],
      [[down.button, railDown], !!(a?.moveDown && idle)],
      [[remover.button, railDelete], !!a?.delete],
    ];
    for (const [buttons, on] of enabled) for (const b of buttons) setDisabled(b, !on);
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
