// Folder icon per node; git projects get the git icon.
import folderUrl from "../assets/icons/folder.svg?url";
import folderOpenUrl from "../assets/icons/folder-open.svg?url";
import gitUrl from "../assets/icons/git.svg?url";
import { el } from "./dom";
import type { TreeNode } from "../ipc/types";

export const folderIcon = (n: TreeNode, open = false) =>
  el("img", { class: "icon", src: n.isGit ? gitUrl : open ? folderOpenUrl : folderUrl, alt: "", width: "16", height: "16", draggable: "false" });
