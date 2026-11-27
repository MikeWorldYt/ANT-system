// Preset swatches plus a last custom one: a native color picker and a HEX field that only saves a valid color.
import { el } from "../dom";
import { isHexColor, normalizeHex, type HexColor } from "../../services/store/hex-color";
import { dotChoices, pressGroup, type Dot } from "./choice-group";

const DEFAULT_CUSTOM: HexColor = "#ff6b9d";
const RAINBOW = "conic-gradient(#f44, #fa0, #ee0, #4c4, #4cf, #84f, #f4a, #f44)";
const CUSTOM = "custom";

export function colorControl<T extends string>(label: string, options: Dot<T>[], value: string, onChange: (value: T | HexColor) => void): HTMLElement {
  let custom: HexColor | null = isHexColor(value) ? value : null;
  const swatch = el("button", { type: "button", title: "Custom color", "aria-label": `${label}: custom color` });
  const picker = el("input", { type: "color", class: "sw-picker", tabindex: "-1", "aria-hidden": "true" });
  const hex = el("input", { type: "text", class: "sw-hex", placeholder: "#RRGGBB", maxlength: "7", spellcheck: "false", "aria-label": `${label} HEX value` });
  const note = el("span", { class: "sw-hex-note", role: "alert" });

  const paint = () => {
    swatch.style.background = custom ?? RAINBOW;
    picker.value = custom ?? DEFAULT_CUSTOM;
    hex.value = custom ?? "";
  };
  paint();

  const choices: [string, HTMLButtonElement][] = [...dotChoices<string>(options), [CUSTOM, swatch]];
  const group = pressGroup<string>("sw-dots", label, choices, custom ? CUSTOM : value, (v) => {
    if (v !== CUSTOM) return onChange(v as T);
    custom ??= DEFAULT_CUSTOM;
    paint();
    onChange(custom);
  });

  const commit = (raw: string) => {
    const next = normalizeHex(raw);
    if (next === null) {
      hex.setAttribute("aria-invalid", "true");
      note.textContent = "Use #RGB or #RRGGBB";
      return;
    }
    hex.removeAttribute("aria-invalid");
    note.textContent = "";
    custom = next;
    paint();
    group.querySelectorAll("button").forEach((b) => b.setAttribute("aria-pressed", String(b === swatch)));
    onChange(next);
  };

  swatch.addEventListener("click", () => {
    try {
      picker.showPicker();
    } catch (e) {
      console.warn("Color picker could not open", e);
      note.textContent = "Type a HEX value instead";
    }
  });
  picker.addEventListener("change", () => commit(picker.value));
  hex.addEventListener("change", () => commit(hex.value));

  const wrap = el("span", { class: "sw-custom" });
  swatch.replaceWith(wrap);
  wrap.append(swatch, picker);
  group.append(hex, note);
  return group;
}
