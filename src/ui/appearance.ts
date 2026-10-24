// Puts the theme, accent color and text size from the settings onto the page.
import { state, type AccentId, type TextSize } from "../store/state";

export const ACCENTS: { value: AccentId; label: string; color: string }[] = [
  { value: "green", label: "Green", color: "#00b386" },
  { value: "blue", label: "Blue", color: "#4f8cff" },
  { value: "purple", label: "Purple", color: "#a06cff" },
  { value: "coral", label: "Coral", color: "#ef7a5c" },
  { value: "gold", label: "Gold", color: "#e5b93c" },
];

// The whole interface is scaled, because most sizes in the stylesheet are fixed points.
const TEXT_ZOOM: Record<TextSize, string> = { small: "1", medium: "1.1", large: "1.25" };

export function applyAppearance(): void {
  const root = document.documentElement;
  const { theme, accent, textSize } = state.settings;
  const color = ACCENTS.find((a) => a.value === accent)?.color ?? "#00b386";
  root.dataset.theme = theme;
  root.style.setProperty("--title", color);
  root.style.setProperty("--button", color);
  root.style.zoom = TEXT_ZOOM[textSize];
}
