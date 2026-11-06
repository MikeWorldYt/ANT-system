// Navigation bar: back/forward/up over the folders visited plus a breadcrumb of the selected path.
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { iconButton } from "../../atoms/icon-button";
import { createPathInput, rootName } from "./path-input";
import { currentPlace, folderOf, samePlace, type Place } from "../../../services/store/place";
import { findNode, select, selectedNode, state } from "../../../services/store/state";
import { goToPlace, openVirtual } from "../../../hooks/virtual-nav";
import { openEntry } from "../../molecules/virtual-entry";
import { reportError } from "./status-line";
import type { Trail } from "../../../hooks/tabs";

interface Crumb {
  label: string;
  go(): void;
}

export function createNavigator(): { element: HTMLElement; render(): void; editPath(): void; goUp(): void; trail(): Trail; enter(trail: Trail, place: Place): Promise<void> } {
  let history: Place[] = [];
  let at = -1;
  // Restoring a place passes through others (the entry, its favorite), which must not become entries.
  let restoring = false;

  // History is per folder: moving among the children of one folder updates its entry instead of adding one.
  function record(): void {
    const place = currentPlace();
    const current = history[at];
    if (restoring || (place.key === null && place.virtual === null) || (current && samePlace(current, place))) return;
    history.splice(at + 1);
    const folder = folderOf(place);
    if (current && folder !== null && folder === folderOf(current)) history[at] = place;
    else history.push(place);
    at = history.length - 1;
  }

  async function restore(place: Place | undefined): Promise<void> {
    restoring = true;
    try {
      await (place ? goToPlace(place) : Promise.resolve(select(null)));
    } finally {
      restoring = false;
    }
  }

  async function go(index: number): Promise<void> {
    const place = history[index];
    if (place === undefined) return;
    const previous = at;
    try {
      await restore(place);
    } catch (e) {
      reportError("error", e);
      return;
    }
    at = index;
    if (place.key !== null && !findNode(place.key.split("/"))) {
      history.splice(index, 1);
      at = previous >= index ? previous - 1 : previous;
      // The missing folder was selected, so the selection goes back to the entry now current.
      await restore(history[at]).catch((e: unknown) => reportError("error", e));
    }
    // A place may come back changed (a favorite removed meanwhile), so the entry takes what was reached.
    if (history[at]) history[at] = currentPlace();
    render();
  }

  const parentOf = (): string[] | null => {
    const parts = state.selectedKey?.split("/") ?? [];
    return parts.length > 1 ? parts.slice(0, -1) : null;
  };

  // A listed entry's parent is its virtual list, not the real folder it lives in.
  function goUp(): void {
    if (state.virtual !== null && state.selectedKey !== null && state.selectedKey === state.via) return void openVirtual(state.virtual);
    select(parentOf());
  }

  const back = iconButton("Back", "back", () => void go(at - 1));
  const forward = iconButton("Forward", "forward", () => void go(at + 1));
  const up = iconButton("Up one level", "up", goUp);
  const path = el("nav", { class: "breadcrumb", "aria-label": "Current location", title: "Double-click or press Ctrl+L to type a path" });
  const element = el("div", { class: "nav-bar" }, back, forward, up, path);

  // A typed place lands as one history entry, not one per step on the way.
  const pathInput = createPathInput(path, render, restore);

  path.addEventListener("dblclick", pathInput.start);
  document.addEventListener("keydown", (ev) => {
    if (pathInput.editing() || !ev.altKey || ev.ctrlKey || ev.shiftKey || ev.metaKey) return;
    if (ev.key === "ArrowLeft") void go(at - 1);
    else if (ev.key === "ArrowRight") void go(at + 1);
    else return;
    ev.preventDefault();
  });

  // Inside a virtual entry the trail runs through it and the entry the selection went through, not the real folders.
  function trail(): Crumb[] {
    // A selected file cannot be opened as a folder, so the trail ends at the folder that holds it.
    const parts = (state.selectedKey?.split("/") ?? []).slice(0, selectedNode()?.file ? -1 : undefined);
    const crumb = (end: number): Crumb => ({ label: parts[end - 1] ?? "", go: () => select(parts.slice(0, end)) });
    const entry = openEntry();
    if (entry === null) return parts.map((_, i) => crumb(i + 1));
    const head: Crumb = { label: entry.label, go: () => void openVirtual(entry.id) };
    if (state.via === null) return [head];
    const from = state.via.split("/").length;
    return [head, ...parts.slice(from - 1).map((_, i) => crumb(from + i))];
  }

  function crumbs(): (Node | string)[] {
    const out: (Node | string)[] = [];
    const sep = () => el("span", { class: "crumb-sep", "aria-hidden": "true" }, guiIcon("separator"));
    const rootBtn = el("button", { type: "button", class: "crumb crumb-btn crumb-root", title: state.tree?.root ?? "" }, rootName(state.tree?.root));
    rootBtn.addEventListener("click", () => select(null));
    out.push(rootBtn);
    const steps = trail();
    if (steps.length > 0) out.push(sep());
    steps.forEach((s, i) => {
      const leaf = i === steps.length - 1;
      const b = el("button", { type: "button", class: leaf ? "crumb crumb-btn current" : "crumb crumb-btn" }, s.label);
      if (leaf) b.setAttribute("aria-current", "location");
      b.addEventListener("click", s.go);
      out.push(b);
      if (!leaf) out.push(sep());
    });
    return out;
  }

  function render(): void {
    record();
    back.setAttribute("aria-disabled", String(at <= 0));
    forward.setAttribute("aria-disabled", String(at >= history.length - 1));
    up.setAttribute("aria-disabled", String(state.selectedKey === null || selectedNode() === null));
    if (pathInput.editing()) return;
    path.replaceChildren(...crumbs());
    path.scrollLeft = path.scrollWidth;
  }

  // A tab brings its own history, so reaching its place adds nothing to it.
  async function enter(trail: Trail, place: Place): Promise<void> {
    history = trail.history;
    at = trail.at;
    await restore(place.key === null && place.virtual === null ? undefined : place);
  }

  return { element, render, editPath: pathInput.start, goUp, trail: () => ({ history, at }), enter };
}
