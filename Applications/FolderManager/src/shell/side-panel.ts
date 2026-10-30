// Right column: quick actions, details and order of the selected node, collapsible to an icon rail.
import { el } from "../ui/dom";
import { iconButton } from "../ui/icon-button";
import { move, remove, setFavorite } from "../actions/ops";
import { readPref, savePref } from "../store/prefs";
import { selectedNode, state } from "../store/state";
import { canEdit, createDetailsPanel } from "./details-panel";
import { openSettings } from "./settings-dialog";
import { setDisabled, sideAction } from "./side-action";
import type { TreeNode } from "../ipc/types";

const PANEL_PREF = "fm.details";

export interface SidePanel {
  elements: HTMLElement[];
  render(): void;
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
  const favorite = sideAction(favoriteLabel(null), "starred", toggleFavorite, "favorite");
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
  const railFavorite = iconButton(favoriteLabel(null), "starred", toggleFavorite);
  const railEdit = iconButton("Edit details", "pencil", () => {
    show(false);
    if (details.startEdit()) collapseAfterEdit = true;
    else show(true);
  });
  const railUp = iconButton("Move up", "moveUp", moveUp);
  const railDown = iconButton("Move down", "moveDown", moveDown);
  const railDelete = iconButton("Delete", "trash", del);
  railDelete.classList.add("danger");
  const about = iconButton("About", "pageAbout", () => void openSettings("about"));
  about.classList.add("side-rail-end");
  const rail = el(
    "aside",
    { class: "side-rail", "aria-label": "Side panel" },
    railExpand,
    iconButton("Settings", "settings", () => void openSettings()),
    railFavorite,
    railEdit,
    railUp,
    railDown,
    railDelete,
    about,
  );

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

  function render(): void {
    details.render();
    const n = selectedNode();
    const a = n?.actions;
    const idle = !state.busy;
    title.textContent = n?.name ?? "Details";
    quick.hidden = order.hidden = foot.hidden = n === null;
    favorite.setLabel(favoriteLabel(n));
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

  panel.hidden = collapsed;
  rail.hidden = !collapsed;
  return {
    elements: [panel, rail],
    render,
  };
}
