// Deliverable helpers: the delivery date stamp and a picker that keeps the chosen path inside its workboard.
import { open } from "@tauri-apps/plugin-dialog";
import { findNode } from "../services/store/state";
import type { TreeNode } from "../services/ipc/types";

export type Delivery = "file" | "folder" | "all" | "none";
export const DELIVERIES: readonly Delivery[] = ["file", "folder", "all", "none"];
export const isDelivery = (value: string): value is Delivery => (DELIVERIES as readonly string[]).includes(value);

// Folders are named by the local date as yy-mm-dd.
export function todayStamp(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getFullYear() % 100)}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

// The sibling Outcomes folder names where outputs usually live, so the picker starts in that folder of the workboard.
function startPath(node: TreeNode): string {
  const siblings = findNode(node.relParts.slice(0, -1))?.children ?? [];
  const output = siblings
    .find((n) => n.nodeType?.base === "OUT")
    ?.info?.fields.find((f) => f.key === "defaultOutput")?.value;
  const known = output && node.children?.some((c) => !c.file && c.name.toLowerCase() === output.toLowerCase());
  return known ? `${node.absPath}\\${output}` : node.absPath;
}

// The path relative to the workboard, or null when the pick lies outside it; a cancelled dialog gives undefined.
export async function pickInside(node: TreeNode, directory: boolean): Promise<{ relative: string | null } | undefined> {
  const picked = await open({ directory, multiple: false, defaultPath: startPath(node) });
  if (typeof picked !== "string") return undefined;
  const base = node.absPath.replace(/\\/g, "/").replace(/\/+$/, "");
  const path = picked.replace(/\\/g, "/").replace(/\/+$/, "");
  const inside = path.toLowerCase().startsWith(`${base.toLowerCase()}/`);
  return { relative: inside ? path.slice(base.length + 1) : null };
}