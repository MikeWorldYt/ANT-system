// Hands the selected folder to other apps: the Windows clipboard for Explorer, or a shortcut in a picked folder.
import { open } from "@tauri-apps/plugin-dialog";
import { copyItems, createFileShortcutIn, createShortcutIn } from "../services/ipc/api";
import { setStatus, reportError } from "../components/organisms/bars/status-line";
import type { TreeNode } from "../services/ipc/types";

// Explorer moves a cut folder out of the storage, so only folders that hold loose data may leave.
const cuttable = (node: TreeNode) => node.file || node.flags.includes("data") || node.nodeType?.base === "WKB";

export async function putOnClipboard(nodes: TreeNode[], cut: boolean): Promise<void> {
  if (nodes.length === 0) return;
  if (cut && !nodes.every(cuttable)) return setStatus("warning", "Only data folders and workboards can be cut");
  const label = nodes.length === 1 ? `"${nodes[0].name}"` : `${nodes.length} items`;
  try {
    await copyItems(nodes.map((n) => n.relParts), cut);
    setStatus("info", `${cut ? "Cut" : "Copied"} ${label} to the clipboard`);
  } catch (e) {
    reportError("warning", e);
  }
}

export async function createShortcut(node: TreeNode): Promise<void> {
  try {
    const folder = await open({ directory: true, multiple: false, title: `Create a shortcut to "${node.name}" in` });
    if (folder === null) return;
    const { name, folder: placed } = node.file ? await createFileShortcutIn(node.relParts, folder) : await createShortcutIn(node.relParts, folder);
    setStatus("info", `Created shortcut "${name}" in "${placed}"`);
  } catch (e) {
    reportError("warning", e);
  }
}
