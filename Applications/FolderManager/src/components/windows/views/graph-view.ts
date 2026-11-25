// Graph view: the NODE and WKB folders as a mind map that unfolds to the right; other types stay out so the structure reads at a glance.
import { el } from "../../dom";
import { rootName } from "../../organisms/bars/path-input";
import { nodeContextMenu } from "../../organisms/menus/context-menu";
import { folderIcon } from "../../atoms/icons";
import { keepExtensionInSight } from "../../atoms/fit-name";
import { toggleExpand } from "../../../hooks/ops";
import { folderMoveActive } from "../../../hooks/folder-move";
import { openFolder } from "../../../hooks/path-actions";
import { readPref, savePref } from "../../../services/store/prefs";
import { reportError } from "../../organisms/bars/status-line";
import { clickSelect, findNode, hydrate, isVirtualShown, notify, keyOf, select, state, virtualRoot } from "../../../services/store/state";
import { openVirtual } from "../../../hooks/virtual-nav";
import { keepChildren } from "../../../services/store/virtual";
import { VIRTUAL_ENTRIES } from "../../molecules/virtual-entry";
import { guiIcon } from "../../atoms/gui-icons";
import { sendPacket } from "./graph-packets";
import type { Trip } from "../../../hooks/file-trips";
import { bandSelect } from "./graph-band";
import { boxMenu, boxView, fileArt, fileDetail, fitWidth, keepSize, sortedFiles } from "./graph-box-view";
import type { FilesMoved } from "../../../hooks/automations/batch-replay";
import { DOWNLOADS_ROOT, inSystem, systemName, type TreeNode } from "../../../services/ipc/types";
import { isSystemRoot } from "../../../services/store/system-folders";
import { listChildren } from "../../../services/ipc/api";
import { play, snapshot } from "./graph-motion";
import type { ViewMode } from "./view-mode";

const SVG = "http://www.w3.org/2000/svg";
const BREATH_MS = 2400;
const breathPhase = () => `-${Math.round(performance.now() % BREATH_MS)}ms`;
const isNode = (n: TreeNode) => !n.file && (n.nodeType?.base === "NODE" || n.nodeType?.base === "WKB");
const UNDER = new Set(["REC", "SS", "RES", "REV"]);
// Folders numbered 00. sit above their parent, so they leave the fan of sub-nodes.
const isTopNote = (n: TreeNode) => !n.file && n.name.startsWith("00.");
const isSupport = (n: TreeNode) => !n.file && (n.name.startsWith("00.") || UNDER.has(n.nodeType?.base ?? ""));
const nodesOf = (list: TreeNode[] | null | undefined) => (list ?? []).filter((n) => isNode(n) && !isTopNote(n));

// A straight elbow with rounded corners: out to the split, along it, then in to the child.
function bracket(x1: number, y1: number, sx: number, x2: number, y2: number): string {
  const dy = y2 - y1;
  const r = Math.min(8, Math.abs(dy) / 2, (sx - x1) / 2);
  if (r < 1) return `M${x1},${y1} H${sx} V${y2} H${x2}`;
  const s = Math.sign(dy);
  return `M${x1},${y1} H${sx - r} Q${sx},${y1} ${sx},${y1 + s * r} V${y2 - s * r} Q${sx},${y2} ${sx + r},${y2} H${x2}`;
}

export function createGraphView(): ViewMode {
  const canvas = el("div", { class: "graph-canvas" });
  const lines = document.createElementNS(SVG, "svg");
  lines.classList.add("graph-lines");
  lines.setAttribute("aria-hidden", "true");
  const stage = el("div", { class: "graph-stage" }, lines, canvas);
  const root = el("div", { class: "graph", role: "tree", tabindex: "0", "aria-label": "Node graph" }, stage);
  // Parent of each shown node, so the arrow keys can walk the graph.
  let parents = new Map<string, TreeNode | null>();
  let frame = 0;
  // The lit path stands a little to the right; ids say which element moved (branch, column or card).
  let shifted = new Set<string>();
  const SHIFT_MS = 320;
  let motionMs = 0;
  let fresh = true;

  // Branches whose sub-nodes stood blurred last render; they start there so the blur animates in and out.
  let blurred = new Set<string>();

  // Folding a node also folds its open support folder, so its sub-nodes come back when it opens again.
  function fold(n: TreeNode): void {
    if (state.expanded.has(keyOf(n.relParts))) for (const c of n.children ?? []) if (isSupport(c)) state.expanded.delete(keyOf(c.relParts));
    void toggleExpand(n);
  }

  // What was unfolded is kept per storage, so the graph opens as it was left.
  let restoredFor: string | null = null;
  let restoring = false;

  async function restore(): Promise<void> {
    const root = state.tree?.root ?? "";
    let keys: string[] = [];
    try {
      const raw: unknown = JSON.parse(readPref(`fm.graph.expanded:${root}`) ?? "[]");
      if (Array.isArray(raw)) keys = raw.filter((k): k is string => typeof k === "string");
    } catch (e) {
      reportError("warning", e, "Map layout");
    }
    restoring = true;
    // All keys go in at once and load in one pass with one redraw; opening them one by one redrew and animated the graph for every folder.
    try {
      for (const key of keys) state.expanded.add(key);
      await hydrate();
    } catch (e) {
      reportError("warning", e, "Map layout");
    } finally {
      restoring = false;
      notify();
    }
  }

  // Whichever of the graph or its host actually scrolls.
  const scroller = (): HTMLElement => (root.scrollWidth > root.clientWidth + 1 || root.scrollHeight > root.clientHeight + 1 ? root : (root.parentElement ?? root));

  // As in Columns, a new selection or fold brings what it opened in from the right; what is left behind matters less.
  let revealed = "";
  function reveal(): void {
    const mark = `${state.selectedKey}|${state.expanded.size}`;
    if (mark === revealed || state.selectedKey === null) return;
    revealed = mark;
    const card = canvas.querySelector<HTMLElement>(`[data-key="${CSS.escape(state.selectedKey)}"]`);
    const branchBox = card?.closest(".graph-branch")?.getBoundingClientRect();
    const cardBox = card?.getBoundingClientRect();
    if (!branchBox || !cardBox) return;
    const view = scroller().getBoundingClientRect();
    const over = branchBox.right + 24 - view.right;
    if (over > 0) scroller().scrollBy({ left: Math.min(over, cardBox.left - view.left - 24), behavior: "smooth" });
  }

  // The path held when a support folder or a native folder opened over the branches; a click on the empty graph closes it and goes back there.
  let beforeOverlay: string | null = null;
  const openOverlays = (): string[] =>
    [...state.expanded].filter((k) => {
      if (isSystemRoot(k)) return true;
      const n = findNode(k.split("/"));
      return n !== null && isSupport(n);
    });
  // Several support boxes may stand open across the graph; only the one on the current path, or else the last one opened, closes.
  let lastOpened: string | null = null;
  const currentOverlay = (): string | null => {
    const open = openOverlays();
    const sel = state.selectedKey;
    const onPath = sel === null ? [] : open.filter((k) => sel === k || sel.startsWith(`${k}/`)).sort((a, b) => b.length - a.length);
    return onPath[0] ?? (lastOpened !== null && open.includes(lastOpened) ? lastOpened : null);
  };

  // While files are dragged, resting on the empty graph closes what covers the branches, and resting on a node with its support box open brings its sub-nodes back, so the way to a destination opens as the pointer goes.
  const REST_MS = 600;
  let rest: { spot: string; timer: number } | null = null;
  const onDragMove = (ev: PointerEvent): void => {
    if (!folderMoveActive() || !root.isConnected) return;
    // The pointer is captured by the drag, so where it rests is read from the graph's own bounds and the element drawn there.
    const area = root.getBoundingClientRect();
    if (ev.clientX < area.left || ev.clientX > area.right || ev.clientY < area.top || ev.clientY > area.bottom) return;
    const under = document.elementFromPoint(ev.clientX, ev.clientY);
    if (!under) return;
    const card = under.closest<HTMLElement>(".graph-node-box[data-key]");
    const spot = card ? `card:${card.dataset.key ?? ""}` : under.closest(".graph-files-box") ? "box" : "empty";
    if (rest?.spot === spot) return;
    if (rest) window.clearTimeout(rest.timer);
    rest = null;
    if (spot === "box") return;
    const shut = card
      ? (findNode((card.dataset.key ?? "").split("/"))?.children ?? []).filter((c) => isSupport(c) && state.expanded.has(keyOf(c.relParts))).map((c) => keyOf(c.relParts))
      : [currentOverlay()].filter((k): k is string => k !== null);
    if (shut.length === 0) return;
    rest = {
      spot,
      timer: window.setTimeout(() => {
        rest = null;
        for (const k of shut) state.expanded.delete(k);
        notify();
      }, REST_MS),
    };
  };

  root.addEventListener("click", (ev) => {
    if (!(ev.target instanceof Element) || ev.target.closest(".graph-node-box, .graph-files-box, .graph-file")) return;
    const shut = currentOverlay();
    if (shut === null) return;
    state.expanded.delete(shut);
    const back = shut === lastOpened ? beforeOverlay : null;
    if (shut === lastOpened) {
      beforeOverlay = null;
      lastOpened = null;
    }
    // Without a remembered path, a selection inside what closed goes up to the folder that held it.
    const inside = !isSystemRoot(shut) && state.selectedKey !== null && (state.selectedKey === shut || state.selectedKey.startsWith(`${shut}/`)) ? shut : undefined;
    if (back !== null && findNode(back.split("/"))) select(back.split("/"));
    else if (inside) select(inside.split("/").slice(0, -1));
    else notify();
  });

  // Holding Space turns a drag into panning, the way back to what scrolled away on the left.
  let spaceHeld = false;
  // A click that ends a pan is not a pick.
  root.addEventListener("click", (ev) => {
    if (spaceHeld) ev.stopPropagation();
  }, true);
  root.addEventListener("keyup", (ev) => {
    if (ev.key !== " ") return;
    spaceHeld = false;
    root.classList.remove("panning");
  });
  root.addEventListener("blur", () => {
    spaceHeld = false;
    root.classList.remove("panning");
  });
  // The browser would select the cards' text and drag it as a picture; the graph is browsed, not read as text.
  root.addEventListener("selectstart", (ev) => ev.preventDefault());
  root.addEventListener("dragstart", (ev) => ev.preventDefault());
  root.addEventListener("pointerdown", (ev) => {
    if (spaceHeld) ev.stopPropagation();
  }, true);
  root.addEventListener("mousedown", (ev) => {
    if (!spaceHeld || ev.button !== 0) return;
    ev.preventDefault();
    // The pan owns this drag, so the marquee never starts.
    ev.stopPropagation();
    let x = ev.clientX;
    let y = ev.clientY;
    const move = (m: MouseEvent) => {
      // Each axis may scroll on a different element, so both get the move.
      for (const el of [root, root.parentElement]) el?.scrollBy(x - m.clientX, y - m.clientY);
      x = m.clientX;
      y = m.clientY;
    };
    const up = () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  });

  // What moves for a lit card: the selected one takes its whole branch along, an ancestor only its own column, a stacked support card only itself.
  function shiftTarget(c: HTMLElement): [string, Element] | null {
    const key = c.dataset.key ?? "";
    const head = c.parentElement?.classList.contains("graph-head") ? c.parentElement : null;
    const selected = key === state.selectedKey;
    if (c.classList.contains("graph-root")) return null;
    if (!head) return selected ? [`${key}|c`, c] : null;
    if (selected) {
      const b = head.parentElement;
      return b ? [`${key}|b`, b] : null;
    }
    return [`${key}|h`, head];
  }

  function shiftTargets(): Map<string, Element> {
    const found = new Map<string, Element>();
    for (const c of canvas.querySelectorAll<HTMLElement>(".graph-node-box.on-path")) {
      const t = shiftTarget(c);
      if (t) found.set(t[0], t[1]);
    }
    return found;
  }

  // New elements start where the last render left them, so the move animates instead of jumping.
  function restoreShift(): void {
    for (const id of shifted) {
      const [key, kind] = id.split("|");
      const c = canvas.querySelector<HTMLElement>(`.graph-node-box[data-key="${CSS.escape(key ?? "")}"]`);
      const e = kind === "c" ? c : kind === "h" ? c?.parentElement : c?.parentElement?.parentElement;
      e?.classList.add("shift");
    }
  }

  function settle(): void {
    for (const k of canvas.querySelectorAll<HTMLElement>(".graph-kids[data-owner]")) k.classList.toggle("behind", k.dataset.support === "1");
    blurred = new Set([...canvas.querySelectorAll<HTMLElement>(".graph-kids[data-support='1']")].map((k) => k.dataset.owner ?? ""));
    const now = shiftTargets();
    for (const e of canvas.querySelectorAll(".shift")) e.classList.remove("shift");
    for (const e of now.values()) e.classList.add("shift");
    shifted = new Set(now.keys());
    reveal();
    const end = performance.now() + Math.max(SHIFT_MS, motionMs);
    const follow = () => {
      drawLines();
      if (performance.now() < end) frame = requestAnimationFrame(follow);
    };
    follow();
  }
  let scale = Number(readPref("fm.graph.zoom")) || 1;

  function zoomBy(delta: number): void {
    scale = Math.min(2, Math.max(0.4, Math.round((scale + delta) * 10) / 10));
    stage.style.zoom = String(scale);
    savePref("fm.graph.zoom", String(scale));
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(drawLines);
  }

  const kidsShown = (n: TreeNode): TreeNode[] => {
    if (!state.expanded.has(keyOf(n.relParts))) return [];
    return n.nodeType?.base === "NODE" ? nodesOf(n.children) : (n.children ?? []).filter((c) => !c.file && !isSupport(c));
  };

  // Outside the node structure every folder unfolds as cards, and its files share one box beside them.
  function filesBox(n: TreeNode, folders: number): HTMLElement[] {
    if (!state.expanded.has(keyOf(n.relParts))) return [];
    // A node's loose files show beside its sub-nodes; an open node with neither says it is empty instead of showing nothing.
    if (n.nodeType?.base === "NODE") {
      const own = (n.children ?? []).filter((c) => c.file);
      return !loading(n) && (own.length > 0 || folders === 0) ? [fileList({ ...n, children: own })] : [];
    }
    const files = (n.children ?? []).filter((c) => c.file);
    return files.length > 0 || folders === 0 ? [fileList({ ...n, children: files })] : [];
  }

  // An open folder whose contents are still on their way keeps a box in their place, under the same key, so nothing blinks out and back.
  const loading = (n: TreeNode) => n.children === null && n.hasChildren && state.expanded.has(keyOf(n.relParts));
  const loadingBox = (n: TreeNode): HTMLElement => el("ul", { class: "graph-files-box loading", "data-key": `${keyOf(n.relParts)}#files`, "aria-busy": "true" }, el("li", { class: "graph-file-empty" }, "Loading…"));

  // Files a batch just delivered, lit as if selected while the replay shows them.
  let arrived = new Set<string>();

  function fileList(n: TreeNode): HTMLElement {
    const entries = sortedFiles(n);
    const key = keyOf(n.relParts);
    const view = boxView(key);
    const items = entries.map((c) => {
      const fileKey = keyOf(c.relParts);
      const row = el("li", { class: arrived.has(fileKey) || state.selectedKeys.includes(fileKey) ? "graph-file selected" : "graph-file", title: c.name, "data-key": fileKey }, fileArt(c, view, folderIcon(c, false)), el("span", { class: "graph-file-text" }, (view === "grid" && c.file ? keepExtensionInSight(el("span", { class: "graph-file-name" }, c.name), c.name) : el("span", { class: "graph-file-name" }, c.name)), ...fileDetail(c, view)));
      // Ctrl adds or removes one file, Shift takes the run from the last pick, as in the other views.
      row.addEventListener("click", (ev) => {
        if (ev.ctrlKey || ev.shiftKey) clickSelect(c.relParts, entries.map((e) => keyOf(e.relParts)), { ctrl: ev.ctrlKey, shift: ev.shiftKey });
        else select(c.relParts);
      });
      row.addEventListener("dblclick", () => (!c.file && inSystem(c.relParts) ? void enterSystem(c.relParts) : void openFolder(c)));
      row.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, c, root));
      return row;
    });
    const up = inSystem(n.relParts) && n.relParts.length > 1 ? [upRow(n.relParts)] : [];
    const box = el("ul", { class: `graph-files-box view-${view}`, "data-key": `${key}#files`, "aria-label": `Contents of ${n.name}` }, ...up, ...(items.length > 0 ? items : [el("li", { class: "graph-file-empty" }, "Empty")]));
    fitWidth(box, view, entries.length);
    keepSize(box, key);
    bandSelect(box);
    boxMenu(box, n);
    return box;
  }

  // The selected card and every folder above it light up, so the route from the root reads at a glance.
  const onPath = (key: string) => state.selectedKey !== null && (state.selectedKey === key || state.selectedKey.startsWith(`${key}/`));

  function card(n: TreeNode): HTMLElement {
    const key = keyOf(n.relParts);
    const open = state.expanded.has(key);
    const item = el("div", { class: "graph-node-box", role: "treeitem", id: `graph-${key}`, "data-key": key, "aria-selected": String(state.selectedKey === key) }, folderIcon(n, open), el("span", { class: "graph-name" }, n.name));
    if (n.hasChildren) item.setAttribute("aria-expanded", String(open));
    item.classList.toggle("selected", state.selectedKey === key);
    item.classList.toggle("on-path", onPath(key));
    // Every redraw makes new elements; the shared phase keeps the breathing glow from restarting on each one.
    item.style.animationDelay = breathPhase();
    item.classList.toggle("ghost", n.ghost);
    if (n.status) item.dataset.status = n.status;
    // In the graph one click both selects a card and unfolds it, since browsing is the point of the view.
    item.addEventListener("click", () => {
      // Clicking an open card that is not selected only moves to it; it closes from a click once selected.
      const wasSelected = state.selectedKey === key;
      if (isSupport(n) && !state.expanded.has(key)) {
        beforeOverlay = state.selectedKey;
        lastOpened = key;
      }
      select(n.relParts);
      // With a support folder open, the parent's click brings its sub-nodes back instead of folding it.
      const open = (n.children ?? []).filter((c) => isSupport(c) && state.expanded.has(keyOf(c.relParts)));
      if (open.length === 0 && !wasSelected && state.expanded.has(key)) return;
      if (open.length > 0) {
        for (const c of open) state.expanded.delete(keyOf(c.relParts));
        notify();
      } else if (!n.hasChildren) {
        // A folder with nothing inside still opens, to its Empty box.
        if (state.expanded.has(key)) state.expanded.delete(key);
        else state.expanded.add(key);
        notify();
      } else {
        // Only one support folder opens at a time, so a new one never hides behind the last.
        const owner = isSupport(n) ? findNode(n.relParts.slice(0, -1)) : null;
        for (const c of owner?.children ?? []) if (c !== n && isSupport(c)) state.expanded.delete(keyOf(c.relParts));
        fold(n);
      }
    });
    item.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, n, root));
    return item;
  }

  // Records, screenshots, resources and reviews hang under their node instead of beside it, so they do not read as sub-nodes.
  function head(n: TreeNode): HTMLElement {
    const open = state.expanded.has(keyOf(n.relParts)) ? (n.children ?? []) : [];
    const under = open.filter((c) => !c.file && !isTopNote(c) && UNDER.has(c.nodeType?.base ?? ""));
    const above = open.filter(isTopNote);
    for (const u of [...under, ...above]) parents.set(keyOf(u.relParts), n);
    return el("div", { class: "graph-head" }, ...(above.length > 0 ? [el("div", { class: "graph-above", role: "group" }, ...above.map(card))] : []), card(n), ...(under.length > 0 ? [el("div", { class: "graph-under", role: "group" }, ...under.map(card))] : []));
  }

  function branch(n: TreeNode, parent: TreeNode | null): HTMLElement {
    parents.set(keyOf(n.relParts), parent);
    const kids = kidsShown(n);
    // An open support folder shows its contents where the sub-nodes stand, with them blurred behind it.
    const support = state.expanded.has(keyOf(n.relParts)) ? (n.children ?? []).find((c) => isSupport(c) && state.expanded.has(keyOf(c.relParts))) : undefined;
    if (support) {
      // Its folders stand as cards that unfold like nodes; loose files share one box.
      const inside = support.children ?? [];
      const folders = inside.filter((c) => !c.file);
      const files = inside.filter((c) => c.file);
      const items = loading(support) ? [loadingBox(support)] : [...folders.map((f) => branch(f, support)), ...(files.length > 0 || folders.length === 0 ? [fileList({ ...support, children: files })] : [])];
      const overlay = el("div", { class: "graph-support", "data-for": keyOf(support.relParts) }, ...items);
      // It spans every sub-node row, so a long list makes the column grow instead of covering what is below.
      overlay.style.gridRow = `1 / span ${Math.max(kids.length, 1)}`;
      return el("div", { class: `graph-branch${onPath(keyOf(n.relParts)) ? " open-path" : ""}${state.selectedKey === keyOf(n.relParts) ? " selected-branch" : ""}` }, head(n), el("div", { class: `graph-kids${blurred.has(keyOf(n.relParts)) ? " behind" : ""}`, role: "group", "data-owner": keyOf(n.relParts), "data-support": "1" }, ...kids.map((k, i) => {
        const b = branch(k, n);
        b.style.gridArea = `${i + 1} / 1`;
        return b;
      }), overlay));
    }
    const items = loading(n) ? [loadingBox(n)] : [...kids.map((k) => branch(k, n)), ...filesBox(n, kids.length)];
    return el("div", { class: `graph-branch${onPath(keyOf(n.relParts)) ? " open-path" : ""}${state.selectedKey === keyOf(n.relParts) ? " selected-branch" : ""}` }, head(n), ...(items.length > 0 ? [el("div", { class: `graph-kids${blurred.has(keyOf(n.relParts)) ? " behind" : ""}`, role: "group", "data-owner": keyOf(n.relParts) }, ...items)] : []));
  }

  // Curves are drawn after layout, from each card's right edge to its children's left edges.
  function drawLines(): void {
    const box = canvas.getBoundingClientRect();
    // Rects come in screen pixels; the SVG sits inside the zoomed stage, so they are scaled back to its units.
    const unit = box.width / (canvas.offsetWidth || 1);
    lines.setAttribute("width", String(canvas.scrollWidth));
    lines.setAttribute("height", String(canvas.scrollHeight));
    const paths: string[] = [];
    const lit: string[] = [];
    // Lines of sub-nodes standing behind an open support folder fade with their cards.
    const faded: string[] = [];
    const behind = (e: Element) => e.closest(".graph-kids.behind > .graph-branch") !== null;
    // Lines inside the selected card's branch take a soft accent, so its group reads as one.
    const group: string[] = [];
    const rest = (e: Element) => (e.closest(".graph-branch.selected-branch") ? group : paths);
    for (const b of canvas.querySelectorAll<HTMLElement>(".graph-branch")) {
      const fromCard = b.querySelector(":scope > .graph-head > .graph-node-box");
      const from = fromCard?.getBoundingClientRect();
      const kids = b.querySelector(":scope > .graph-kids");
      if (!from || !kids) continue;
      const x1 = (from.right - box.left) / unit;
      // The split runs past the whole head column, so the bracket clears the cards stacked under the node.
      const headRight = b.querySelector(":scope > .graph-head")?.getBoundingClientRect().right ?? from.right;
      const tx = (headRight - box.left) / unit + 10;
      // One split line per fan, taken from the column's edge, so a shifted lit child does not bend its own trunk.
      const split = (tx + (kids.getBoundingClientRect().left - box.left) / unit) / 2;
      const y1 = (from.top + from.height / 2 - box.top) / unit;
      const fromLit = fromCard?.classList.contains("on-path") ?? false;
      const overlay = kids.querySelector<HTMLElement>(":scope > .graph-support");
      if (overlay) {
        const owner = b.querySelector<HTMLElement>(`:scope > .graph-head [data-key="${CSS.escape(overlay.dataset.for ?? "")}"]`);
        // The sub-nodes behind keep their own lines, faded like them.
        for (const k of kids.children) {
          const target = k.querySelector(":scope > .graph-head > .graph-node-box");
          if (k === overlay || !target) continue;
          const r = target.getBoundingClientRect();
          const x2 = (r.left - box.left) / unit;
          faded.push(bracket(x1, y1, split, x2, (r.top + r.height / 2 - box.top) / unit));
        }
        const o = owner?.getBoundingClientRect();
        if (!owner || !o) continue;
        const ox = (o.right - box.left) / unit;
        const oy = (o.top + o.height / 2 - box.top) / unit;
        for (const k of overlay.children) {
          const target = k.querySelector(":scope > .graph-head > .graph-node-box");
          const to = (target ?? k).getBoundingClientRect();
          const litHere = owner.classList.contains("on-path") && (target ? target.classList.contains("on-path") : true);
          const x2 = (to.left - box.left) / unit;
          (behind(b) ? faded : litHere ? lit : rest(b)).push(bracket(ox, oy, split + 14, x2, (to.top + to.height / 2 - box.top) / unit));
        }
        continue;
      }
      for (const k of kids.children) {
        const target = k.classList.contains("graph-files-box") ? null : k.querySelector(":scope > .graph-head > .graph-node-box");
        const to = (target ?? k).getBoundingClientRect();
        if (!to) continue;
        const x2 = (to.left - box.left) / unit;
        const y2 = (to.top + to.height / 2 - box.top) / unit;
        const mid = split;
        (behind(b) ? faded : fromLit && (target ? target.classList.contains("on-path") : true) ? lit : rest(b)).push(bracket(x1, y1, mid, x2, y2));
      }
    }
    for (const h of canvas.querySelectorAll<HTMLElement>(".graph-head")) {
      const owner = h.querySelector(":scope > .graph-node-box");
      const rect = owner?.getBoundingClientRect();
      if (!owner || !rect) continue;
      const x = (rect.left + 14 - box.left) / unit;
      const y = (rect.bottom - box.top) / unit;
      for (const u of h.querySelectorAll<HTMLElement>(":scope > .graph-under > .graph-node-box")) {
        const r = u.getBoundingClientRect();
        const uy = (r.top + r.height / 2 - box.top) / unit;
        const ux = (r.left - box.left) / unit;
        (behind(h) ? faded : owner.classList.contains("on-path") && u.classList.contains("on-path") ? lit : rest(h)).push(`M${x},${y} V${uy - 6} Q${x},${uy} ${x + 6},${uy} H${ux}`);
      }
      const top = (rect.top - box.top) / unit;
      for (const u of h.querySelectorAll<HTMLElement>(":scope > .graph-above > .graph-node-box")) {
        const r = u.getBoundingClientRect();
        const uy = (r.top + r.height / 2 - box.top) / unit;
        const ux = (r.left - box.left) / unit;
        (behind(h) ? faded : owner.classList.contains("on-path") && u.classList.contains("on-path") ? lit : rest(h)).push(`M${x},${top} V${uy + 6} Q${x},${uy} ${x + 6},${uy} H${ux}`);
      }
    }
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("d", paths.join(" "));
    path.classList.add("graph-path-rest");
    const glow = document.createElementNS(SVG, "path");
    glow.setAttribute("d", lit.join(" "));
    glow.classList.add("graph-path-lit");
    glow.style.animationDelay = breathPhase();
    const dim = document.createElementNS(SVG, "path");
    dim.setAttribute("d", faded.join(" "));
    dim.classList.add("graph-path-faded");
    const soft = document.createElementNS(SVG, "path");
    soft.setAttribute("d", group.join(" "));
    soft.classList.add("graph-path-group");
    lines.replaceChildren(dim, path, soft, glow);
  }

  function step(dir: "up" | "down" | "left" | "right"): void {
    const key = state.selectedKey;
    if (key && inSystem(key.split("/")) && state.expanded.has(key.split("/")[0] ?? "")) return systemStep(key, dir);
    const tops = nodesOf(state.tree?.nodes);
    const n = key && parents.has(key) ? findShown(key) : null;
    if (!n) {
      if (tops[0]) select(tops[0].relParts);
      return;
    }
    const parent = parents.get(key ?? "") ?? null;
    const siblings = parent ? kidsShown(parent) : tops;
    const i = siblings.indexOf(n);
    if (dir === "left" && parent) select(parent.relParts);
    else if (dir === "right") {
      const first = kidsShown(n)[0];
      if (first) select(first.relParts);
      else if (n.hasChildren) void toggleExpand(n);
    } else if (dir === "up" && i > 0) select(siblings[i - 1]!.relParts);
    else if (dir === "down" && i < siblings.length - 1) select(siblings[i + 1]!.relParts);
  }

  function findShown(key: string): TreeNode | null {
    const walk = (list: TreeNode[]): TreeNode | null => {
      for (const n of list) {
        if (keyOf(n.relParts) === key) return n;
        const hit = walk(kidsShown(n));
        if (hit) return hit;
      }
      return null;
    };
    return walk(nodesOf(state.tree?.nodes));
  }

  function onKey(ev: KeyboardEvent): void {
    if (ev.key === " ") {
      spaceHeld = true;
      root.classList.add("panning");
      ev.preventDefault();
      return;
    }
    if (ev.ctrlKey && (ev.key === "+" || ev.key === "=" || ev.key === "-")) {
      zoomBy(ev.key === "-" ? -0.1 : 0.1);
      ev.preventDefault();
      return;
    }
    if (ev.ctrlKey || ev.altKey || ev.metaKey) return;
    const moves: Record<string, "up" | "down" | "left" | "right"> = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
    const move = moves[ev.key];
    if (move) step(move);
    else if (ev.key === "Enter" && state.selectedKey) {
      const n = findShown(state.selectedKey);
      if (n) fold(n);
    } else return;
    ev.preventDefault();
  }

  root.addEventListener("wheel", (ev) => {
    if (!ev.ctrlKey) return;
    ev.preventDefault();
    zoomBy(ev.deltaY < 0 ? 0.1 : -0.1);
  }, { passive: false });

  // The native folders shown, Downloads first, each with the root marker of its paths.
  const systemEntries = () => VIRTUAL_ENTRIES.flatMap((e) => {
    const root = virtualRoot(e.id);
    return root !== null && isVirtualShown(e.id) ? [{ entry: e, root }] : [];
  });
  // Only one native folder's box stands open over the branches at a time.
  const openSystem = (): string | null => systemEntries().find(({ root }) => state.expanded.has(root))?.root ?? null;

  // Downloads hangs under the root like a support folder, the other native folders below it; files a batch sends out start their trip from Downloads.
  function systemCards(): HTMLElement[] {
    const shown = systemEntries();
    if (shown.length === 0) return [];
    // A native folder opens like a support folder: its files show to the right, over the top nodes, which blur behind them; a double click opens it as a list.
    const cards = shown.map(({ entry, root }) => {
      const open = state.expanded.has(root);
      const item = el("div", { class: "graph-node-box graph-entry", role: "treeitem", "data-entry": entry.id, "data-key": root, "aria-expanded": String(open) }, guiIcon(entry.icon), el("span", { class: "graph-name" }, systemName([root])));
      item.addEventListener("click", () => {
        if (open) state.expanded.delete(root);
        else {
          if (openSystem() === null) beforeOverlay = state.selectedKey;
          for (const other of shown) state.expanded.delete(other.root);
          lastOpened = root;
          state.expanded.add(root);
          readSystem(root);
        }
        notify();
      });
      item.addEventListener("dblclick", () => void openVirtual(entry.id));
      return item;
    });
    return [el("div", { class: "graph-under", role: "group" }, ...cards)];
  }

  // A native folder's box browses its subfolders in place; the folder shown is kept while the graph lives.
  let systemAt: string[] = [DOWNLOADS_ROOT];
  const systemTop = (root: string): TreeNode => ({ name: systemName([root]), relParts: [root], children: state.system[root] ?? [] }) as unknown as TreeNode;
  const systemShown = (): TreeNode | null => (systemAt.length > 1 ? findNode(systemAt) : systemTop(systemAt[0] ?? DOWNLOADS_ROOT));

  async function enterSystem(parts: string[]): Promise<void> {
    const n = parts.length > 1 ? findNode(parts) : null;
    try {
      if (n && n.children === null) n.children = await listChildren(parts);
    } catch (e) {
      reportError("warning", e, systemName(parts));
      return;
    }
    systemAt = n || parts.length === 1 ? parts : parts.slice(0, 1);
    const first = sortedFiles(systemShown() ?? ({ children: [] } as unknown as TreeNode))[0];
    if (first) select(first.relParts);
    else notify();
  }

  function upRow(parts: string[]): HTMLElement {
    const row = el("li", { class: "graph-file graph-file-up", title: "Up one folder" }, guiIcon("up"), el("span", { class: "graph-file-text" }, el("span", { class: "graph-file-name" }, "..")));
    row.addEventListener("dblclick", () => void enterSystem(parts.slice(0, -1)));
    return row;
  }

  function systemStep(key: string, dir: "up" | "down" | "left" | "right"): void {
    const entries = sortedFiles(systemShown() ?? ({ children: [] } as unknown as TreeNode));
    const i = entries.findIndex((e) => keyOf(e.relParts) === key);
    const n = entries[i];
    if (dir === "up" && i > 0) select(entries[i - 1]!.relParts);
    else if (dir === "down" && i < entries.length - 1) select(entries[i + 1]!.relParts);
    else if (dir === "right" && n && !n.file) void enterSystem(n.relParts);
    else if (dir === "left") {
      if (systemAt.length > 1) void enterSystem(systemAt.slice(0, -1));
      else select(systemAt.slice(0, 1));
    } else if (i < 0 && entries[0]) select(entries[0].relParts);
  }

  // A native folder is read when its box opens, and also when the graph comes back with it already open; until then the box shows it is loading.
  const systemRead = new Set<string>();
  const systemLoaded = new Set<string>();
  function readSystem(root: string): void {
    systemRead.add(root);
    void listChildren([root]).then((nodes) => {
      state.system[root] = keepChildren(root, nodes);
      systemLoaded.add(root);
      notify();
    }, (e: unknown) => reportError("warning", e, systemName([root])));
  }

  function rootKids(tops: TreeNode[]): HTMLElement {
    const branches = tops.map((n) => branch(n, null));
    const shown = openSystem();
    if (shown === null) return el("div", { class: "graph-kids", role: "group" }, ...branches);
    branches.forEach((b, i) => (b.style.gridArea = `${i + 1} / 1`));
    if (systemAt[0] !== shown || (systemAt.length > 1 && !findNode(systemAt))) systemAt = [shown];
    const folder = systemShown() ?? systemTop(shown);
    if (!systemRead.has(shown)) readSystem(shown);
    const overlay = el("div", { class: "graph-support", "data-for": shown }, (state.system[shown]?.length ?? 0) > 0 || systemLoaded.has(shown) ? fileList(folder) : loadingBox(folder));
    overlay.style.gridRow = `1 / span ${Math.max(tops.length, 1)}`;
    return el("div", { class: `graph-kids${blurred.has("") ? " behind" : ""}`, role: "group", "data-owner": "", "data-support": "1" }, ...branches, overlay);
  }

  // A file dropped in a node lands in its review folder; a worknode keeps it.
  const landing = (n: TreeNode): TreeNode => (n.nodeType?.base === "NODE" ? ((n.children ?? []).find((c) => c.nodeType?.base === "REV") ?? n) : n);

  // Packets leave one after another at the same pace, so on a shared line they keep their distance instead of colliding.
  const GAP_MS = 252;
  const cardOf = (k: string) => canvas.querySelector<HTMLElement>(`.graph-node-box[data-key="${CSS.escape(k)}"]`);
  const prefixes = (parts: string[]) => parts.map((_, k) => keyOf(parts.slice(0, k + 1)));

  // Sends every route in order, top branches first; only the last packet steers the view, so they never fight over the scroll.
  async function wave(routes: HTMLElement[][], news: FilesMoved): Promise<void> {
    const trips = routes.map((cards, k) =>
      news.wait(k * GAP_MS).then(() => sendPacket(stage, canvas, { cards }, news.track, k === routes.length - 1 ? (p) => p.scrollIntoView({ block: "nearest", inline: "nearest" }) : () => undefined)),
    );
    await Promise.all(trips);
  }

  // The trips run through folders that may be folded, so they open first; the packets all travel, then every destination opens at once.
  async function showMoves(news: FilesMoved): Promise<void> {
    const virtual = state.virtual;
    const selected = state.selectedKey;
    // A support folder left open, or a lit path, would cover the trips; the graph starts from a clean map.
    for (const key of [...state.expanded]) {
      const n = findNode(key.split("/"));
      if (n && isSupport(n)) state.expanded.delete(key);
    }
    state.selectedKey = null;
    state.selectedKeys = [];
    for (const parts of [...news.targets.map((t) => t.parts), ...news.chained.map((c) => c.to)]) for (const k of prefixes(parts)) state.expanded.add(k);
    try {
      await hydrate();
    } catch (e) {
      reportError("warning", e, "Map path");
    }
    notify();
    await news.wait(SHIFT_MS + 200);
    const start = canvas.querySelector<HTMLElement>('[data-entry="downloads"]');
    if (!start) return;
    start.scrollIntoView({ block: "center", inline: "center", behavior: "smooth" });
    const order = (a: string[], b: string[]) => keyOf(a).localeCompare(keyOf(b), undefined, { numeric: true });
    const targets = [...news.targets].sort((a, b) => order(a.parts, b.parts));
    const shown: { end: TreeNode; names: string[] }[] = [];
    const first: HTMLElement[][] = [];
    const onward: { from: TreeNode; to: string[]; names: string[] }[] = [];
    for (const { parts, names } of targets) {
      const node = findNode(parts);
      const end = node ? landing(node) : null;
      first.push([start, cardOf(""), ...[...prefixes(parts), ...(end && end !== node ? [keyOf(end.relParts)] : [])].map(cardOf)].filter((c): c is HTMLElement => c !== null));
      if (!end) continue;
      const passed = news.chained.filter((c) => keyOf(c.from) === keyOf(end.relParts));
      const stays = names.filter((n) => !passed.some((c) => c.name === n));
      if (stays.length > 0) shown.push({ end, names: stays });
      const groups = new Map<string, { from: TreeNode; to: string[]; names: string[] }>();
      for (const c of passed) {
        const g = groups.get(keyOf(c.to)) ?? { from: end, to: c.to, names: [] };
        g.names.push(c.name);
        groups.set(keyOf(c.to), g);
      }
      onward.push(...groups.values());
    }
    await wave(first, news);
    // A review folder that runs on its own sends its files on: a second wave leaves from it, up to the shared folder and down to the next node.
    const second = onward.sort((a, b) => order(a.to, b.to)).map(({ from, to, names }) => {
      let common = 0;
      while (common < from.relParts.length && common < to.length && from.relParts[common] === to[common]) common++;
      const node = findNode(to);
      const end = node ? landing(node) : null;
      if (end) shown.push({ end, names });
      const up = [...prefixes(from.relParts.slice(0, -1)).slice(Math.max(common - 1, 0)).reverse(), ...(common === 0 ? [""] : [])];
      return [keyOf(from.relParts), ...up, ...prefixes(to).slice(common), ...(end && end !== node ? [keyOf(end.relParts)] : [])].map(cardOf).filter((c): c is HTMLElement => c !== null);
    });
    if (second.length > 0) await wave(second, news);
    await revealAll(shown, news);
    // The trip borrowed the selection; the user's own place comes back.
    if (virtual) await openVirtual(virtual);
    else select(selected?.split("/") ?? null);
  }

  // Every destination opens together with its new files on top and lit, holds a moment, then closes.
  async function revealAll(shown: { end: TreeNode; names: string[] }[], news: FilesMoved): Promise<void> {
    if (shown.length === 0) return;
    const keys = shown.map(({ end }) => keyOf(end.relParts));
    for (const { end } of shown) {
      state.expanded.add(keyOf(end.relParts));
      // What the graph holds for it was read before the run, often empty, so it is read again.
      const fresh = findNode(end.relParts) ?? end;
      fresh.children = null;
      fresh.hasChildren = true;
    }
    try {
      await hydrate();
    } catch (e) {
      reportError("warning", e, "Map path");
    }
    const lit = new Set<string>();
    for (const { end, names } of shown) {
      const fresh = findNode(end.relParts) ?? end;
      const listed = fresh.children ?? [];
      // The run may still be filing them, so they are drawn where they are going; they lead the list whatever the sort, only for the show.
      const byName = new Map(listed.map((c) => [c.name, c]));
      const arriving = names.map((name) => byName.get(name) ?? ({ name, file: true, relParts: [...end.relParts, name], children: null, hasChildren: false, flags: [], ghost: false, nodeType: null } as unknown as TreeNode));
      fresh.children = [...arriving, ...listed.filter((c) => !names.includes(c.name))];
      for (const c of arriving) lit.add(keyOf(c.relParts));
    }
    arrived = lit;
    notify();
    await news.wait(SHIFT_MS + 900);
    arrived = new Set();
    for (const k of keys) state.expanded.delete(k);
    notify();
    await news.wait(SHIFT_MS);
  }
  // Every file that moved, here or in another program, travels the branches as a packet: up from where it left to the folder both paths share, then down
  // to where it landed, or down from the root when it came from outside. They run one after another, as they happened, seen or not.
  let trips: Promise<void> = Promise.resolve();
  const visibleChain = (parts: string[]): HTMLElement[] => {
    const chain: HTMLElement[] = [];
    for (const k of ["", ...prefixes(parts)]) {
      const c = cardOf(k);
      if (!c) break;
      chain.push(c);
    }
    return chain;
  };
  const travel = async (trip: Trip): Promise<void> => {
    await new Promise((r) => requestAnimationFrame(r));
    if (!root.isConnected) return;
    const down = visibleChain(trip.to);
    const up = trip.from === null ? [] : visibleChain(trip.from);
    let shared = 0;
    while (shared < up.length && shared < down.length && up[shared] === down[shared]) shared++;
    const start = Math.max(shared - 1, 0);
    await sendPacket(stage, canvas, { cards: down.slice(start), back: up.slice(start) }, (a) => a, () => undefined);
  };
  const onTrips = (ev: Event): void => {
    for (const trip of (ev as CustomEvent<Trip[]>).detail) trips = trips.then(() => travel(trip), () => travel(trip));
  };

  const onMoved = (ev: Event) => {
    const news = (ev as CustomEvent<FilesMoved>).detail;
    if (root.isConnected) news.handled = showMoves(news);
  };

  return {
    id: "graph",
    label: "Graph",
    mount(host) {
      root.addEventListener("keydown", onKey);
      window.addEventListener("fm:files-moved", onMoved);
      window.addEventListener("fm:file-trips", onTrips);
      window.addEventListener("pointermove", onDragMove);
      fresh = true;
      stage.style.zoom = String(scale);
      host.append(root);
    },
    render() {
      // Until the storage is read there is nothing to lay out, so the graph says it is loading instead of showing a lone root.
      if (!state.tree) {
        canvas.replaceChildren(el("div", { class: "graph-files-box loading", "aria-busy": "true" }, "Loading…"));
        return;
      }
      if (state.tree && restoredFor !== state.tree.root) {
        restoredFor = state.tree.root;
        void restore();
      }
      // A place reached from another view may sit under folders the graph never opened; they open so the path shows.
      const parts = state.selectedKey?.split("/") ?? [];
      const missing = parts.slice(0, -1).map((_, i) => parts.slice(0, i + 1).join("/")).filter((k) => !state.expanded.has(k));
      if (missing.length > 0) {
        for (const k of missing) state.expanded.add(k);
        hydrate().then(notify, (e: unknown) => reportError("warning", e, "Map path"));
      }
      if (!restoring && state.tree) savePref(`fm.graph.expanded:${state.tree.root}`, JSON.stringify([...state.expanded]));
      parents = new Map();
      const tops = nodesOf(state.tree?.nodes);
      const top = el("div", { class: "graph-node-box graph-root" }, el("span", { class: "graph-name" }, rootName(state.tree?.root) || "Storage"));
      // The selected card is held in place on screen, so the centred layout reflowing around it does not look like a jump.
      const sel = state.selectedKey;
      const at = (k: string | null) => (k === null ? undefined : canvas.querySelector(`[data-key="${CSS.escape(k)}"]`)?.getBoundingClientRect());
      const before = at(sel);
      const layout = snapshot(canvas);
      canvas.replaceChildren(el("div", { class: "graph-branch" }, el("div", { class: "graph-head" }, top, ...systemCards()), ...(tops.length > 0 || openSystem() !== null ? [rootKids(tops)] : [])));
      restoreShift();
      top.classList.toggle("on-path", state.selectedKey !== null && parents.has(state.selectedKey));
      if (state.selectedKey && parents.has(state.selectedKey)) {
        root.setAttribute("aria-activedescendant", `graph-${state.selectedKey}`);
        const after = at(sel);
        if (before && after) {
          for (const scroller of [root, root.parentElement]) {
            const now = at(sel);
            if (now) scroller?.scrollBy(now.left - before.left, now.top - before.top);
          }
        } else canvas.querySelector(`[data-key="${CSS.escape(state.selectedKey)}"]`)?.scrollIntoView({ block: "nearest", inline: "nearest" });
      } else root.removeAttribute("aria-activedescendant");
      top.dataset.key = "";
      motionMs = play(canvas, layout);
      // The padding leaves empty room on every side, so the first frame centres what matters.
      if (fresh && state.tree) {
        fresh = false;
        const focusCard = state.selectedKey ? canvas.querySelector<HTMLElement>(`[data-key="${CSS.escape(state.selectedKey)}"]`) : null;
        (focusCard ?? top).scrollIntoView({ block: "center", inline: "center" });
      }
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(settle);
    },
    focus() {
      root.focus({ preventScroll: true });
    },
    destroy() {
      window.removeEventListener("fm:files-moved", onMoved);
      window.removeEventListener("fm:file-trips", onTrips);
      window.removeEventListener("pointermove", onDragMove);
      cancelAnimationFrame(frame);
      root.remove();
    },
  };
}
