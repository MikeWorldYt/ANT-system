// What the native folders (Downloads and the rest) allow, kept in one place so views, drags and menus refuse the same things.
import { inSystem, systemName, systemRootOf, type TreeNode } from "../ipc/types";

// They are not the storage, so nothing can be dropped, pasted or moved into them.
export const takesFiles = (parts: string[]): boolean => !inSystem(parts);

export const noFilesMessage = (parts: string[]): string => `${systemName(parts)} takes no files from here`;

// Their items reach other apps through the clipboard, not through the storage's drag-out.
export const canDragOut = (parts: string[]): boolean => !inSystem(parts);

// The key of the folder itself, not of anything inside it.
export const isSystemRoot = (key: string): boolean => systemRootOf([key]) === key;

export const allInSystem = (nodes: Pick<TreeNode, "relParts">[]): boolean => nodes.length > 0 && nodes.every((n) => inSystem(n.relParts));
