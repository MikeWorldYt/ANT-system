// Side panel preview of the one selected image, PDF, video or audio file, read through the app's own media scheme.
import { convertFileSrc } from "@tauri-apps/api/core";
import { el } from "../../dom";
import { actedNodes } from "../../../services/store/state";
import { previewImage } from "../../../services/ipc/api";
import { reportError } from "./status-line";
import { SELF_DRAWN } from "../../../services/file-images";
import { pdfPages } from "./pdf-pages";
import type { TreeNode } from "../../../services/ipc/types";

const IMAGE = /\.(png|jpe?g|jfif|gif|webp|bmp|svg|avif)$/i;
const VIDEO = /\.(mp4|m4v|webm|mov|ogv)$/i;
const AUDIO = /\.(mp3|wav|m4a|aac|ogg|oga|flac)$/i;
// Sound turned on stays on for the next previews this long, then they start muted again.
const SOUND_MS = 5 * 60 * 1000;
let soundUntil = 0;

export interface PreviewPanel {
  element: HTMLElement;
  render(): void;
}

type Kind = "image" | "pdf" | "video" | "audio";

function kindOf(n: TreeNode | undefined): Kind | null {
  if (!n?.file) return null;
  if (IMAGE.test(n.name)) return "image";
  if (/\.pdf$/i.test(n.name)) return "pdf";
  if (VIDEO.test(n.name)) return "video";
  return AUDIO.test(n.name) ? "audio" : null;
}

// Starts muted unless the user turned the sound on in the last minutes; turning it on or off sets that window.
function withSound(media: HTMLMediaElement): void {
  media.muted = Date.now() > soundUntil;
  media.addEventListener("volumechange", () => {
    soundUntil = media.muted ? 0 : Date.now() + SOUND_MS;
  });
}

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
      if (SELF_DRAWN.test(n.name)) {
        img.src = convertFileSrc(key, "fmmedia");
        return;
      }
      previewImage(n.relParts)
        .then((url) => {
          if (shownKey === key && url) img.src = url;
        })
        .catch((e: unknown) => reportError("warning", e));
      return;
    }
    if (kind === "pdf") {
      frame.replaceChildren(pdfPages(n, () => shownKey === key));
      return;
    }
    if (kind === "audio") {
      const audio = el("audio", { src: convertFileSrc(key, "fmmedia"), controls: "", preload: "auto" });
      withSound(audio);
      audio.autoplay = true;
      frame.replaceChildren(audio);
      return;
    }
    // Videos start on their own, muted unless the user recently turned the sound on.
    const video = el("video", { src: convertFileSrc(key, "fmmedia"), controls: "", loop: "", playsinline: "", preload: "auto" });
    withSound(video);
    video.autoplay = true;
    frame.replaceChildren(video);
    // Sound but no picture means Windows lacks the video's codec, so a frame drawn by the app stands in, with the controls kept below it.
    video.addEventListener("loadedmetadata", () => {
      if (video.videoWidth > 0) return;
      previewImage(n.relParts)
        .then((url) => {
          if (shownKey !== key || !url) return;
          video.poster = url;
          frame.replaceChildren(el("img", { src: url, alt: n.name, draggable: "false" }), video, el("p", { class: "side-preview-note" }, "This video's codec is not installed in Windows; showing a still frame."));
        })
        .catch((e: unknown) => reportError("warning", e));
    }, { once: true });
  }

  return { element, render };
}
