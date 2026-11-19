// A clamped file name is cut in the middle, so its extension stays in sight however long the name is.
import { extOf } from "../molecules/keep-extension";

const ELLIPSIS = "\u2026";

function fit(text: HTMLElement): void {
  const full = text.dataset.full ?? "";
  const ext = extOf(full);
  text.textContent = full;
  if (!ext || text.scrollHeight <= text.clientHeight + 1) return;
  const stem = full.slice(0, -ext.length);
  let [low, high] = [0, stem.length];
  while (low < high) {
    const mid = Math.ceil((low + high) / 2);
    text.textContent = `${stem.slice(0, mid).trimEnd()}${ELLIPSIS}${ext}`;
    if (text.scrollHeight <= text.clientHeight + 1) low = mid;
    else high = mid - 1;
  }
  text.textContent = `${stem.slice(0, low).trimEnd()}${ELLIPSIS}${ext}`;
}

// Measured once laid out and again whenever the tile changes width; a tile taken out of the page by a repaint is let go.
const observer = new ResizeObserver((entries) => {
  for (const e of entries) {
    if (!e.target.isConnected) observer.unobserve(e.target);
    else if (e.target instanceof HTMLElement) fit(e.target);
  }
});

export function keepExtensionInSight(text: HTMLElement, name: string): HTMLElement {
  text.dataset.full = name;
  text.title = name;
  observer.observe(text);
  return text;
}
