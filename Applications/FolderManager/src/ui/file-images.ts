// Native file images, fetched once per file and size; a repaint shows each one as it arrives.
import { fileImage } from "../ipc/api";
import { keyOf, notify } from "../store/state";
import type { TreeNode } from "../ipc/types";

// Null marks a file with no native image, or one that could not be read; it keeps the generic icon.
const images = new Map<string, string | null>();

export function nativeImage(n: TreeNode, large: boolean): string | null {
  const key = `${keyOf(n.relParts)}|${large ? "large" : "small"}`;
  if (!images.has(key)) {
    images.set(key, null);
    void fileImage(n.relParts, large).then(
      (src) => {
        images.set(key, src);
        // A file without a native image needs no repaint.
        if (src) notify();
      },
      (e) => console.warn(`No image for ${n.name}`, e),
    );
  }
  return images.get(key) ?? null;
}
