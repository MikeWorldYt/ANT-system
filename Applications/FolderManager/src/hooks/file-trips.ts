// Files that left one folder and reached another, whoever moved them, become trips the graph draws as packets; a file from outside the Storage starts at its root.
import { keyOf } from "../services/store/state";
import type { TreeNode } from "../services/ipc/types";

export interface Trip {
  // Null for a file that came from outside the Storage.
  from: string[] | null;
  to: string[];
}

interface Seen {
  name: string;
  size: number | null;
  at: string[];
  time: number;
}

// Windows reports a move between folders as a removal and an addition, often read in separate passes, so both are held a moment before they are paired.
const SETTLE_MS = 400;
const PAIR_MS = 2_000;
// A trip the app announced itself is not drawn again when the same files show up in the listing.
const OWN_MS = 3_000;

let departures: Seen[] = [];
let arrivals: Seen[] = [];
let own: { to: string; until: number }[] = [];
let timer: number | undefined;
let hushedUntil = 0;

// A batch draws its own replay, so the listings it changes add no packets of their own for a while.
export function hush(ms: number): void {
  hushedUntil = Date.now() + ms;
}

const seen = (n: TreeNode, at: string[]): Seen => ({ name: n.name.toLowerCase(), size: n.stamps.size, at, time: Date.now() });

export function noteListing(parent: string[], before: TreeNode[], after: TreeNode[]): void {
  const was = new Set(before.filter((n) => n.file).map((n) => n.name));
  const now = new Set(after.filter((n) => n.file).map((n) => n.name));
  // Temporary and hidden files come and go as programs save; they are no move.
  const real = (n: TreeNode) => n.file && !n.concealed && !n.name.startsWith("~");
  const left = before.filter((n) => real(n) && !now.has(n.name));
  const came = after.filter((n) => real(n) && !was.has(n.name));
  if ((left.length === 0 && came.length === 0) || Date.now() < hushedUntil) return;
  departures.push(...left.map((n) => seen(n, parent)));
  arrivals.push(...came.map((n) => seen(n, parent)));
  schedule();
}

export function noteTrip(trip: Trip): void {
  own.push({ to: keyOf(trip.to), until: Date.now() + OWN_MS });
  dispatch([trip]);
}

function schedule(): void {
  window.clearTimeout(timer);
  timer = window.setTimeout(flush, SETTLE_MS);
}

function flush(): void {
  const now = Date.now();
  own = own.filter((o) => o.until > now);
  const trips = new Map<string, Trip>();
  for (const a of arrivals) {
    const i = departures.findIndex((d) => d.name === a.name && (d.size === null || a.size === null || d.size === a.size) && keyOf(d.at) !== keyOf(a.at));
    const from = i >= 0 ? departures.splice(i, 1)[0]!.at : null;
    if (from === null && own.some((o) => o.to === keyOf(a.at))) continue;
    // A name that went and another that came in the same folder is a rename, which goes nowhere.
    if (from === null) {
      const renamed = departures.findIndex((d) => keyOf(d.at) === keyOf(a.at));
      if (renamed >= 0) {
        departures.splice(renamed, 1);
        continue;
      }
    }
    // Many files on the same route travel as one packet.
    trips.set(`${from === null ? "*" : keyOf(from)}|${keyOf(a.at)}`, { from, to: a.at });
  }
  arrivals = [];
  departures = departures.filter((d) => now - d.time < PAIR_MS);
  if (trips.size > 0) dispatch([...trips.values()]);
}

const dispatch = (trips: Trip[]): void => void window.dispatchEvent(new CustomEvent("fm:file-trips", { detail: trips }));
