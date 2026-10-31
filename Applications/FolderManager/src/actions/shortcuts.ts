// Every global keyboard shortcut in one table, so adding or changing one never means hunting through the views.
import { selectedNode, state, type KeyCombo } from "../store/state";
import { addNext, move, remove, setFavorite } from "./ops";
import { pasteHere } from "./paste-target";
import { putOnClipboard } from "./folder-share";

export interface ShortcutTargets {
  focusSearch(): void;
  editPath(): void;
  openSettings(): void;
  goUp(): void;
  openSelected(): void;
  toggleSidePanel(): void;
  editDetails(): void;
}

interface Binding extends KeyCombo {
  // Plain keys act only on the folder views, never while typing in a field or pressing a button.
  plain: boolean;
  // False keeps the native behaviour of the combo inside text fields.
  inFields: boolean;
  run(): void;
}

// Ctrl, or Cmd on macOS and Super elsewhere, whichever the platform reports as meta.
export const EDITABLE_SHORTCUTS: { id: string; label: string; combo: KeyCombo }[] = [
  { id: "focusSearch", label: "Search in Storage", combo: { code: "KeyF", mod: true, alt: false } },
  { id: "editPath", label: "Edit path", combo: { code: "KeyL", mod: true, alt: false } },
  { id: "newNode", label: "New node", combo: { code: "KeyN", mod: true, alt: false } },
  { id: "newWorkboard", label: "New Workboard", combo: { code: "KeyN", mod: true, alt: true } },
  { id: "openSettings", label: "Open Settings", combo: { code: "Comma", mod: true, alt: false } },
  { id: "goUp", label: "Go up one level", combo: { code: "ArrowUp", mod: false, alt: true } },
  { id: "toggleSidePanel", label: "Collapse/Expand side panel", combo: { code: "ArrowRight", mod: true, alt: false } },
  { id: "moveDown", label: "Move down", combo: { code: "ArrowDown", mod: false, alt: true } },
  { id: "toggleFavorite", label: "Add/remove to favorite", combo: { code: "KeyD", mod: true, alt: false } },
  { id: "editDetails", label: "Edit details", combo: { code: "KeyE", mod: true, alt: false } },
];

const NOT_IN_FIELDS = new Set(["toggleSidePanel", "moveDown", "toggleFavorite", "editDetails"]);

// The folder views own these keys, so the settings page never offers them.
const FIXED_SHORTCUTS: KeyCombo[] = [
  { code: "Delete", mod: false, alt: false },
  { code: "Enter", mod: false, alt: false },
  { code: "KeyO", mod: true, alt: false },
  { code: "KeyV", mod: true, alt: false },
  { code: "KeyC", mod: true, alt: false },
  { code: "KeyX", mod: true, alt: false },
];

export const comboOf = (id: string): KeyCombo =>
  state.settings.shortcuts[id] ?? EDITABLE_SHORTCUTS.find((s) => s.id === id)!.combo;

const same = (a: KeyCombo, b: KeyCombo) => a.code === b.code && a.mod === b.mod && a.alt === b.alt;

// The name of whatever already uses the combo, so the page can say why it was refused.
export function conflictWith(combo: KeyCombo, exceptId: string): string | null {
  if (FIXED_SHORTCUTS.some((f) => same(f, combo))) return "a fixed shortcut";
  return EDITABLE_SHORTCUTS.find((s) => s.id !== exceptId && same(comboOf(s.id), combo))?.label ?? null;
}

export const isField = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));

const onViews = (t: EventTarget | null) => t === document.body || (t instanceof Element && t.closest(".tree-host") !== null);

const overlayOpen = () => document.querySelector('dialog[open], [role="menu"]') !== null;

function bindings(go: ShortcutTargets): Binding[] {
  const withSelected = (fn: (n: NonNullable<ReturnType<typeof selectedNode>>) => void) => () => {
    const n = selectedNode();
    if (n) fn(n);
  };
  const runs: Record<string, () => void> = {
    ...go,
    newNode: withSelected((n) => n.actions.addNext && void addNext(n, "node")),
    newWorkboard: withSelected((n) => n.actions.addNext && n.relParts.length > 1 && void addNext(n, "workboard")),
    moveDown: withSelected((n) => n.actions.moveDown && !state.busy && void move(n, "down")),
    toggleFavorite: withSelected((n) => n.actions.favorite && void setFavorite(n, !n.favorite)),
  };
  return [
    ...EDITABLE_SHORTCUTS.map((s) => ({ ...comboOf(s.id), plain: false, inFields: !NOT_IN_FIELDS.has(s.id), run: runs[s.id]! })),
    { code: "Delete", mod: false, alt: false, plain: true, inFields: false, run: withSelected((n) => n.actions.delete && void remove(n)) },
    { code: "KeyO", mod: true, alt: false, plain: false, inFields: false, run: go.openSelected },
    { code: "KeyV", mod: true, alt: false, plain: false, inFields: false, run: pasteHere },
    { code: "KeyC", mod: true, alt: false, plain: false, inFields: false, run: withSelected((n) => void putOnClipboard(n, false)) },
    { code: "KeyX", mod: true, alt: false, plain: false, inFields: false, run: withSelected((n) => void putOnClipboard(n, true)) },
  ];
}

export function installShortcuts(go: ShortcutTargets): void {
  document.addEventListener("keydown", (ev) => {
    if (ev.defaultPrevented || ev.shiftKey || overlayOpen()) return;
    const mod = ev.ctrlKey || ev.metaKey;
    const hit = bindings(go).find((b) => b.code === ev.code && b.mod === mod && b.alt === ev.altKey);
    if (!hit) return;
    if (hit.plain && !onViews(ev.target)) return;
    if (!hit.inFields && isField(ev.target)) return;
    ev.preventDefault();
    hit.run();
  });
}
