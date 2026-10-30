// Hands the selected folder to other apps: the Windows clipboard for Explorer, or a shortcut in a picked folder.
import { open } from "@tauri-apps/plugin-dialog";
import { copyFolder, createShortcutIn, describeError } from "../ipc/api";
import { setStatus } from "../ui/status";
import type { TreeNode } from "../ipc/types";

// Explorer moves a cut folder out of the storage, so only folders that hold loose data may leave.
const cuttable = (node: TreeNode) => node.flags.includes("data") || node.nodeType?.base === "WKB";

export async function putOnClipboard(node: TreeNode, cut: boolean): Promise<void> {
  if (cut && !cuttable(node)) return setStatus("warning", "Only data folders and workboards can be cut");
  try {
    await copyFolder(node.relParts, cut);
    setStatus("info", `${cut ? "Cut" : "Copied"} "${node.name}" to the clipboard`);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

export async function createShortcut(node: TreeNode): Promise<void> {
  try {
    const folder = await open({ directory: true, multiple: false, title: `Create a shortcut to "${node.name}" in` });
    if (folder === null) return;
    const { name, folder: placed } = await createShortcutIn(node.relParts, folder);
    setStatus("info", `Created shortcut "${name}" in "${placed}"`);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}
