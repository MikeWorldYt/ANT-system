// Hands the selected folder to other apps: the Windows clipboard for Explorer, or a shortcut in a picked folder.
import { open } from "@tauri-apps/plugin-dialog";
import { copyFile, copyFolder, createFileShortcutIn, createShortcutIn, describeError } from "../ipc/api";
import { setStatus } from "../ui/status";
import type { TreeNode } from "../ipc/types";

// Explorer moves a cut folder out of the storage, so only folders that hold loose data may leave.
const cuttable = (node: TreeNode) => node.flags.includes("data") || node.nodeType?.base === "WKB";

// Files cannot be cut, so only the copy reaches the clipboard.
export async function putOnClipboard(node: TreeNode, cut: boolean): Promise<void> {
  if (node.file && cut) return setStatus("warning", "Files can be copied but not cut");
  if (!node.file && cut && !cuttable(node)) return setStatus("warning", "Only data folders and workboards can be cut");
  try {
    if (node.file) await copyFile(node.relParts);
    else await copyFolder(node.relParts, cut);
    setStatus("info", `${cut ? "Cut" : "Copied"} "${node.name}" to the clipboard`);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

export async function createShortcut(node: TreeNode): Promise<void> {
  try {
    const folder = await open({ directory: true, multiple: false, title: `Create a shortcut to "${node.name}" in` });
    if (folder === null) return;
    const { name, folder: placed } = node.file ? await createFileShortcutIn(node.relParts, folder) : await createShortcutIn(node.relParts, folder);
    setStatus("info", `Created shortcut "${name}" in "${placed}"`);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}
