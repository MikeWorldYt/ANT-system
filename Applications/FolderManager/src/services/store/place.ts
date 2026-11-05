// A place is where the user stands, symbolic paths included, so history can store and restore it.
import { state, type VirtualId } from "./state";

export interface Place {
  virtual: VirtualId | null;
  via: string | null;
  key: string | null;
}

export const currentPlace = (): Place => ({ virtual: state.virtual, via: state.via, key: state.selectedKey });

export const samePlace = (a: Place, b: Place): boolean => a.virtual === b.virtual && a.via === b.via && a.key === b.key;

// The folder that lists the selection, null when nothing is; a listed entry's folder is its virtual list.
export function folderOf(p: Place): string | null {
  if (p.key === null) return null;
  if (p.virtual === null) return `/${p.key.split("/").slice(0, -1).join("/")}`;
  return p.key === p.via ? `${p.virtual}:` : `${p.virtual}:${p.key.split("/").slice(0, -1).join("/")}`;
}
