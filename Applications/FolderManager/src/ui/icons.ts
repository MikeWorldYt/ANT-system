// Folder icon per node; git projects get the git icon and review links the arrow icon.
import folderSvg from "../assets/icons/folder.svg?raw";
import folderOpenSvg from "../assets/icons/folder-open.svg?raw";
import folderLnkSvg from "../assets/icons/folder-lnk.svg?raw";
import gitSvg from "../assets/icons/git.svg?raw";
import fileUrl from "../assets/icons/file.svg?url";
import { el } from "./dom";
import { nativeImage } from "./file-images";
import type { TreeNode } from "../ipc/types";

// The colors the SVGs were drawn with; the link arrow's blues are not in this list, so they stay.
const DRAWN_BASE = /#7e54d1/gi;
const DRAWN_SHADE = /#7d3de2/gi;

const toUrl = (svg: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;

const shade = (hex: string) => {
  const n = parseInt(hex.slice(1), 16);
  const dark = (c: number) => Math.round(c * 0.78).toString(16).padStart(2, "0");
  return `#${dark(n >> 16)}${dark((n >> 8) & 255)}${dark(n & 255)}`;
};

const paint = (svg: string, base: string | null) => toUrl(base ? svg.replace(DRAWN_BASE, base).replace(DRAWN_SHADE, shade(base)) : svg);

let urls = { folder: "", open: "", lnk: "", git: "" };
let painted: string | null | undefined;

// A null color keeps the icons exactly as drawn.
export function setIconColor(base: string | null): void {
  if (base === painted) return;
  painted = base;
  urls = { folder: paint(folderSvg, base), open: paint(folderOpenSvg, base), lnk: paint(folderLnkSvg, base), git: paint(gitSvg, base) };
}

// Files show the shell's icon for their type, or the generic page where no native icon exists.
export const folderIcon = (n: TreeNode, open = false) => {
  if (painted === undefined) setIconColor(null);
  const src = n.file ? (nativeImage(n, false) ?? fileUrl) : n.linked ? urls.lnk : n.isGit ? urls.git : open ? urls.open : urls.folder;
  return el("img", { class: "icon", src, alt: "", width: "16", height: "16", draggable: "false" });
};
