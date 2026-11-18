// Native file images, fetched once per file and size; a repaint shows each one as it arrives.
import { fileImage } from "./ipc/api";
import { keyOf, notify } from "./store/state";
import type { TreeNode } from "./ipc/types";

// Null marks a file with no native image, or one that could not be read; it keeps the generic icon.
const images = new Map<string, string | null>();
const settled = new Set<string>();

// A folder of pictures answers many requests at once; one repaint per burst keeps the views from redrawing for every picture.
const REPAINT_MS = 120;
let repaint: number | undefined;
const repaintSoon = (): void => {
  if (repaint !== undefined) return;
  repaint = window.setTimeout(() => {
    repaint = undefined;
    notify();
  }, REPAINT_MS);
};

export function nativeImage(n: TreeNode, large: boolean): string | null {
  const key = `${keyOf(n.relParts)}|${large ? "large" : "small"}`;
  if (!images.has(key)) {
    images.set(key, null);
    void fileImage(n.relParts, large).then(
      (src) => {
        settled.add(key);
        images.set(key, src);
        // Even a file without a native image repaints, so its loader gives way to the icon.
        repaintSoon();
      },
      (e) => {
        settled.add(key);
        repaintSoon();
        console.warn(`No image for ${n.name}`, e);
      },
    );
  }
  return images.get(key) ?? null;
}

// Asked for and not answered yet: the view shows the loader, not a generic icon that the real picture would replace a moment later.
export const imagePending = (n: TreeNode, large: boolean): boolean => {
  const key = `${keyOf(n.relParts)}|${large ? "large" : "small"}`;
  return images.has(key) && images.get(key) === null && !settled.has(key);
};
