// Folder icon per node: link, locked, workboard and git variants, and a barred folder for nodes that do not take files.
import folderSvg from "../../assets/icons/folder.svg?raw";
import folderNoFlagSvg from "../../assets/icons/folder-noflag.svg?raw";
import folderOpenSvg from "../../assets/icons/folder-open.svg?raw";
import folderOpenNoFlagSvg from "../../assets/icons/folder-open-noflag.svg?raw";
import folderLnkSvg from "../../assets/icons/folder-lnk.svg?raw";
import folderLockedSvg from "../../assets/icons/folder-locked.svg?raw";
import folderWkbSvg from "../../assets/icons/folder-wkb.svg?raw";
import folderWkbGitSvg from "../../assets/icons/folder-wkb-git.svg?raw";
import gitSvg from "../../assets/icons/git.svg?raw";
import recordingsSvg from "../../assets/gui/recordings.svg?raw";
import screenshotsSvg from "../../assets/gui/screenshots.svg?raw";
import fileUrl from "../../assets/icons/file.svg?url";
import { el } from "../dom";
import { nativeImage } from "../../services/file-images";
import { isWorkboard } from "../../services/store/node-info";
import { inDownloads, type TreeNode } from "../../services/ipc/types";

// The colors the SVGs were drawn with; the link arrow's blues are not in this list, so they stay.
const DRAWN_BASE = /#7e54d1/gi;
const DRAWN_SHADE = /#7d3de2/gi;

const toUrl = (svg: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;

const shade = (hex: string) => {
  const n = parseInt(hex.slice(1), 16);
  const dark = (c: number) => Math.round(c * 0.78).toString(16).padStart(2, "0");
  return `#${dark(n >> 16)}${dark((n >> 8) & 255)}${dark(n & 255)}`;
};

const LOCKED_LABEL = "System folder, locked";

const SVGS = { folder: folderSvg, noflag: folderNoFlagSvg, open: folderOpenSvg, openNoflag: folderOpenNoFlagSvg, lnk: folderLnkSvg, locked: folderLockedSvg, wkb: folderWkbSvg, wkbGit: folderWkbGitSvg, git: gitSvg };
type Kind = keyof typeof SVGS;

const urls = new Map<string, string>();
let painted: string | null | undefined;

const paint = (svg: string, base: string | null) => (base ? svg.replace(DRAWN_BASE, base).replace(DRAWN_SHADE, shade(base)) : svg);

const urlOf = (kind: Kind) => {
  let url = urls.get(kind);
  if (!url) {
    url = toUrl(paint(SVGS[kind], painted ?? null));
    urls.set(kind, url);
  }
  return url;
};

// A null color keeps the icons exactly as drawn.
export function setIconColor(base: string | null): void {
  if (base === painted) return;
  painted = base;
  urls.clear();
}

// A folder in Downloads follows no storage rule, so it never gets the barred icon of a node that takes no files.
const kindOf = (n: TreeNode, open: boolean): Kind =>
  inDownloads(n.relParts) ? (open ? "open" : "folder") : n.linked ? "lnk" : n.locked ? "locked" : isWorkboard(n) ? (n.isGit ? "wkbGit" : "wkb") : n.isGit ? "git" : n.flags.includes("data") ? (open ? "open" : "folder") : open ? "openNoflag" : "noflag";

// The global media folders at the root show the same icons as their side panel shortcuts.
const MEDIA_URLS: Record<string, string> = { REC: `data:image/svg+xml,${encodeURIComponent(recordingsSvg)}`, SS: `data:image/svg+xml,${encodeURIComponent(screenshotsSvg)}` };

// Files show the shell's icon for their type, or the generic page where no native icon exists.
export const folderIcon = (n: TreeNode, open = false) => {
  if (painted === undefined) setIconColor(null);
  // Shortcuts get the review-link arrow, which reads better than the shell's blank page for a .lnk.
  const shortcut = n.file && n.name.toLowerCase().endsWith(".lnk");
  const media = !n.file && n.relParts.length === 1 ? MEDIA_URLS[n.nodeType?.base ?? ""] : undefined;
  const src = media ?? (shortcut ? urlOf("lnk") : n.file ? (nativeImage(n, false) ?? fileUrl) : urlOf(kindOf(n, open)));
  const locked = !n.file && !n.linked && n.locked;
  const img = el("img", { class: "icon", src, alt: locked ? LOCKED_LABEL : "", width: "16", height: "16", draggable: "false" });
  if (locked) img.title = LOCKED_LABEL;
  return img;
};

// A search hit past the loaded levels has only its name, so the icon is guessed from the naming convention.
export const nameIcon = (name: string) =>
  el("img", { class: "icon", src: urlOf(/^[A-Z]{2}\.\d+_/.test(name) ? "wkb" : "folder"), alt: "", width: "16", height: "16", draggable: "false" });

// Folders that do not exist yet, such as the projects a sort would make, have only their kind.
export const kindIcon = (kind: "wkb" | "folder") => el("img", { class: "icon", src: urlOf(kind), alt: "", width: "16", height: "16", draggable: "false" });
