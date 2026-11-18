// How a key combination reads on screen, shared by the Shortcuts page and the tooltips.
import type { KeyCombo } from "./store/state";

const MAC = /Mac/i.test(navigator.platform);
const KEY_NAMES: Record<string, string> = { Comma: ",", Period: ".", Slash: "/", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→" };

export function keyLabel(code: string): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return KEY_NAMES[code] ?? code;
}

export const comboKeys = (combo: KeyCombo): string[] => [combo.mod ? (MAC ? "⌘" : "Ctrl") : "", combo.alt ? (MAC ? "⌥" : "Alt") : "", keyLabel(combo.code)].filter(Boolean);
