// The ids a reorder by id hands out, mirrored from the backend so the dialog can show and check them before saving.
import { keyOf, state } from "./state";
import type { IdKey } from "../ipc/types";

export type IdKind = "node" | "worknode";

const TRACK = /^((?:PJ|PR|HW|CR|TK)\.)(\d+)/;
const NODE_ID = /^(\d+)(?=\.)/;

export const isTracked = (name: string): boolean => TRACK.test(name);

export const trackerOf = (name: string, kind: IdKind): string => (kind === "worknode" ? (TRACK.exec(name)?.[1] ?? "") : "");

// The id as written: "05" of "05.Math", "001" of "PJ.001_Web".
export const idOf = (name: string, kind: IdKind): string => (kind === "worknode" ? TRACK.exec(name)?.[2] : NODE_ID.exec(name)?.[1]) ?? "";

// The name with its id, and for a worknode its tracker, replaced; the title stays.
export const withId = (name: string, kind: IdKind, id: string, tracker = trackerOf(name, kind)): string =>
  tracker + id + name.slice(trackerOf(name, kind).length + idOf(name, kind).length);

// A node id pads to 2 digits, a worknode number to at least 3; a main category keeps its plain number.
export const padId = (digits: string, kind: IdKind, root: boolean): string =>
  root && kind === "node" ? String(Number(digits)) : String(Number(digits)).padStart(kind === "worknode" ? 3 : 2, "0");

// Why digits cannot be an id, or null.
export function idProblem(digits: string, kind: IdKind, root: boolean): string | null {
  if (digits === "") return "Type an id";
  const n = Number(digits);
  if (kind === "worknode") return n === 0 ? "Worknode numbers start at 1" : null;
  if (root) return n >= 100 && n <= 800 && n % 100 === 0 ? null : "Main categories use 100 to 800, in steps of 100";
  if (n === 0) return "00 is reserved for Outcomes";
  return n > 99 ? "Node ids go up to 99" : null;
}

// Same key as the backend: the tracker, empty for a node, and the number.
export const keyFor = (tracker: string, digits: string): string => `${tracker}${Number(digits)}`;

export const takenKey = (key: IdKey): string => `${key.tracker}${key.number}`;

// Stale paths are followed by id, which after a reorder belongs to another child, so the keys are renamed first.
export function followReorder(parent: string[], renames: Map<string, string>): void {
  const at = parent.length;
  const parentKey = keyOf(parent);
  const move = (key: string): string => {
    const parts = key.split("/");
    const next = parts.length > at && keyOf(parts.slice(0, at)) === parentKey ? renames.get(parts[at] ?? "") : undefined;
    if (next === undefined) return key;
    parts[at] = next;
    return keyOf(parts);
  };
  state.expanded = new Set([...state.expanded].map(move));
  state.selectedKeys = state.selectedKeys.map(move);
  if (state.selectedKey !== null) state.selectedKey = move(state.selectedKey);
  if (state.via !== null) state.via = move(state.via);
}
