// Typed-path editing for the navigation bar: turns the breadcrumb into an input and goes to the place it names.
import { listChildren, listFavorites } from "../../../services/ipc/api";
import { findNode, keyOf, state, type VirtualId } from "../../../services/store/state";
import { loadPath } from "../../../services/store/virtual";
import type { Place } from "../../../services/store/place";
import { clearStatus, setStatus, reportError } from "./status-line";
import { textInput } from "../../atoms/text-input";
import type { TreeNode } from "../../../services/ipc/types";

const VIRTUAL_IDS: VirtualId[] = ["favorites", "board"];

const cleaned = (text: string): string => text.trim().replace(/^"+|"+$/g, "");
const splitPath = (text: string): string[] => cleaned(text).split(/[\\/]/).filter(Boolean);

export const rootName = (root: string | undefined): string => splitPath(root ?? "").pop() ?? "";

const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

// Canonical names of the typed folders below `level`, loading each one on the way; null when one is missing.
async function walk(level: TreeNode[], typed: string[]): Promise<string[] | null> {
  const found: string[] = [];
  for (const part of typed) {
    const node = level.find((n) => same(n.name, part));
    if (!node) return null;
    found.push(node.name);
    if (node.children === null && node.hasChildren) node.children = await listChildren(node.relParts);
    level = node.children ?? [];
  }
  return found;
}

// `/favorites/<favorite>/...` and `/board` name the virtual root entries, which live in no folder on disk.
async function resolveVirtual(id: VirtualId, rest: string[]): Promise<Place | null> {
  const [name, ...below] = rest;
  if (name === undefined) return { virtual: id, via: null, key: null };
  if (id === "board") return null;
  const favorite = (await listFavorites()).find((p) => same(p.at(-1) ?? "", name));
  if (!favorite) return null;
  await loadPath(favorite);
  const node = findNode(favorite);
  const found = node ? await walk([node], [node.name, ...below]) : null;
  if (found === null) return null;
  return { virtual: id, via: keyOf(favorite), key: keyOf([...favorite.slice(0, -1), ...found]) };
}

// Maps a typed path to an existing place: inside the storage or a virtual entry; undefined when outside the storage.
async function resolveTyped(text: string): Promise<Place | null | undefined> {
  const typed = splitPath(text);
  const id = VIRTUAL_IDS.find((v) => same(v, typed[0] ?? ""));
  if (id && /^[\\/]/.test(cleaned(text))) return resolveVirtual(id, typed.slice(1));
  const root = splitPath(state.tree?.root ?? "");
  if (typed.includes("..") || typed.length < root.length || !root.every((p, i) => same(p, typed[i] ?? ""))) return undefined;
  const found = await walk(state.tree?.nodes ?? [], typed.slice(root.length));
  return found && { virtual: null, via: null, key: found.length > 0 ? keyOf(found) : null };
}

// Inside a virtual entry the typed form is its symbolic path, so Enter comes back to the same place.
function currentPath(root: string): string {
  const parts = state.selectedKey?.split("/") ?? [];
  if (state.virtual === null) return [root.replace(/[\\/]+$/, ""), ...parts].join("\\");
  const below = state.via === null ? [] : parts.slice(state.via.split("/").length - 1);
  return ["", state.virtual.toUpperCase(), ...below].join("\\");
}

export function createPathInput(path: HTMLElement, render: () => void, visit: (place: Place) => Promise<void>): { editing(): boolean; start(): void } {
  let editing = false;

  function stopEditing(): void {
    editing = false;
    path.classList.remove("editing");
    render();
  }

  function start(): void {
    if (editing || !state.tree) return;
    editing = true;
    path.classList.add("editing");
    const input = textInput({ class: "path-input", "aria-label": "Path" });
    input.value = currentPath(state.tree.root);
    input.addEventListener("keydown", (ev) => {
      if (ev.key === "Escape") {
        ev.stopPropagation();
        clearStatus();
        stopEditing();
      } else if (ev.key === "Enter") {
        ev.preventDefault();
        void submit(input);
      }
    });
    input.addEventListener("input", () => input.removeAttribute("aria-invalid"));
    input.addEventListener("blur", () => {
      if (editing) stopEditing();
    });
    path.replaceChildren(input);
    input.focus();
    input.select();
  }

  async function submit(input: HTMLInputElement): Promise<void> {
    let target: Place | null | undefined;
    try {
      target = await resolveTyped(input.value);
    } catch (e) {
      target = null;
      console.warn("Path lookup failed", e);
    }
    if (target === undefined) {
      input.removeAttribute("aria-invalid");
      input.value = currentPath(state.tree?.root ?? "");
      input.select();
      setStatus("error", "You can only browse inside your storage");
      return;
    }
    if (target === null) {
      input.setAttribute("aria-invalid", "true");
      setStatus("error", "That folder does not exist in the storage");
      return;
    }
    clearStatus();
    editing = false;
    path.classList.remove("editing");
    try {
      await visit(target);
    } catch (e) {
      reportError("error", e);
    }
    render();
  }

  return { editing: () => editing, start };
}
