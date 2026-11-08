// Side panel preview of the one selected image or video, read through the app's own media scheme.
import { convertFileSrc } from "@tauri-apps/api/core";
import { el } from "../../dom";
import { actedNodes } from "../../../services/store/state";
import { previewImage } from "../../../services/ipc/api";
import { reportError } from "./status-line";
import type { TreeNode } from "../../../services/ipc/types";

const IMAGE = /\.(png|jpe?g|gif|webp|bmp|svg|avif)$/i;
const VIDEO = /\.(mp4|m4v|webm|mov|ogv)$/i;

export interface PreviewPanel {
  element: HTMLElement;
  render(): void;
}

const kindOf = (n: TreeNode | undefined): "image" | "video" | null =>
  !n?.file ? null : IMAGE.test(n.name) ? "image" : VIDEO.test(n.name) ? "video" : null;

export function createPreviewPanel(): PreviewPanel {
  const frame = el("div", { class: "side-preview" });
  const element = el("section", { class: "side-section", "aria-labelledby": "side-preview" }, el("h3", { id: "side-preview", class: "side-label" }, "Preview"), frame);
  let shownKey: string | null = null;

  function render(): void {
    const nodes = actedNodes();
    const n = nodes.length === 1 ? nodes[0] : undefined;
    const kind = kindOf(n);
    element.hidden = kind === null;
    const key = kind && n ? n.relParts.join("/") : null;
    if (key === shownKey) return;
    shownKey = key;
    if (!kind || !n || !key) {
      frame.replaceChildren();
      return;
    }
    if (kind === "image") {
      // A shell thumbnail sized for the panel, not the full picture.
      const img = el("img", { alt: n.name, draggable: "false" });
      frame.replaceChildren(img);
      previewImage(n.relParts)
        .then((url) => {
          if (shownKey === key && url) img.src = url;
        })
        .catch((e: unknown) => reportError("warning", e));
      return;
    }
    // Videos start on their own but muted; the user turns the sound on from the controls.
    const video = el("video", { src: convertFileSrc(key, "fmmedia"), controls: "", loop: "", playsinline: "", preload: "auto" });
    video.muted = true;
    video.autoplay = true;
    frame.replaceChildren(video);
  }

  return { element, render };
}
