// Map view: the NODE and WKB folders as a mind map that unfolds to the right; other types stay out so the structure reads at a glance.
import { el } from "../../dom";
import { rootName } from "../../organisms/bars/path-input";
import { nodeContextMenu } from "../../organisms/menus/context-menu";
import { folderIcon } from "../../atoms/icons";
import { toggleExpand } from "../../../hooks/ops";
import { openFolder } from "../../../hooks/path-actions";
import { readPref, savePref } from "../../../services/store/prefs";
import { reportError } from "../../organisms/bars/status-line";
import { findNode, notify, keyOf, select, state } from "../../../services/store/state";
import type { TreeNode } from "../../../services/ipc/types";
import type { ViewMode } from "./view-mode";

const SVG = "http://www.w3.org/2000/svg";
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

export function createMapView(): ViewMode {
  const canvas = el("div", { class: "map-canvas" });
  const lines = document.createElementNS(SVG, "svg");
  lines.classList.add("map-lines");
  lines.setAttribute("aria-hidden", "true");
  const stage = el("div", { class: "map-stage" }, lines, canvas);
  const root = el("div", { class: "map", role: "tree", tabindex: "0", "aria-label": "Node map" }, stage);
  // Parent of each shown node, so the arrow keys can walk the map.
  let parents = new Map<string, TreeNode | null>();
  let frame = 0;
  // Cards on the lit path stand a little to the right; new cards start where the last render left them so the move animates.
  let shifted = new Set<string>();
  const SHIFT_MS = 320;

  // Branches whose sub-nodes stood blurred last render; they start there so the blur animates in and out.
  let blurred = new Set<string>();

  // Folding a node also folds its open support folder, so its sub-nodes come back when it opens again.
  function fold(n: TreeNode): void {
    if (state.expanded.has(keyOf(n.relParts))) for (const c of n.children ?? []) if (isSupport(c)) state.expanded.delete(keyOf(c.relParts));
    void toggleExpand(n);
  }

  // What was unfolded is kept per storage, so the map opens as it was left.
  let restoredFor: string | null = null;
  let restoring = false;

  async function restore(): Promise<void> {
    const root = state.tree?.root ?? "";
    let keys: string[] = [];
    try {
      const raw: unknown = JSON.parse(readPref(`fm.map.expanded:${root}`) ?? "[]");
      if (Array.isArray(raw)) keys = raw.filter((k): k is string => typeof k === "string");
    } catch (e) {
      reportError("warning", e, "Map layout");
    }
    restoring = true;
    try {
      for (const key of keys.sort((a, b) => a.split("/").length - b.split("/").length)) {
        const n = findNode(key.split("/"));
        if (n && n.hasChildren && !state.expanded.has(key)) await toggleExpand(n);
      }
    } finally {
      restoring = false;
      notify();
    }
  }

  // Whichever of the map or its host actually scrolls.
  const scroller = (): HTMLElement => (root.scrollWidth > root.clientWidth + 1 || root.scrollHeight > root.clientHeight + 1 ? root : (root.parentElement ?? root));

  // As in Columns, a new selection or fold brings what it opened in from the right; what is left behind matters less.
  let revealed = "";
  function reveal(): void {
    const mark = `${state.selectedKey}|${state.expanded.size}`;
    if (mark === revealed || state.selectedKey === null) return;
    revealed = mark;
    const card = canvas.querySelector<HTMLElement>(`[data-key="${CSS.escape(state.selectedKey)}"]`);
    const branchBox = card?.closest(".map-branch")?.getBoundingClientRect();
    const cardBox = card?.getBoundingClientRect();
    if (!branchBox || !cardBox) return;
    const view = scroller().getBoundingClientRect();
    const over = branchBox.right + 24 - view.right;
    if (over > 0) scroller().scrollBy({ left: Math.min(over, cardBox.left - view.left - 24), behavior: "smooth" });
  }

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

  function settle(): void {
    for (const k of canvas.querySelectorAll<HTMLElement>(".map-kids[data-owner]")) k.classList.toggle("behind", k.dataset.support === "1");
    blurred = new Set([...canvas.querySelectorAll<HTMLElement>(".map-kids[data-support='1']")].map((k) => k.dataset.owner ?? ""));
    // The root has nothing beside it to stand out from, so it never shifts.
    for (const c of canvas.querySelectorAll<HTMLElement>(".map-card:not(.map-root)")) c.classList.toggle("shift", c.classList.contains("on-path"));
    shifted = new Set([...canvas.querySelectorAll<HTMLElement>(".map-card.on-path")].map((c) => c.dataset.key ?? ""));
    reveal();
    const end = performance.now() + SHIFT_MS;
    const follow = () => {
      drawLines();
      if (performance.now() < end) frame = requestAnimationFrame(follow);
    };
    follow();
  }
  let scale = Number(readPref("fm.map.zoom")) || 1;

  function zoomBy(delta: number): void {
    scale = Math.min(2, Math.max(0.4, Math.round((scale + delta) * 10) / 10));
    stage.style.zoom = String(scale);
    savePref("fm.map.zoom", String(scale));
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(drawLines);
  }

  const kidsShown = (n: TreeNode): TreeNode[] => {
    if (!state.expanded.has(keyOf(n.relParts))) return [];
    return n.nodeType?.base === "NODE" ? nodesOf(n.children) : (n.children ?? []).filter((c) => !c.file && !isSupport(c));
  };

  // Outside the node structure every folder unfolds as cards, and its files share one box beside them.
  function filesBox(n: TreeNode, folders: number): HTMLElement[] {
    if (n.nodeType?.base === "NODE" || !state.expanded.has(keyOf(n.relParts))) return [];
    const files = (n.children ?? []).filter((c) => c.file);
    return files.length > 0 || folders === 0 ? [fileList({ ...n, children: files })] : [];
  }

  function fileList(n: TreeNode): HTMLElement {
    const entries = n.children ?? [];
    const items = entries.map((c) => {
      const row = el("li", { class: "map-file", title: c.name }, folderIcon(c, false), el("span", { class: "map-file-name" }, c.name));
      row.addEventListener("click", () => select(c.relParts));
      row.addEventListener("dblclick", () => void openFolder(c));
      row.addEventListener("contextmenu", (ev) => nodeContextMenu(ev, c, root));
      return row;
    });
    return el("ul", { class: "map-files", "aria-label": `Contents of ${n.name}` }, ...(items.length > 0 ? items : [el("li", { class: "map-file-empty" }, "Empty")]));
  }

  // The selected card and every folder above it light up, so the route from the root reads at a glance.
  const onPath = (key: string) => state.selectedKey !== null && (state.selectedKey === key || state.selectedKey.startsWith(`${key}/`));

  function card(n: TreeNode): HTMLElement {
    const key = keyOf(n.relParts);
    const open = state.expanded.has(key);
    const item = el("div", { class: "map-card", role: "treeitem", id: `map-${key}`, "data-key": key, "aria-selected": String(state.selectedKey === key) }, folderIcon(n, open), el("span", { class: "map-name" }, n.name));
    if (n.hasChildren) item.setAttribute("aria-expanded", String(open));
    item.classList.toggle("selected", state.selectedKey === key);
    item.classList.toggle("on-path", onPath(key));
    item.classList.toggle("shift", shifted.has(key));
    item.classList.toggle("ghost", n.ghost);
    if (n.status) item.dataset.status = n.status;
    // In the map one click both selects a card and unfolds it, since browsing is the point of the view.
    item.addEventListener("click", () => {
      select(n.relParts);
      // With a support folder open, the parent's click brings its sub-nodes back instead of folding it.
      const open = (n.children ?? []).filter((c) => isSupport(c) && state.expanded.has(keyOf(c.relParts)));
      if (open.length > 0) {
        for (const c of open) state.expanded.delete(keyOf(c.relParts));
        notify();
      } else if (n.hasChildren) {
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
    return el("div", { class: "map-head" }, ...(above.length > 0 ? [el("div", { class: "map-above", role: "group" }, ...above.map(card))] : []), card(n), ...(under.length > 0 ? [el("div", { class: "map-under", role: "group" }, ...under.map(card))] : []));
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
      const items = [...folders.map((f) => branch(f, support)), ...(files.length > 0 || folders.length === 0 ? [fileList({ ...support, children: files })] : [])];
      const overlay = el("div", { class: "map-support", "data-for": keyOf(support.relParts) }, ...items);
      return el("div", { class: "map-branch" }, head(n), el("div", { class: `map-kids${blurred.has(keyOf(n.relParts)) ? " behind" : ""}${kids.length > 0 ? "" : " alone"}`, role: "group", "data-owner": keyOf(n.relParts), "data-support": "1" }, ...kids.map((k) => branch(k, n)), overlay));
    }
    const items = [...kids.map((k) => branch(k, n)), ...filesBox(n, kids.length)];
    return el("div", { class: "map-branch" }, head(n), ...(items.length > 0 ? [el("div", { class: `map-kids${blurred.has(keyOf(n.relParts)) ? " behind" : ""}`, role: "group", "data-owner": keyOf(n.relParts) }, ...items)] : []));
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
    const behind = (e: Element) => e.closest(".map-kids.behind > .map-branch") !== null;
    for (const b of canvas.querySelectorAll<HTMLElement>(".map-branch")) {
      const fromCard = b.querySelector(":scope > .map-head > .map-card");
      const from = fromCard?.getBoundingClientRect();
      const kids = b.querySelector(":scope > .map-kids");
      if (!from || !kids) continue;
      const x1 = (from.right - box.left) / unit;
      // The split runs past the whole head column, so the bracket clears the cards stacked under the node.
      const headRight = b.querySelector(":scope > .map-head")?.getBoundingClientRect().right ?? from.right;
      const tx = (headRight - box.left) / unit + 10;
      const y1 = (from.top + from.height / 2 - box.top) / unit;
      const fromLit = fromCard?.classList.contains("on-path") ?? false;
      const overlay = kids.querySelector<HTMLElement>(":scope > .map-support");
      if (overlay) {
        const owner = b.querySelector<HTMLElement>(`:scope > .map-head [data-key="${CSS.escape(overlay.dataset.for ?? "")}"]`);
        // The sub-nodes behind keep their own lines, faded like them.
        for (const k of kids.children) {
          const target = k.querySelector(":scope > .map-head > .map-card");
          if (k === overlay || !target) continue;
          const r = target.getBoundingClientRect();
          const x2 = (r.left - box.left) / unit;
          faded.push(bracket(x1, y1, (tx + x2) / 2, x2, (r.top + r.height / 2 - box.top) / unit));
        }
        const o = owner?.getBoundingClientRect();
        if (!owner || !o) continue;
        const ox = (o.right - box.left) / unit;
        const oy = (o.top + o.height / 2 - box.top) / unit;
        for (const k of overlay.children) {
          const target = k.querySelector(":scope > .map-head > .map-card");
          const to = (target ?? k).getBoundingClientRect();
          const litHere = owner.classList.contains("on-path") && (target ? target.classList.contains("on-path") : true);
          const x2 = (to.left - box.left) / unit;
          (behind(b) ? faded : litHere ? lit : paths).push(bracket(ox, oy, (tx + x2) / 2, x2, (to.top + to.height / 2 - box.top) / unit));
        }
        continue;
      }
      for (const k of kids.children) {
        const target = k.classList.contains("map-files") ? null : k.querySelector(":scope > .map-head > .map-card");
        const to = (target ?? k).getBoundingClientRect();
        if (!to) continue;
        const x2 = (to.left - box.left) / unit;
        const y2 = (to.top + to.height / 2 - box.top) / unit;
        const mid = (tx + x2) / 2;
        (behind(b) ? faded : fromLit && (target ? target.classList.contains("on-path") : true) ? lit : paths).push(bracket(x1, y1, mid, x2, y2));
      }
    }
    for (const h of canvas.querySelectorAll<HTMLElement>(".map-head")) {
      const owner = h.querySelector(":scope > .map-card");
      const rect = owner?.getBoundingClientRect();
      if (!owner || !rect) continue;
      const x = (rect.left + 14 - box.left) / unit;
      const y = (rect.bottom - box.top) / unit;
      for (const u of h.querySelectorAll<HTMLElement>(":scope > .map-under > .map-card")) {
        const r = u.getBoundingClientRect();
        const uy = (r.top + r.height / 2 - box.top) / unit;
        const ux = (r.left - box.left) / unit;
        (behind(h) ? faded : owner.classList.contains("on-path") && u.classList.contains("on-path") ? lit : paths).push(`M${x},${y} V${uy - 6} Q${x},${uy} ${x + 6},${uy} H${ux}`);
      }
      const top = (rect.top - box.top) / unit;
      for (const u of h.querySelectorAll<HTMLElement>(":scope > .map-above > .map-card")) {
        const r = u.getBoundingClientRect();
        const uy = (r.top + r.height / 2 - box.top) / unit;
        const ux = (r.left - box.left) / unit;
        (behind(h) ? faded : owner.classList.contains("on-path") && u.classList.contains("on-path") ? lit : paths).push(`M${x},${top} V${uy + 6} Q${x},${uy} ${x + 6},${uy} H${ux}`);
      }
    }
    const path = document.createElementNS(SVG, "path");
    path.setAttribute("d", paths.join(" "));
    const glow = document.createElementNS(SVG, "path");
    glow.setAttribute("d", lit.join(" "));
    glow.classList.add("map-path-lit");
    const dim = document.createElementNS(SVG, "path");
    dim.setAttribute("d", faded.join(" "));
    dim.classList.add("map-path-faded");
    lines.replaceChildren(dim, path, glow);
  }

  function step(dir: "up" | "down" | "left" | "right"): void {
    const key = state.selectedKey;
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

  return {
    id: "map",
    label: "Map",
    mount(host) {
      root.addEventListener("keydown", onKey);
      stage.style.zoom = String(scale);
      host.append(root);
    },
    render() {
      if (state.tree && restoredFor !== state.tree.root) {
        restoredFor = state.tree.root;
        void restore();
      }
      if (!restoring && state.tree) savePref(`fm.map.expanded:${state.tree.root}`, JSON.stringify([...state.expanded]));
      parents = new Map();
      const tops = nodesOf(state.tree?.nodes);
      const top = el("div", { class: "map-card map-root" }, el("span", { class: "map-name" }, rootName(state.tree?.root) || "Storage"));
      // The selected card is held in place on screen, so the centred layout reflowing around it does not look like a jump.
      const sel = state.selectedKey;
      const at = (k: string | null) => (k === null ? undefined : canvas.querySelector(`[data-key="${CSS.escape(k)}"]`)?.getBoundingClientRect());
      const before = at(sel);
      canvas.replaceChildren(el("div", { class: "map-branch" }, el("div", { class: "map-head" }, top), ...(tops.length > 0 ? [el("div", { class: "map-kids", role: "group" }, ...tops.map((n) => branch(n, null)))] : [])));
      top.classList.toggle("on-path", state.selectedKey !== null && parents.has(state.selectedKey));
      if (state.selectedKey && parents.has(state.selectedKey)) {
        root.setAttribute("aria-activedescendant", `map-${state.selectedKey}`);
        const after = at(sel);
        if (before && after) {
          for (const scroller of [root, root.parentElement]) {
            const now = at(sel);
            if (now) scroller?.scrollBy(now.left - before.left, now.top - before.top);
          }
        } else canvas.querySelector(`[data-key="${CSS.escape(state.selectedKey)}"]`)?.scrollIntoView({ block: "nearest", inline: "nearest" });
      } else root.removeAttribute("aria-activedescendant");
      top.dataset.key = "";
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(settle);
    },
    focus() {
      root.focus();
    },
    destroy() {
      cancelAnimationFrame(frame);
      root.remove();
    },
  };
}
