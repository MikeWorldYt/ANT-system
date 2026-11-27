// Custom accent and icon colors are stored as one normalized lowercase #rrggbb string.
export type HexColor = `#${string}`;

const FULL = /^#[0-9a-f]{6}$/;

export const isHexColor = (value: string | undefined): value is HexColor => FULL.test(value ?? "");

// Accepts #RGB or #RRGGBB, with or without the hash; anything else is null.
export function normalizeHex(input: string): HexColor | null {
  const raw = input.trim().toLowerCase().replace(/^#/, "");
  if (!/^([0-9a-f]{3}|[0-9a-f]{6})$/.test(raw)) return null;
  const full = raw.length === 3 ? [...raw].map((c) => c + c).join("") : raw;
  return `#${full}`;
}
