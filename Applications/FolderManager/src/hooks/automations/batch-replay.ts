// A run is over in a blink, so the user sees it again at a readable pace: names retyped, folders made, files filed, then their trip through the graph.
import { createStage, type Stage } from "../../components/windows/views/batch-replay-stage";
import { createClock, ended, type ReplayClock } from "./replay-clock";

const FADE_MS = 500;
import { currentSplit, paneOn } from "../split";
import type { Chained, PlannedFile } from "../../services/ipc/types";

// The toolbar names the view, so it is told when the replay switches one.
export interface ReplayHost {
  viewChanged(): void;
}

export interface FilesMoved {
  targets: Moved[];
  // Where a review folder that runs on its own sent the files on, for a second trip from it.
  chained: Chained[];
  wait(ms: number): Promise<void>;
  track(a: Animation): Animation;
  handled: Promise<void> | null;
}

export type ParkedReplay = Replay;

interface Replay {
  clock: ReplayClock;
  stage: Stage;
  pane: HTMLElement;
  right: boolean;
  // The view to come back to once the trip is shown.
  back: string;
  graph: boolean;
}

let host: ReplayHost | null = null;
let live: Replay | null = null;

export const bindReplay = (h: ReplayHost): void => {
  host = h;
};

// A batch remembers its pane even after the split closes, and then it stands in the left one.
const side = (right: boolean): boolean => right && currentSplit() !== null;
export interface Snapshot {
  view: HTMLElement | null;
  scroll: number;
}

// Taken before the run, while the hidden view still lists the folder as it was.
export function snapshotView(right: boolean): Snapshot {
  const root = paneOn(side(right))?.host.firstElementChild;
  const view = root instanceof HTMLElement ? root : null;
  return { view: (view?.cloneNode(true) as HTMLElement | undefined) ?? null, scroll: view?.scrollTop ?? 0 };
}

function show(r: Replay, id: string): void {
  const views = paneOn(r.right)?.views;
  if (!views || views.current().id === id) return;
  views.switchTo(id);
  host?.viewChanged();
}

// Pinned to where the pane sits on screen: the pane scrolls (the graph moves it to follow a packet) and the replay must not scroll with it.
function pin(stage: HTMLElement, pane: HTMLElement): void {
  const place = () => {
    const box = pane.getBoundingClientRect();
    Object.assign(stage.style, { position: "fixed", left: `${box.left}px`, top: `${box.top}px`, width: `${box.width}px`, height: `${box.height}px` });
  };
  place();
  const watch = new ResizeObserver(place);
  watch.observe(pane);
  stage.addEventListener("replay-end", () => watch.disconnect(), { once: true });
}

function finish(r: Replay): void {
  r.stage.element.dispatchEvent(new Event("replay-end"));
  r.stage.element.remove();
  r.pane.classList.remove("replaying");
  if (live === r) live = null;
  show(r, r.back);
}

// One entry per destination node, so the graph sends a single packet for every group of files going the same way.
export interface Moved {
  parts: string[];
  // The names the files land under, so the graph can show them there even before the disk has caught up.
  names: string[];
}

function movedTo(rows: PlannedFile[]): Moved[] {
  const out = new Map<string, Moved>();
  for (const r of rows) {
    if (!r.target) continue;
    // A folder the run created has a name the plan could not know, so the trip ends at its parent.
    const parts = r.create ? r.target.slice(0, -1) : r.target;
    const key = parts.join("/");
    const hit = out.get(key);
    if (hit) hit.names.push(r.name);
    else out.set(key, { parts, names: [r.name] });
  }
  return [...out.values()];
}

async function travel(r: Replay, rows: PlannedFile[], chained: Chained[]): Promise<void> {
  const targets = movedTo(rows);
  if (targets.length === 0) return;
  r.graph = true;
  // The graph is put in place under the replay, which then fades away to reveal it.
  show(r, "graph");
  const stage = r.stage.element;
  const grid = stage.querySelector<HTMLElement>(".replay-grid");
  const fades = [r.clock.track(stage.animate([{ backgroundColor: getComputedStyle(stage).backgroundColor }, { backgroundColor: "transparent" }], { duration: FADE_MS, easing: "ease-out", fill: "forwards" }))];
  if (grid) fades.push(r.clock.track(grid.animate([{ opacity: 1 }, { opacity: 0 }], { duration: FADE_MS, easing: "ease-out", fill: "forwards" })));
  await Promise.all(fades.map(ended));
  stage.classList.add("through");
  const news: FilesMoved = { targets, chained, wait: (ms) => r.clock.wait(ms), track: (a) => r.clock.track(a), handled: null };
  window.dispatchEvent(new CustomEvent("fm:files-moved", { detail: news }));
  await news.handled;
  await r.clock.wait(500);
}

async function play(r: Replay, rows: PlannedFile[], chained: Chained[]): Promise<void> {
  const steps = [() => r.clock.wait(300), () => Promise.all([r.stage.rename(r.clock), r.stage.addFolders(r.clock)]), () => r.clock.wait(350), () => r.stage.file(r.clock), () => r.clock.wait(250), () => travel(r, rows, chained)];
  for (const step of steps) {
    if (r.clock.stopped) return;
    await step();
  }
  finish(r);
}

export function startReplay(shot: Snapshot, folder: string[], rows: PlannedFile[], dirs: Set<string>, right: boolean, chained: Chained[]): void {
  // It runs even with reduced motion: the user asked for it, and Esc ends it.
  const at = side(right);
  const p = paneOn(at);
  if (!p || rows.length === 0) return;
  if (live) skipReplay();
  const stage = createStage(shot.view, folder, rows, dirs);
  p.host.classList.add("replaying");
  p.host.append(stage.element);
  pin(stage.element, p.host);
  const grid = stage.element.firstElementChild;
  if (grid) grid.scrollTop = shot.scroll;
  const r: Replay = { clock: createClock(), stage, pane: p.host, right: at, back: p.views.current().id, graph: false };
  live = r;
  void play(r, rows, chained);
}

// Esc ends the show at once; the files were moved long before it.
export function skipReplay(): boolean {
  const r = live;
  if (!r) return false;
  r.clock.stop();
  finish(r);
  return true;
}

// The replay belongs to the tab it started in: leaving pauses it out of sight, coming back resumes it.
export function parkReplay(): Replay | null {
  const r = live;
  if (!r) return null;
  r.clock.pause();
  r.stage.element.hidden = true;
  live = null;
  return r;
}

export function unparkReplay(r: Replay | null): void {
  if (!r || r.clock.stopped) return;
  live = r;
  r.stage.element.hidden = false;
  if (r.graph) show(r, "graph");
  r.clock.resume();
}

window.addEventListener(
  "keydown",
  (ev) => {
    if (ev.key !== "Escape" || !live) return;
    ev.preventDefault();
    ev.stopPropagation();
    skipReplay();
  },
  true,
);
