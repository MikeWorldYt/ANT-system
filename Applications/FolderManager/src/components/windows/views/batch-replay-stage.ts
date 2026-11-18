// The folder exactly as the user's view drew it before the run, copied so the replay can rename, gather and file its tiles in place.
import { el } from "../../dom";
import { nameIcon } from "../../atoms/icons";
import { ended, type ReplayClock } from "../../../hooks/automations/replay-clock";
import type { PlannedFile } from "../../../services/ipc/types";

const TYPE_MS = 11;
const GATHER_MS = 390;
const ENTER_MS = 270;
const SETTLE_MS = 240;

export interface Stage {
  element: HTMLElement;
  rename(clock: ReplayClock): Promise<void>;
  addFolders(clock: ReplayClock): Promise<void>;
  file(clock: ReplayClock): Promise<void>;
}

const folderTile = (name: string) => el("div", { class: "grid-item replay-folder", "data-folder": name }, el("span", { class: "grid-art" }, nameIcon(name)), el("span", { class: "name" }, el("span", { class: "name-text" }, name)));

// The group a file joins: a folder inside Downloads, or the node it leaves for.
const groupOf = (r: PlannedFile): string | null => (r.target ? `@${r.target.join("/")}` : (r.folder ?? r.project));

// The rest of the grid slides into the room the moved tiles left, the way the real view closes up.
function closeUp(clock: ReplayClock, grid: HTMLElement, gone: HTMLElement[], change: () => void): Promise<void> {
  const rest = [...grid.querySelectorAll<HTMLElement>(":scope .grid-item")].filter((t) => !gone.includes(t) && t.parentElement === grid);
  const before = new Map(rest.map((t) => [t, t.getBoundingClientRect()]));
  change();
  const moves = rest.map((t) => {
    const was = before.get(t)!;
    const now = t.getBoundingClientRect();
    const dx = was.left - now.left;
    const dy = was.top - now.top;
    if (dx === 0 && dy === 0) return null;
    return clock.track(t.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }], { duration: SETTLE_MS, easing: "ease-out" }));
  });
  return Promise.all(moves.filter((m): m is Animation => m !== null).map(ended)).then(() => undefined);
}

export function createStage(view: HTMLElement | null, folder: string[], rows: PlannedFile[], dirs: Set<string>): Stage {
  // A copy of the real view, so the first frame of the replay is the very frame the user was looking at.
  const grid = (view?.cloneNode(true) as HTMLElement | undefined) ?? el("div", { class: "grid" });
  grid.classList.add("replay-grid");
  grid.removeAttribute("tabindex");
  grid.querySelectorAll(".selected").forEach((t) => t.classList.remove("selected"));
  const tileOf = (name: string) => grid.querySelector<HTMLElement>(`.grid-item[data-key="${CSS.escape([...folder, name].join("/"))}"]`);
  const tiles = new Map<PlannedFile, HTMLElement>();
  for (const r of rows) {
    const t = tileOf(r.file);
    if (t) tiles.set(r, t);
  }
  const inside = new Set(rows.filter((r) => !r.target).map((r) => r.folder ?? r.project).filter((f): f is string => !!f));
  const folders = new Map<string, HTMLElement>();
  for (const f of inside) {
    const t = dirs.has(f.toLowerCase()) ? tileOf(f) : null;
    if (t) folders.set(f, t);
  }
  const fresh = [...inside].filter((f) => !folders.has(f));
  const element = el("div", { class: "batch-replay" }, grid, el("p", { class: "replay-hint" }, "Press Esc to skip the animation"));

  // Each name loses the part that goes and types the part that comes, keeping what both share.
  async function retype(clock: ReplayClock, r: PlannedFile): Promise<void> {
    const label = tiles.get(r)?.querySelector<HTMLElement>(".name-text");
    if (!label || r.name === r.file) return;
    let head = 0;
    while (head < r.file.length && r.file[head] === r.name[head]) head++;
    let tail = 0;
    while (tail < r.file.length - head && tail < r.name.length - head && r.file.at(-1 - tail) === r.name.at(-1 - tail)) tail++;
    const end = r.file.slice(r.file.length - tail);
    label.classList.add("typing");
    for (let cut = r.file.length - tail; cut > head && !clock.stopped; cut--) {
      label.textContent = r.file.slice(0, cut - 1) + end;
      await clock.wait(TYPE_MS);
    }
    const middle = r.name.slice(head, r.name.length - tail);
    for (let i = 1; i <= middle.length && !clock.stopped; i++) {
      label.textContent = r.name.slice(0, head) + middle.slice(0, i) + end;
      await clock.wait(TYPE_MS);
    }
    label.textContent = r.name;
    label.classList.remove("typing");
  }

  // Every group piles up at once, each on its first tile; the grid then closes up around the piles in one move.
  async function pileUp(clock: ReplayClock, piles: HTMLElement[][]): Promise<void> {
    const moves = piles.flatMap((pile) => {
      const anchor = pile[0]?.getBoundingClientRect();
      if (!anchor) return [];
      return pile.map((t, i) => {
        const r = t.getBoundingClientRect();
        t.style.zIndex = String(10 + i);
        return clock.track(t.animate([{ transform: "none" }, { transform: `translate(${anchor.left - r.left + i * 4}px, ${anchor.top - r.top - i * 4}px) scale(0.85)` }], { duration: GATHER_MS, easing: "cubic-bezier(.3,.7,.3,1)", fill: "forwards" }));
      });
    });
    await Promise.all(moves.map(ended));
    if (clock.stopped) return;
    await closeUp(clock, grid, piles.flatMap((p) => p.slice(1)), () => piles.forEach(stack));
  }

  // The pile becomes one tile, with the others tucked behind the first and a count on top.
  function stack(pile: HTMLElement[]): void {
    const [head, ...rest] = pile;
    if (!head) return;
    for (const t of pile) t.getAnimations().forEach((a) => a.cancel());
    head.classList.add("replay-pile");
    rest.forEach((t, i) => {
      t.classList.add("replay-pile-under");
      t.style.transform = `translate(${(i + 1) * 4}px, ${-(i + 1) * 4}px)`;
      t.style.zIndex = String(-1 - i);
      head.append(t);
    });
    if (rest.length > 0) head.append(el("span", { class: "replay-count" }, String(pile.length)));
  }

  // The piles bound for folders in Downloads all fly into them at once, then the grid closes up in one move; a node's pile stays for its trip through the graph.
  async function enterAll(clock: ReplayClock, trips: [HTMLElement, HTMLElement][]): Promise<void> {
    if (trips.length === 0) return;
    const moves = trips.map(([pile, into]) => {
      const from = pile.getBoundingClientRect();
      const to = into.getBoundingClientRect();
      pile.style.zIndex = "20";
      return `translate(${to.left - from.left}px, ${to.top - from.top}px)`;
    });
    await Promise.all(trips.map(([pile], k) => ended(clock.track(pile.animate([{ transform: "none" }, { transform: `${moves[k]!} scale(0.85)` }], { duration: GATHER_MS, easing: "cubic-bezier(.3,.7,.3,1)", fill: "forwards" })))));
    trips.forEach(([, into]) => into.classList.add("replay-receiving"));
    await Promise.all(trips.map(([pile], k) => ended(clock.track(pile.animate([{ transform: `${moves[k]!} scale(0.85)`, opacity: 1 }, { transform: `${moves[k]!} scale(0.2)`, opacity: 0 }], { duration: ENTER_MS, easing: "ease-in", fill: "forwards" })))));
    trips.forEach(([, into]) => into.classList.remove("replay-receiving"));
    if (!clock.stopped) await closeUp(clock, grid, trips.map(([pile]) => pile), () => trips.forEach(([pile]) => pile.remove()));
  }

  return {
    element,
    async rename(clock) {
      await Promise.all(rows.map((r) => retype(clock, r)));
    },
    async addFolders(clock) {
      // New folders join the existing ones, ahead of the files, as the view sorts them.
      const files = [...grid.querySelectorAll<HTMLElement>(".grid-item")].find((t) => t.dataset.key && !folders.has(t.dataset.key.split("/").at(-1) ?? "") && !dirs.has((t.dataset.key.split("/").at(-1) ?? "").toLowerCase()));
      for (const f of fresh) {
        if (clock.stopped) return;
        const tile = folderTile(f);
        tile.classList.add("replay-new");
        folders.set(f, tile);
        grid.insertBefore(tile, files ?? null);
        await clock.wait(180);
      }
      if (fresh.length > 0) await clock.wait(400);
    },
    async file(clock) {
      const groups = new Map<string, HTMLElement[]>();
      for (const r of rows) {
        const g = groupOf(r);
        const t = tiles.get(r);
        if (g && t) groups.set(g, [...(groups.get(g) ?? []), t]);
      }
      await pileUp(clock, [...groups.values()]);
      if (clock.stopped) return;
      const trips = [...groups].flatMap(([g, pile]): [HTMLElement, HTMLElement][] => {
        const into = g.startsWith("@") ? undefined : folders.get(g);
        return into && pile[0] ? [[pile[0], into]] : [];
      });
      await enterAll(clock, trips);
    },
  };
}
