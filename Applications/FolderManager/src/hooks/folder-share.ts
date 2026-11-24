// Hands the selected folder to other apps: the Windows clipboard for Explorer, or a shortcut in a picked folder.
import { open } from "@tauri-apps/plugin-dialog";
import { copyItems, createFileShortcutIn, createShortcutIn } from "../services/ipc/api";
import { setStatus, reportError } from "../components/organisms/bars/status-line";
import { inSystem, systemName, type TreeNode } from "../services/ipc/types";

// Explorer moves a cut folder out of the storage, so only folders that hold loose data may leave; Downloads is not the storage, so anything there can be cut.
const cuttable = (node: TreeNode) => node.file || node.flags.includes("data") || node.nodeType?.base === "WKB";

export async function putOnClipboard(nodes: TreeNode[], cut: boolean): Promise<void> {
  if (nodes.length === 0) return;
  const native = nodes.filter((n) => inSystem(n.relParts));
  if (native.length > 0 && native.length < nodes.length) return setStatus("warning", `Copy items from ${systemName(native[0]?.relParts ?? [])} and from the Storage separately`);
  if (new Set(native.map((n) => n.relParts[0])).size > 1) return setStatus("warning", "Copy items from each system folder separately");
  if (cut && native.length === 0 && !nodes.every(cuttable)) return setStatus("warning", "Only data folders and worknodes can be cut");
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
