// Puts the theme, accent color and text size from the settings onto the page.
import { state, type AccentId, type TextSize } from "../services/store/state";
import { setIconColor } from "../components/atoms/icons";

export const ACCENTS: { value: AccentId; label: string; color: string }[] = [
  { value: "green", label: "Green", color: "#00b386" },
  { value: "blue", label: "Blue", color: "#4f8cff" },
  { value: "purple", label: "Purple", color: "#a06cff" },
  { value: "coral", label: "Coral", color: "#ef7a5c" },
  { value: "gold", label: "Gold", color: "#e5b93c" },
];

type Rgb = [number, number, number];
const toRgb = (hex: string): Rgb => {
  const n = parseInt(hex.trim().replace("#", ""), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
};
const toHex = (c: Rgb) => `#${c.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("")}`;
const luminance = (c: Rgb) => {
  const [r, g, b] = c.map((v) => (v / 255 <= 0.03928 ? v / 255 / 12.92 : ((v / 255 + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const contrast = (a: Rgb, b: Rgb) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

// The accent doubles as text on every surface, so it is darkened (light theme) or lightened (dark theme) until it reaches WCAG AA 4.5:1.
// Cached because applyAppearance runs on every state change and reading computed styles forces a style recalc.
const readableCache = new Map<string, string>();
function readableAccent(color: string, light: boolean): string {
  const key = `${color}|${light}`;
  const hit = readableCache.get(key);
  if (hit) return hit;
  const css = getComputedStyle(document.documentElement);
  const surfaces = ["--bg", "--tree-bg", "--input-bg", "--field-bg"].map((v) => toRgb(css.getPropertyValue(v)));
  const target: Rgb = light ? [0, 0, 0] : [255, 255, 255];
  let c = toRgb(color);
  for (let step = 0; step < 20 && surfaces.some((s) => contrast(c, s) < 4.5); step++) c = c.map((v, i) => v + (target[i] - v) * 0.08) as Rgb;
  const out = toHex(c);
  readableCache.set(key, out);
  return out;
}

const onColor = (color: string) => (contrast(toRgb(color), toRgb("#202020")) >= contrast(toRgb(color), toRgb("#ffffff")) ? "#202020" : "#ffffff");

const lightQuery = matchMedia("(prefers-color-scheme: light)");
lightQuery.addEventListener("change", () => {
  if (state.settings.theme === "system") applyAppearance();
});

// The whole interface is scaled, because most sizes in the stylesheet are fixed points.
const TEXT_ZOOM: Record<TextSize, string> = { small: "1", medium: "1.1", large: "1.25" };

export function applyAppearance(): void {
  const root = document.documentElement;
  const { theme, accent, iconColor, textSize } = state.settings;
  const color = ACCENTS.find((a) => a.value === accent)?.color ?? "#00b386";
  root.dataset.theme = theme;
  const readable = readableAccent(color, theme === "light" || (theme === "system" && lightQuery.matches));
  for (const token of ["--title", "--button", "--accent"]) root.style.setProperty(token, readable);
  root.style.setProperty("--on-accent", onColor(readable));
  // Purple is the color the icons were drawn in, so it keeps their original shading.
  setIconColor(iconColor === "purple" ? null : (ACCENTS.find((a) => a.value === iconColor)?.color ?? null));
  root.style.zoom = TEXT_ZOOM[textSize];
}
