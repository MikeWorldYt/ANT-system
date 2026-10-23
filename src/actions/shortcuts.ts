// Every global keyboard shortcut in one table, so adding or changing one never means hunting through the views.
import { selectedNode } from "../store/state";
import { addNext, remove } from "./ops";

export interface ShortcutTargets {
  focusSearch(): void;
  editPath(): void;
  openSelected(): void;
}

interface Binding {
  code: string;
  // Ctrl, or Cmd on macOS and Super elsewhere, whichever the platform reports as meta.
  mod: boolean;
  alt: boolean;
  // Plain keys act only on the folder views, never while typing in a field or pressing a button.
  plain: boolean;
  run(): void;
}

const isField = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));

const onViews = (t: EventTarget | null) => t === document.body || (t instanceof Element && t.closest(".tree-host") !== null);

const overlayOpen = () => document.querySelector('dialog[open], [role="menu"]') !== null;

function bindings(go: ShortcutTargets): Binding[] {
  const withSelected = (fn: (n: NonNullable<ReturnType<typeof selectedNode>>) => void) => () => {
    const n = selectedNode();
    if (n) fn(n);
  };
  return [
    { code: "KeyF", mod: true, alt: false, plain: false, run: go.focusSearch },
    { code: "KeyL", mod: true, alt: false, plain: false, run: go.editPath },
    { code: "KeyN", mod: true, alt: false, plain: false, run: withSelected((n) => n.actions.addNext && void addNext(n, "node")) },
    { code: "KeyN", mod: true, alt: true, plain: false, run: withSelected((n) => n.actions.addNext && void addNext(n, "workboard")) },
    { code: "Delete", mod: false, alt: false, plain: true, run: withSelected((n) => n.actions.delete && void remove(n)) },
    { code: "Enter", mod: false, alt: false, plain: true, run: go.openSelected },
  ];
}

export function installShortcuts(go: ShortcutTargets): void {
  const all = bindings(go);
  document.addEventListener("keydown", (ev) => {
    if (ev.defaultPrevented || ev.shiftKey || overlayOpen()) return;
    const mod = ev.ctrlKey || ev.metaKey;
    const hit = all.find((b) => b.code === ev.code && b.mod === mod && b.alt === ev.altKey);
    if (!hit) return;
    if (hit.plain && (isField(ev.target) || !onViews(ev.target))) return;
    ev.preventDefault();
    hit.run();
  });
}
