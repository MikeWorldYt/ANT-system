// Status text that confirms a drop from Explorer, so a successful drop is never silent.
import type { PlacedFile } from "../ipc/types";

const NAME_MAX = 15;

export interface Notice {
  level: "info" | "warning";
  text: string;
}

// Spread by code points so a name never gets cut in the middle of a surrogate pair.
function shortName(name: string): string {
  const chars = [...name];
  return chars.length > NAME_MAX ? `${chars.slice(0, NAME_MAX).join("")}…` : name;
}

function placedText(placed: PlacedFile[]): string {
  const verb = placed[0].copied ? "Copied" : "Moved";
  const folder = placed[0].folder.join("/");
  if (placed.length === 1) return `${verb} "${shortName(placed[0].name)}" to "${folder}"`;
  return `${verb} ${placed.length} files to "${folder}"`;
}

// The status line holds one message, so the skipped files are appended to keep them visible.
export function dropNotice(placed: PlacedFile[], warnings: string[]): Notice | null {
  if (placed.length === 0) return null;
  const text = placedText(placed);
  if (warnings.length === 0) return { level: "info", text };
  return { level: "warning", text: `${text}. ${warnings.join(". ")}` };
}
