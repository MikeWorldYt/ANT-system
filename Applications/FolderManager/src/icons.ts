// Folder icon per node; git projects get the git icon.
import folderUrl from "./assets/icons/folder.svg?url";
import gitUrl from "./assets/icons/git.svg?url";
import { el } from "./dom";
import type { TreeNode } from "./types";

export const folderIcon = (n: TreeNode) =>
  el("img", { class: "icon", src: n.isGit ? gitUrl : folderUrl, alt: "", width: "16", height: "16", draggable: "false" });
