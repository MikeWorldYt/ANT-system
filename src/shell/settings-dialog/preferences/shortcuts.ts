// Shortcuts page: shows the editable keys, records a new combination and goes back to the defaults.
import { changeSetting } from "../../../actions/ops";
import { comboOf, conflictWith, EDITABLE_SHORTCUTS } from "../../../actions/shortcuts";
import { state, type KeyCombo } from "../../../store/state";
import { el } from "../../../ui/dom";
import { actions, buttonControl, type RowHandle, type SettingsPage } from "../page";

const MAC = /Mac/i.test(navigator.platform);
const KEY_NAMES: Record<string, string> = { Comma: ",", Period: ".", Slash: "/", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→" };

function keyLabel(code: string): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return KEY_NAMES[code] ?? code;
}

function keys(combo: KeyCombo): HTMLElement {
  const parts = [combo.mod ? (MAC ? "⌘" : "Ctrl") : "", combo.alt ? (MAC ? "⌥" : "Alt") : "", keyLabel(combo.code)].filter(Boolean);
  const out = el("span", { class: "sw-keys" });
  parts.forEach((part, i) => {
    if (i > 0) out.append("+");
    out.append(el("kbd", {}, part));
  });
  return out;
}

const isModifierKey = (code: string) => /^(Control|Shift|Alt|Meta)/.test(code);

function record(id: string, row: RowHandle, done: () => void): void {
  const finish = (message = "") => {
    document.removeEventListener("keydown", onKey, true);
    row.describe(message);
    done();
  };
  const onKey = (ev: KeyboardEvent) => {
    if (isModifierKey(ev.code)) return;
    ev.preventDefault();
    ev.stopPropagation();
    if (ev.code === "Escape") return finish();
    const combo: KeyCombo = { code: ev.code, mod: ev.ctrlKey || ev.metaKey, alt: ev.altKey };
    if (ev.shiftKey || !(combo.mod || combo.alt)) return finish("Use Ctrl or Alt with a key, without Shift.");
    const taken = conflictWith(combo, id);
    if (taken) return finish(`Already used by ${taken}.`);
    const defaults = EDITABLE_SHORTCUTS.find((s) => s.id === id)!.combo;
    const { [id]: _replaced, ...others } = state.settings.shortcuts;
    const isDefault = defaults.code === combo.code && defaults.mod === combo.mod && defaults.alt === combo.alt;
    void changeSetting("shortcuts", isDefault ? others : { ...others, [id]: combo }).then(() => finish());
  };
  row.describe("Press the new keys, or Escape to cancel.");
  document.addEventListener("keydown", onKey, true);
}

export const shortcutsPage: SettingsPage = {
  id: "shortcuts",
  group: "Preferences",
  title: "Shortcuts",
  icon: "pageShortcuts",
  rows: (ctx) => [
    ...EDITABLE_SHORTCUTS.map((s) => ({
      title: s.label,
      control: (row: RowHandle) => actions(keys(comboOf(s.id)), buttonControl("Edit", () => record(s.id, row, ctx.refresh))),
    })),
    {
      title: "Reset shortcuts",
      description: "Go back to the shortcuts ANT ships with.",
      control: () => buttonControl("Reset", () => void changeSetting("shortcuts", {}).then(ctx.refresh)),
    },
  ],
};
