// Folder icon per node; git projects get the git icon and review links the arrow icon.
import folderUrl from "../assets/icons/folder.svg?url";
import folderOpenUrl from "../assets/icons/folder-open.svg?url";
import folderLnkUrl from "../assets/icons/folder-lnk.svg?url";
import fileUrl from "../assets/icons/file.svg?url";
import gitUrl from "../assets/icons/git.svg?url";
import { el } from "./dom";
import { nativeImage } from "./file-images";
import type { TreeNode } from "../ipc/types";

// Files show the shell's icon for their type, or the generic page where no native icon exists.
export const folderIcon = (n: TreeNode, open = false) => {
  const src = n.file ? (nativeImage(n, false) ?? fileUrl) : n.linked ? folderLnkUrl : n.isGit ? gitUrl : open ? folderOpenUrl : folderUrl;
  return el("img", { class: "icon", src, alt: "", width: "16", height: "16", draggable: "false" });
};
