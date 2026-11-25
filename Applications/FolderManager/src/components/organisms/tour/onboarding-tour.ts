// First-run tour: a tooltip anchored to each basic control; using the control moves the tour on, so the user learns by doing.
import { el } from "../../dom";
import { textButton } from "../../atoms/button";
import { readPref, savePref } from "../../../services/store/prefs";
import { refresh } from "../../../hooks/ops";
import { setDemoHandler } from "../../../services/ipc/api";
import { createDemoStorage, type DemoStorage } from "../../../services/demo/demo-storage";
import { fakeExplorer, type FakeExplorer } from "./fake-explorer";
import { findNode, select, state } from "../../../services/store/state";
import type { TreeNode } from "../../../services/ipc/types";
import nodeReviewGif from "../../../assets/tour/node-review.gif";
import allowFilesGif from "../../../assets/tour/allow-files.gif";

const DONE_KEY = "fm.tour.done";

// A step lights every element `targets` gives; a click on `advanceOn` (by default the targets) moves on, and Next runs `onNext` instead.
interface Step {
  // Steps of one part teach one subject; Next Chapter jumps to the next one.
  part: number;
  targets: () => Element[];
  title: string;
  text: string;
  advanceOn?: () => Element[];
  onNext?: (targets: Element[]) => void;
  // Checked when the step comes up; a false skips it.
  when?: () => boolean;
  // While the step shows, only these menu items and dialog buttons take clicks; the rest are dimmed.
  allow?: () => Element[];
  // Shown instead of `text` while a confirmation dialog of the step is open.
  confirmText?: string;
  // Once shown (a menu item), the tip sits beside it and points there instead of at the targets.
  pointAt?: () => Element | null;
  // The tip points at this element instead of the lit targets.
  anchor?: () => Element | null;
  // A looping clip under the text that acts out what the step explains.
  media?: string;
  // The lit targets are only shown: the shield covers them too.
  inert?: boolean;
  // Lights each target on its own instead of one box around all of them.
  separate?: boolean;
  // Forces the tip to the left of the targets, for the side panel whose fields sit below them.
  left?: boolean;
  // Puts the tip right of the targets when it fits, where the empty view leaves room.
  right?: boolean;
  // A pick in the menu or popup this control opens closes it, so the tour opens it again to let the user keep trying.
  reopen?: () => HTMLElement | null;
  // Runs when the tour moves past the step or ends.
  leave?: () => void;
  // Runs when the step comes up, to put the view where the step needs it.
  enter?: () => void;
  // Polled while the step shows; the tour moves on once it holds, and clicks alone no longer do.
  done?: () => boolean;
}

const PARTS = ["Levels and nodes", "Projects", "Sorting", "Views", "Adding files", "Shortcuts"];

const one = (selector: string) => (): Element[] => [...document.querySelectorAll(selector)].slice(0, 1);
const menuItems = (danger: boolean) => (): Element[] => [...document.querySelectorAll(danger ? ".ctx-menu .ctx-item.danger" : ".ctx-menu .ctx-item")];
const rowOf = (key: string | null): Element[] => (key ? [...document.querySelectorAll(`.tree-host .col-item[data-key="${CSS.escape(key)}"]`)].slice(0, 1) : []);

const rowsIn = (column: number) => (): Element[] => [...(document.querySelectorAll(".tree-host .column")[column]?.querySelectorAll<HTMLElement>(":scope > .col-item[data-key]") ?? [])];

// Only numbered user categories count; Downloads, Recordings, Screenshots and the 9xx system folders come later.
const categories = (): Element[] => rowsIn(0)().filter((r) => /^[1-8]\d\d\./.test((r as HTMLElement).dataset.key ?? ""));

// Columns open a folder with a click on its arrow (on the name it starts an edit), so Next sends one there.
const openRow = (rows: Element[], prefer?: string): void => {
  const row = (prefer ? rows.find((r) => r.textContent?.trim().startsWith(prefer)) : undefined) ?? rows[0];
  row?.querySelector(".chevron")?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
};

const menuChoice = (label: string) => (): Element[] => menuItems(false)().filter((e) => e.textContent?.trim() === label);
const allowFiles = menuChoice("Allow files");
// Selecting a node closes its open column, so these steps only open the Projects list through its arrow when it is not shown.
// The demo's Projects list holds a review node with loose files and the example project with files inside.
const UPPER = "200.WORK/03.Projects";
const REVIEW = `${UPPER}/REV.Projects`;
const EXAMPLE = `${UPPER}/PJ.001_Example-Project`;
const newWorknode = menuChoice("New worknode");

// The first worknode of the demo (PJ.001_Example-Project) is selected, so its folder opens beside the projects list.
function firstWorknode(nodes: TreeNode[]): string[] | null {
  for (const n of nodes) {
    if (n.nodeType?.base === "WKB") return n.relParts;
    const below = firstWorknode(n.children ?? []);
    if (below) return below;
  }
  return null;
}
const openProjects = (): void => select(firstWorknode(state.tree?.nodes ?? []));

const visible = (e: Element): boolean => (e as HTMLElement).offsetParent !== null;
const editButton = (): Element[] => [...document.querySelectorAll('button[aria-label="Edit details"]')].filter(visible).slice(0, 1);
const nameField = (): Element[] => [...document.querySelectorAll(".details-name")].filter((e) => e.querySelector(".name-fixed") && visible(e)).slice(0, 1);
const saveButton = (): Element[] => [...document.querySelectorAll(".details-panel .btn-primary")].filter((e) => e.textContent?.trim() === "Save" && visible(e)).slice(0, 1);
const selectedStatus = (): string | null => (state.selectedKey ? (findNode(state.selectedKey.split("/"))?.status ?? null) : null);

// Columns shows the project and its contents side by side, so a drop is easy to see.
const easyDropView = (): void => {
  if (host.currentView() !== "columns") host.showView("columns");
};
// Opens the project's column through its arrow, so the dropped files show beside it.
const openColumnOf = (key: string | null): void => {
  if (key) document.querySelector(`.tree-host .col-item[data-key="${CSS.escape(key)}"] .chevron`)?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
};
// The project row and, once open, the column listing its contents both take a drop.
const dropZones = (key: string | null): Element[] => (key ? [...itemOf(key), ...document.querySelectorAll(`.tree-host .column[data-key="${CSS.escape(key)}"]`)] : []);
const showProjects = (): void => {
  if (rowOf(REVIEW).length === 0) openColumnOf(UPPER);
};
const itemOf = (key: string | null): Element[] => (key ? [...document.querySelectorAll(`.tree-host [data-key="${CSS.escape(key)}"]`)].slice(0, 1) : []);

function steps(): Step[] {
  let project: string | null = null;
  let explorer: FakeExplorer | null = null;
  const projectKey = (): string | null => project ?? firstWorknode(state.tree?.nodes ?? [])?.join("/") ?? null;
  return [
    { targets: categories, part: 0, title: "Main categories", text: "These are the main categories of your Storage, the top level of everything you keep. Try opening one.", onNext: (rows) => openRow(rows, "200."), done: () => rowsIn(1)().length > 0, right: true },
    { targets: rowsIn(1), part: 0, title: "Level 2", text: "Each folder you open takes you one level deeper. You are on level 2, where we suggest keeping the subcategories of the one before. Try going to level 3.", onNext: (rows) => openRow(rows, "03."), done: () => rowsIn(2)().length > 0 },
    { targets: rowsIn(2), part: 0, title: "Nodes", text: "This is level 3, by default the deepest level we suggest. In ANT, folders are called nodes.", advanceOn: () => [] },
    { targets: () => rowOf(REVIEW), part: 0, title: "Review Nodes", text: "By default index levels refuse any files and send them to the Review node of the same level until you sort them.", enter: showProjects, advanceOn: () => [], inert: true, media: nodeReviewGif },
    {
      targets: () => dropZones(EXAMPLE),
      separate: true,
      part: 0, title: "Files",
      left: true,
      anchor: () => [...document.querySelectorAll(`.tree-host .column[data-key="${CSS.escape(EXAMPLE)}"] > .col-item`)].at(-1) ?? null,
      text: "Your files should live on the levels past the index. The default depth is 3, so your files stay on level 4 or deeper.",
      enter: () => {
        showProjects();
        setTimeout(() => openColumnOf(EXAMPLE), 100);
      },
      advanceOn: () => [],
      inert: true,
    },
    {
      targets: () => rowOf(UPPER),
      part: 0, title: "Allow files",
      text: "Even so, you can always allow files in a node inside the index when you need to. Try it with a right-click to open its context menu and choose Allow files.",
      media: allowFilesGif,
      pointAt: () => allowFiles()[0] ?? null,
      allow: allowFiles,
      done: () => findNode(UPPER.split("/"))?.flags.includes("data") ?? false,
    },
    {
      targets: one(".nav-add"),
      part: 1, title: "Create a worknode",
      text: "Projects live in worknodes. Press + and choose New worknode; it takes the next project number.",
      enter: openProjects,
      advanceOn: newWorknode,
      allow: newWorknode,
    },
    {
      targets: () => rowOf(state.selectedKey?.endsWith("_New-Worknode") ? state.selectedKey : null),
      part: 1, title: "Your project",
      when: () => {
        if (state.selectedKey?.endsWith("_New-Worknode")) project = state.selectedKey;
        return true;
      },
      text: "This is your new project, numbered right after the example. Worknodes track a status from start to done.",
    },
    {
      targets: editButton,
      part: 1, title: "Edit your project",
      text: "Press the pencil to edit the details of your project.",
      onNext: (els) => (els[0] as HTMLElement | undefined)?.click(),
      done: () => nameField().length > 0,
    },
    { targets: nameField, part: 1, title: "Name it", text: "Type a name you like. The PJ number stays; it is how ANT tracks the project. Then press Next.", advanceOn: () => [], left: true },
    {
      targets: one(".details-panel .status-picker"),
      part: 1, title: "Set the status",
      left: true,
      text: "Open the status list and choose In progress.",
      allow: () => [...document.querySelectorAll(".ctx-menu .ctx-status.st-in-progress")],
      pointAt: () => document.querySelector(".ctx-menu .ctx-status.st-in-progress"),
      done: () => document.querySelector(".details-panel .status-picker.st-in-progress") !== null,
    },
    {
      targets: saveButton,
      part: 1, title: "Save",
      left: true,
      text: "Save to apply the new name and status.",
      done: () => {
        const saved = selectedStatus() === "in-progress" && saveButton().length === 0;
        if (saved) project = state.selectedKey;
        return saved;
      },
    },
    {
      targets: one(".sort-btn"),
      part: 2, title: "Sort",
      text: "We added more projects to try it. Open Sort and order them by track, name, date or status as many times as you like, then press Next.",
      enter: () => void seedProjects().then(() => document.querySelector<HTMLElement>(".sort-btn")?.click()),
      advanceOn: () => [],
      reopen: () => document.querySelector<HTMLElement>(".sort-btn"),
    },
    {
      targets: one(".view-picker:not(.sort-btn)"),
      part: 3, title: "Views",
      text: "Columns is not the only view. Open the views and switch between Cards, Tree, Grid, List, Details and Graph as you like, then press Next.",
      enter: () => document.querySelector<HTMLElement>(".view-picker:not(.sort-btn)")?.click(),
      advanceOn: () => [],
      reopen: () => document.querySelector<HTMLElement>(".view-picker:not(.sort-btn)"),
    },
    {
      targets: () => itemOf(projectKey()),
      part: 4, title: "Select your project",
      text: "Click your project to select it; files go into the folder you pick.",
      enter: easyDropView,
      onNext: (els) => (els[0] as HTMLElement | undefined)?.click(),
      done: () => state.selectedKey === projectKey(),
    },
    {
      targets: () => dropZones(projectKey()),
      separate: true,
      part: 4, title: "Drop your files",
      text: "Drag files from the explorer window onto your project or into its open column. In a real Storage this moves them in. Press Next when you are done.",
      advanceOn: () => [],
      enter: () => {
        easyDropView();
        openColumnOf(projectKey());
        explorer?.remove();
        explorer = fakeExplorer(
          (target) => dropZones(projectKey()).some((zone) => zone.contains(target)),
          (file) => {
            const key = projectKey();
            if (!key || !activeDemo) return;
            activeDemo.addFile(key.split("/"), file.name, file.size);
            void reloadWhenIdle().then(() => openColumnOf(key));
          },
          () => itemOf(projectKey())[0] ?? null,
        );
      },
      leave: () => {
        explorer?.remove();
        explorer = null;
      },
    },
    { targets: one(".search-box"), part: 5, title: "Search", text: "Find any folder or file in the Storage by its name." },
    { targets: one(".side-rail"), part: 5, title: "Side panel", text: "Settings, the board, your downloads, recordings, screenshots and review folders are one click away." },
  ];
}

function union(elements: Element[]): DOMRect | null {
  const rects = elements.map((e) => e.getBoundingClientRect());
  if (!rects.length) return null;
  const left = Math.min(...rects.map((r) => r.left));
  const top = Math.min(...rects.map((r) => r.top));
  return new DOMRect(left, top, Math.max(...rects.map((r) => r.right)) - left, Math.max(...rects.map((r) => r.bottom)) - top);
}

// A check and "Good job!" float in the centre for a moment, then the next chapter starts.
function cheer(then: () => void): void {
  const mark = el("div", { class: "tour-cheer", role: "status" }, el("div", { class: "tour-cheer-mark", "aria-hidden": "true" }), el("span", {}, "Good job!"));
  document.body.append(mark);
  window.setTimeout(() => {
    mark.remove();
    then();
  }, 2600);
}

export const tourSeen = (): boolean => readPref(DONE_KEY) === "true";
export const skipTour = (): void => savePref(DONE_KEY, "true");

// The main window owns the views: it puts the tour's own view in place, and lets a step read or change the active one.
interface TourHost {
  prepare(): void;
  currentView(): string;
  showView(id: string): void;
}
let host: TourHost = { prepare: () => {}, currentView: () => "", showView: () => {} };
// Dev only: the step survives a Vite reload, so a step can be reworked without walking the tour again.
const DEV_STEP = "fm.devTourStep";
const devStep = (): number | null => {
  if (!import.meta.env.DEV) return null;
  try {
    const raw = sessionStorage.getItem(DEV_STEP);
    return raw === null ? null : Number(raw);
  } catch (e) {
    console.warn("Dev tour step unreadable", e);
    return null;
  }
};
const saveDevStep = (i: number | null): void => {
  if (!import.meta.env.DEV) return;
  try {
    if (i === null) sessionStorage.removeItem(DEV_STEP);
    else sessionStorage.setItem(DEV_STEP, String(i));
  } catch (e) {
    console.warn("Dev tour step not saved", e);
  }
};

export function onTourStart(next: TourHost): void {
  host = next;
  if (devStep() !== null) void startTour();
}

// The tour runs on a demo Storage from the bundled template; the real one is read again when it ends.
// A refresh is refused while another operation runs (the launch checks, a pending reload), so it is tried again until it lands.
async function reloadWhenIdle(): Promise<void> {
  for (let tries = 0; tries < 50; tries++) {
    if (await refresh()) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  console.error("The tutorial could not reload the tree");
}

let activeDemo: DemoStorage | null = null;

// The made-up projects land next to the user's own one, and the tree is read again to show them.
async function seedProjects(): Promise<void> {
  const parent = (state.selectedKey?.includes("_") ? state.selectedKey : firstWorknode(state.tree?.nodes ?? [])?.join("/"))?.split("/").slice(0, -1) ?? [];
  if (!activeDemo || parent.length === 0) return;
  activeDemo.seedProjects(parent);
  await reloadWhenIdle();
}

async function enterDemo(): Promise<void> {
  const demo = createDemoStorage();
  activeDemo = demo;
  setDemoHandler((cmd, args) => demo.handle(cmd, args));
  state.expanded.clear();
  select(null);
  await reloadWhenIdle();
}

async function leaveDemo(): Promise<void> {
  activeDemo = null;
  setDemoHandler(null);
  state.expanded.clear();
  select(null);
  await reloadWhenIdle();
}

export async function startTour(): Promise<void> {
  host.prepare();
  await enterDemo();
  const all = steps();
  let index = -1;
  // The dim layer is an SVG with a hole per lit target, so a row and a column light up without the space between them.
  const SVG = "http://www.w3.org/2000/svg";
  const spot = document.createElementNS(SVG, "svg");
  spot.setAttribute("class", "tour-spot");
  spot.setAttribute("aria-hidden", "true");
  const dim = document.createElementNS(SVG, "path");
  dim.setAttribute("class", "tour-dim");
  const rim = document.createElementNS(SVG, "path");
  rim.setAttribute("class", "tour-rim");
  spot.append(dim, rim);
  // One pane with the same holes cut out takes every click outside them, so only the lit controls can be used.
  const shield = el("div", { class: "tour-shield", "aria-hidden": "true" });
  const shields = [shield];
  const title = el("h2", { id: "tour-title" });
  const text = el("p", { class: "tour-text", id: "tour-text" });
  const media = el("img", { class: "tour-media", alt: "", hidden: "" });
  // The tip grows once the clip loads, so it is placed again.
  media.addEventListener("load", () => place());
  const next = textButton("Next", "btn-primary", () => {
    const step = all[index];
    // onNext acts on a target itself, and the click or the `done` check it causes moves the tour on.
    if (step?.onNext) step.onNext(step.targets());
    else goOn();
  });
  const skip = el("button", { type: "button", class: "tour-skip tour-quit" }, "Skip tour");
  const count = el("span", { class: "tour-count" });
  const skipPart = el("button", { type: "button", class: "tour-skip" }, "Next Chapter");
  skipPart.addEventListener("click", () => {
    const part = all[index]?.part ?? 0;
    if (part + 1 >= PARTS.length) finish();
    else jumpTo(part + 1);
  });
  skip.addEventListener("click", finish);
  const tip = el("div", { class: "tour-tip", role: "dialog", tabindex: "-1", "aria-labelledby": "tour-title", "aria-describedby": "tour-text" }, el("div", { class: "tour-arrow" }), el("div", { class: "tour-head" }, title, count), text, media, el("div", { class: "tour-actions" }, skip, el("span", { class: "tour-forward" }, skipPart, next)));

  // The chapter checklist: done chapters are ticked, any chapter can be opened again, and the bar shows how far the user got.
  const finished = new Set<number>();
  const cheered = new Set<number>();
  const bar = el("span", { class: "tour-bar-fill" });
  const percent = el("span", { class: "tour-percent" });
  const chapters = el("ul", { class: "tour-chapters" });
  const checklist = el(
    "aside",
    { class: "tour-checklist", "aria-label": "Tutorial chapters" },
    el("div", { class: "tour-check-head" }, el("strong", {}, "Getting started with ANT"), el("div", { class: "tour-progress" }, percent, el("span", { class: "tour-bar" }, bar))),
    chapters,
  );
  let jumping = false;
  const jumpTo = (part: number): void => {
    const first = all.findIndex((s) => s.part === part);
    if (first < 0) return;
    jumping = true;
    show(first);
  };
  function paintChecklist(): void {
    const current = all[index]?.part ?? -1;
    const share = Math.round((finished.size / PARTS.length) * 100);
    percent.textContent = `${share}%`;
    bar.style.width = `${share}%`;
    chapters.replaceChildren(
      ...PARTS.map((name, part) => {
        const item = el("button", { type: "button", class: "tour-chapter" }, el("span", { class: "tour-mark", "aria-hidden": "true" }), name);
        item.classList.toggle("is-done", finished.has(part));
        item.classList.toggle("is-current", part === current);
        if (part === current) item.setAttribute("aria-current", "step");
        item.addEventListener("click", () => jumpTo(part));
        return el("li", {}, item);
      }),
    );
  }

  function place(): void {
    // A modal sits in the top layer above the page, so the tip joins it to stay visible and clickable.
    const host = document.querySelector("dialog[open]") ?? document.body;
    if (tip.parentElement !== host) host.append(tip);
    const allowed = all[index]?.allow?.() ?? null;
    document.body.classList.toggle("tour-restrict", allowed !== null);
    document.querySelectorAll(".tour-allow").forEach((e) => e.classList.remove("tour-allow"));
    allowed?.forEach((e) => e.classList.add("tour-allow"));
    const lit = union(all[index]?.targets() ?? []);
    if (!lit) return;
    const holes = all[index]?.separate ? (all[index]?.targets() ?? []).map((e) => e.getBoundingClientRect()) : [lit];
    const [w, h] = [window.innerWidth, window.innerHeight];
    const cut = holes.map((r) => `M${r.left - 4} ${r.top - 4}h${r.width + 8}v${r.height + 8}h${-(r.width + 8)}Z`).join("");
    const frame = `M0 0H${w}V${h}H0Z${cut}`;
    spot.setAttribute("viewBox", `0 0 ${w} ${h}`);
    dim.setAttribute("d", frame);
    rim.setAttribute("d", cut);
    shield.style.clipPath = all[index]?.inert ? "none" : `path(evenodd, "${frame}")`;
    // A menu the control opened is kept clear too, so its choices and the tip show together.
    const menu = document.querySelector(".ctx-menu, .view-segment");
    const pointed = all[index]?.pointAt?.() ?? null;
    pointed?.classList.add("tour-point");
    const anchored = all[index]?.anchor?.() ?? null;
    const r = pointed ? pointed.getBoundingClientRect() : anchored ? anchored.getBoundingClientRect() : ((menu ? union([...(all[index]?.targets() ?? []), menu]) : null) ?? lit);
    const t = tip.getBoundingClientRect();
    const margin = 12;
    // Below the target when it fits, else beside it for tall targets such as the tree and the rail.
    const inDialog = pointed?.closest("dialog") !== null && pointed !== null;
    const wanted = (inDialog ? all[index]?.confirmText : undefined) ?? all[index]?.text ?? "";
    if (text.textContent !== wanted) text.textContent = wanted;
    // Beside a menu item, so the menu stays readable; under a dialog button, so the dialog's other buttons stay clear.
    const forceLeft = all[index]?.left === true && !pointed;
    const right = !forceLeft && r.right + margin + t.width <= window.innerWidth;
    const below = !forceLeft && !(all[index]?.right && right && !pointed) && (inDialog || !pointed) && r.bottom + margin + t.height <= window.innerHeight;
    let left: number;
    let top: number;
    let side: string;
    if (below) [left, top, side] = [r.left + r.width / 2 - t.width / 2, r.bottom + margin, "top"];
    else if (right) [left, top, side] = [r.right + margin, r.top + Math.min(r.height / 2, 120) - 24, "left"];
    else [left, top, side] = [r.left - margin - t.width, r.top + Math.min(r.height / 2, 120) - 24, "right"];
    left = Math.max(margin, Math.min(left, window.innerWidth - t.width - margin));
    top = Math.max(margin, Math.min(top, window.innerHeight - t.height - margin));
    // The tip keeps clear of the pretend explorer: the first spot around the target that it does not cover wins.
    const blockers = [...document.querySelectorAll(".fx-window, .tour-checklist")].map((e) => e.getBoundingClientRect());
    const hits = (x: number, y: number): boolean => blockers.some((b) => x < b.right && x + t.width > b.left && y < b.bottom && y + t.height > b.top);
    if (hits(left, top)) {
      const spots: [number, number, string][] = [
        [r.right + margin, r.top, "left"],
        [r.left - margin - t.width, r.top, "right"],
        [r.left, r.bottom + margin, "top"],
        [r.left, r.top - margin - t.height, "bottom"],
        [margin, window.innerHeight - t.height - margin, "none"],
      ];
      const fits = (x: number, y: number): boolean => x >= margin && y >= margin && x + t.width <= window.innerWidth - margin && y + t.height <= window.innerHeight - margin;
      const free = spots.find(([x, y]) => fits(x, y) && !hits(x, y)) ?? spots[spots.length - 1];
      if (free) [left, top, side] = free;
    }
    Object.assign(tip.style, { left: `${left}px`, top: `${top}px` });
    tip.dataset.side = side;
    const across = side === "top" || side === "bottom";
    const arrow = across ? r.left + r.width / 2 - left : r.top + Math.min(r.height / 2, 120) - top;
    tip.style.setProperty("--arrow", `${Math.max(16, Math.min(arrow, (across ? t.width : t.height) - 16))}px`);
  }

  // A dialog the step opened (the new node name, the delete confirmation) is waited out before the next tip shows.
  function goOn(): void {
    setTimeout(() => {
      if (document.querySelector("dialog[open]")) goOn();
      else show(index + 1);
    }, 200);
  }

  // Runs after the pick's own handler closed the menu, in the same task, so the reopened menu is all the page ever shows.
  function onPicked(ev: MouseEvent): void {
    const reopen = all[index]?.reopen;
    const picked = ev.target instanceof Element && ev.target.closest(".ctx-menu button, .view-segment button");
    if (picked && reopen && !document.querySelector(".ctx-menu, .view-segment")) reopen()?.click();
  }

  // Views rebuild their rows on every render, so the click is matched against the current elements, not saved ones.
  function onClick(ev: MouseEvent): void {
    const step = all[index];

    if (step?.done) return;
    const hit = ev.target instanceof Node && (step?.advanceOn ?? step?.targets)?.().some((t) => t.contains(ev.target as Node));
    if (hit) goOn();
  }

  let watching = false;
  const watch = window.setInterval(() => {
    const step = all[index];
    if (watching && step?.done?.()) {
      watching = false;
      goOn();
    }
  }, 150);

  // A "Good job" card may call back after the user already left the tour.
  let over = false;
  let fadeIn = 0;
  function show(from: number): void {
    if (over) return;
    const i = all.findIndex((s, n) => n >= from && (s.when?.() ?? true) && s.targets().length > 0);
    const step = all[i];
    if (!step) return finish();
    if (i !== index) all[index]?.leave?.();
    // Walking out of a chapter's last step finishes it; a jump from the checklist or Next Chapter does not.
    const left = all[index]?.part;
    if (!jumping && left !== undefined && left !== step.part) {
      finished.add(left);
      // A chapter of several steps ends on a short "Good job" before the next one starts.
      if (all.filter((s) => s.part === left).length > 1 && !cheered.has(left)) {
        cheered.add(left);
        paintChecklist();
        tip.style.visibility = "hidden";
        cheer(() => show(from));
        return;
      }
    }
    jumping = false;
    // Between steps the tip vanishes at once and fades in where the next step puts it, once its view has settled.
    if (i !== index) {
      tip.classList.add("tour-switching");
      window.clearTimeout(fadeIn);
      fadeIn = window.setTimeout(() => {
        place();
        tip.classList.remove("tour-switching");
      }, 250);
    }
    index = i;
    saveDevStep(i);
    step.enter?.();
    watching = Boolean(step.done);
    title.textContent = step.title;
    text.textContent = step.text;
    media.hidden = !step.media;
    if (step.media) media.src = step.media;
    else media.removeAttribute("src");
    next.textContent = i === all.length - 1 ? "Finish" : "Next";
    // The count runs inside the chapter; a one-step chapter needs none.
    const inPart = all.filter((s) => s.part === step.part);
    count.textContent = inPart.length > 1 ? `${inPart.indexOf(step) + 1} of ${inPart.length}` : "";
    paintChecklist();
    skipPart.hidden = step.part === PARTS.length - 1;
    tip.style.visibility = "";
    place();
    tip.focus();
  }

  function onKey(ev: KeyboardEvent): void {
    if (ev.key === "Escape" && !document.querySelector("dialog[open], .ctx-menu")) finish();
  }

  function finish(): void {
    over = true;
    saveDevStep(null);
    devBar?.remove();
    document.querySelector(".tour-cheer")?.remove();
    savePref(DONE_KEY, "true");
    all[index]?.leave?.();
    window.clearInterval(watch);
    menus.disconnect();
    document.removeEventListener("click", onClick, true);
    document.removeEventListener("click", onPicked);
    spot.remove();
    shields.forEach((pane) => pane.remove());
    document.body.classList.remove("touring", "tour-restrict");
    tip.remove();
    checklist.remove();
    window.removeEventListener("resize", place);
    window.removeEventListener("tour-explorer-moved", place);
    document.removeEventListener("keydown", onKey);
    void leaveDemo();
  }

  // Menus open and close outside the tour's control, so the tip is placed again whenever the page's top-level children change.
  // The tour's own nodes are skipped, since placing the tip changes them and would call itself again.
  const ours = (n: Node): boolean => n === tip || tip.contains(n) || n === checklist || checklist.contains(n) || n === spot || spot.contains(n) || n === shield;
  const menus = new MutationObserver((records) => {
    const moved = (r: MutationRecord): boolean => r.type === "attributes" || ![...r.addedNodes, ...r.removedNodes].every(ours);
    if (records.some((r) => !ours(r.target) && moved(r))) place();
  });
  menus.observe(document.body, { childList: true, subtree: true, attributeFilter: ["open"] });
  document.body.append(...shields, spot, checklist, tip);
  document.body.classList.add("touring");
  window.addEventListener("resize", place);
  window.addEventListener("tour-explorer-moved", place);
  document.addEventListener("keydown", onKey);
  document.addEventListener("click", onClick, true);
  document.addEventListener("click", onPicked);
  const resume = devStep();
  const devBar = import.meta.env.DEV ? debugBar() : null;
  if (devBar) document.body.append(devBar);
  jumping = resume !== null;
  show(resume ?? 0);

  // Dev only, unstyled on purpose: steps back and forth while debugging the tour.
  function debugBar(): HTMLElement {
    const usable = (i: number): boolean => (all[i]?.when?.() ?? true) && (all[i]?.targets().length ?? 0) > 0;
    const step = (dir: -1 | 1) => () => {
      for (let i = index + dir; i >= 0 && i < all.length; i += dir) {
        if (!usable(i)) continue;
        jumping = true;
        show(i);
        return;
      }
    };
    const back = el("button", { type: "button" }, "◀ Prev step");
    const ahead = el("button", { type: "button" }, "Next step ▶");
    back.addEventListener("click", step(-1));
    ahead.addEventListener("click", step(1));
    return el("div", { style: "position:fixed;bottom:12px;left:50%;transform:translateX(-50%);z-index:99999;display:flex;gap:8px" }, back, ahead);
  }
}
