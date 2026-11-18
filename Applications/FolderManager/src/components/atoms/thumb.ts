// A thumbnail shows the loader in its place until the picture arrives, so a folder of pictures keeps its layout while they load.
import { el } from "../dom";
import loader from "../../assets/gui/loader.svg?raw";
import { imagePending, nativeImage } from "../../services/file-images";
import type { TreeNode } from "../../services/ipc/types";

export function loaderMark(): HTMLElement {
  const mark = el("span", { class: "loader", role: "progressbar", "aria-label": "Loading" });
  mark.innerHTML = loader;
  return mark;
}

// A file's picture once it is in, the loader while it is on its way, and the fallback icon when there is none.
export function fileThumb(n: TreeNode, fallback: () => Element, attrs: Record<string, string> = {}): Element {
  const src = nativeImage(n, true);
  if (src) return thumb(src, attrs);
  return imagePending(n, true) ? loaderMark() : fallback();
}

export function thumb(src: string, attrs: Record<string, string> = {}): HTMLElement {
  const img = el("img", { class: "thumb", src, alt: "", ...attrs });
  const wrap = el("span", { class: "thumb-wrap loading" }, loaderMark(), img);
  const done = (): void => {
    wrap.classList.remove("loading");
    wrap.querySelector(".loader")?.remove();
  };
  if (img.complete && img.naturalWidth > 0) done();
  img.addEventListener("load", done, { once: true });
  img.addEventListener("error", done, { once: true });
  return wrap;
}
