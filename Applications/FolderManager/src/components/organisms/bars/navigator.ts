// Navigation bar: back/forward/up over the folders visited plus a breadcrumb of the selected path.
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { iconButton } from "../../atoms/icon-button";
import { createPathInput, rootName } from "./path-input";
import { currentPlace, folderOf, samePlace, type Place } from "../../../services/store/place";
import { findNode, keyOf, select, selectedNode, state } from "../../../services/store/state";
import { attachMenu } from "../menus/anchored-menu";
import { folderIcon } from "../../atoms/icons";
import { goToPlace, openVirtual } from "../../../hooks/virtual-nav";
import { openEntry } from "../../molecules/virtual-entry";
import { reportError } from "./status-line";
import type { Trail } from "../../../hooks/tabs";
import type { TreeNode } from "../../../services/ipc/types";
import { currentSplit, focusPane, paneOn, type Split } from "../../../hooks/split";

interface Crumb {
  label: string;
  go(): void;
  depth?: number;
  parts?: string[];
}

export function createNavigator(shownFolder: () => TreeNode | null | undefined): { element: HTMLElement; render(): void; editPath(): void; goUp(): void; trail(): Trail; setTrail(trail: Trail): void; enter(trail: Trail, place: Place): Promise<void> } {
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
  // In a split each pane keeps its own address: the focused one is live, the other one is read-only and a click moves there.
  const other = el("button", { type: "button", class: "breadcrumb breadcrumb-other", title: "Work in this pane" });
  other.hidden = true;
  other.addEventListener("click", () => {
    const split = currentSplit();
    if (split) focusPane(!split.right);
  });
  const element = el("div", { class: "nav-bar" }, back, forward, up, path, other);

  new ResizeObserver(() => placeOther()).observe(element);

  function otherCrumbs(): (Node | string)[] {
    const parked = currentSplit()?.other;
    const parts = parked?.selectedKey?.split("/") ?? [];
    const shown = findNode(parts)?.file ? parts.slice(0, -1) : parts;
    const labels = parked?.virtual ? [parked.virtual.toUpperCase(), ...(parked.via === null ? [] : shown.slice(parked.via.split("/").length - 1))] : shown;
    const sep = () => el("span", { class: "crumb-sep", "aria-hidden": "true" }, guiIcon("separator"));
    return [el("span", { class: "crumb crumb-root" }, rootName(state.tree?.root)), ...labels.flatMap((label, i) => [sep(), el("span", { class: i === labels.length - 1 ? "crumb current" : "crumb" }, label)])];
  }

  // When the split opens the left address is made to end where the left pane ends, kept as a share of the bar so it scales with the window;
  // resizing the panes later leaves it as it is.
  const leftShares = new WeakMap<Split, number>();
  function alignOnce(split: Split, leftBox: HTMLElement, rightBox: HTMLElement): void {
    if (leftShares.has(split)) return;
    const pane = paneOn(false)?.host.getBoundingClientRect();
    const start = leftBox.getBoundingClientRect().left;
    // Flex grows only the space past each box's padding and border, so the share is taken of that space.
    const frame = (box: HTMLElement): number => {
      const css = getComputedStyle(box);
      return [css.paddingLeft, css.paddingRight, css.borderLeftWidth, css.borderRightWidth].reduce((sum, v) => sum + (parseFloat(v) || 0), 0);
    };
    const free = leftBox.getBoundingClientRect().width + rightBox.getBoundingClientRect().width - frame(leftBox) - frame(rightBox);
    if (!pane || pane.width === 0 || free <= 0) return;
    leftShares.set(split, Math.min(0.8, Math.max(0.2, (pane.right - start - frame(leftBox)) / free)));
  }

  // Below this width two addresses would crowd each other, so only the focused pane's shows.
  const BOTH_MIN = 640;
  function placeOther(): void {
    const split = currentSplit();
    other.hidden = split === null || element.clientWidth < BOTH_MIN;
    path.style.flex = other.style.flex = "";
    if (!split || other.hidden) return;
    if (split.right) path.before(other);
    else path.after(other);
    const [leftBox, rightBox] = split.right ? [other, path] : [path, other];
    leftBox.style.flex = rightBox.style.flex = "1 1 0";
    alignOnce(split, leftBox, rightBox);
    const share = leftShares.get(split) ?? 0.5;
    leftBox.style.flex = `${share} 1 0`;
    rightBox.style.flex = `${1 - share} 1 0`;
    other.replaceChildren(...otherCrumbs());
    other.setAttribute("aria-label", `Other pane: ${other.textContent ?? ""}`);
  }

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
    // The trail is the folder on screen; a view with no single folder falls back to the selection, minus a selected file.
    const shown = shownFolder();
    const parts = shown !== undefined ? (shown?.relParts ?? []) : (state.selectedKey?.split("/") ?? []).slice(0, selectedNode()?.file ? -1 : undefined);
    const crumb = (end: number): Crumb => ({ label: parts[end - 1] ?? "", go: () => select(parts.slice(0, end)), depth: end, parts: parts.slice(0, end) });
    const entry = openEntry();
    if (entry === null) return parts.map((_, i) => crumb(i + 1));
    const head: Crumb = { label: entry.label, go: () => void openVirtual(entry.id) };
    if (state.via === null || parts.length === 0) return [head];
    const from = state.via.split("/").length;
    return [head, ...parts.slice(from - 1).map((_, i) => crumb(from + i))];
  }

  function crumbs(): (Node | string)[] {
    const out: (Node | string)[] = [];
    // Steps inside the levels the database indexes are marked, so where its scope ends shows.
    const sepClass = (depth: number) => (depth <= state.depthLimit ? "crumb-sep db-edge" : "crumb-sep");
    // A separator after a real folder lists its subfolders, as Explorer's address bar does, to jump sideways without leaving the bar.
    const sep = (depth = 1, parent?: string[], next?: string) => {
      if (!parent) return el("span", { class: sepClass(depth), "aria-hidden": "true" }, guiIcon("separator"));
      const b = el("button", { type: "button", class: `${sepClass(depth)} crumb-menu`, "aria-label": `Folders in ${parent.at(-1) ?? rootName(state.tree?.root)}` }, guiIcon("separator"));
      const folders = () => (parent.length === 0 ? (state.tree?.nodes ?? []) : (findNode(parent)?.children ?? [])).filter((n) => !n.file);
      attachMenu(b, "Folders", () => folders().map((n) => ({ label: n.name, checked: n.name === next, lead: () => folderIcon(n), run: () => {
        state.entering = keyOf(n.relParts);
        select(n.relParts);
      } })), false, true);
      return b;
    };
    const rootBtn = el("button", { type: "button", class: "crumb crumb-btn crumb-root", title: state.tree?.root ?? "" }, rootName(state.tree?.root));
    rootBtn.addEventListener("click", () => select(null));
    out.push(rootBtn);
    const steps = trail();
    if (steps.length > 0) out.push(steps[0]?.parts ? sep(1, [], steps[0].parts.at(-1)) : sep());
    steps.forEach((s, i) => {
      const leaf = i === steps.length - 1;
      const b = el("button", { type: "button", class: leaf ? "crumb crumb-btn current" : "crumb crumb-btn" }, s.label);
      if (leaf) b.setAttribute("aria-current", "location");
      b.addEventListener("click", s.go);
      out.push(b);
      if (!leaf) out.push(sep(steps[i + 1]?.depth, s.parts, steps[i + 1]?.parts?.at(-1)));
    });
    return out;
  }

  function render(): void {
    record();
    back.setAttribute("aria-disabled", String(at <= 0));
    forward.setAttribute("aria-disabled", String(at >= history.length - 1));
    up.setAttribute("aria-disabled", String(state.selectedKey === null || selectedNode() === null));
    placeOther();
    if (pathInput.editing()) return;
    path.replaceChildren(...crumbs());
    path.scrollLeft = path.scrollWidth;
  }

  function setTrail(trail: Trail): void {
    history = trail.history;
    at = trail.at;
  }

  // A tab brings its own history, so reaching its place adds nothing to it.
  async function enter(trail: Trail, place: Place): Promise<void> {
    setTrail(trail);
    await restore(place.key === null && place.virtual === null ? undefined : place);
  }

  return { element, render, editPath: pathInput.start, goUp, trail: () => ({ history, at }), setTrail, enter };
}
