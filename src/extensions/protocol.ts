// Message shapes between a sandboxed extension and the host, and the checks every request passes.
import type { ExtensionPermission } from "../ipc/api";
import type { TreeNode } from "../ipc/types";

export const MAX_MESSAGE = 64 * 1024;
export const MAX_MENU_ITEMS = 20;
const MAX_LABEL = 40;
const MENU_ID = /^[a-z0-9-]{1,30}$/;

export type Op = "tree.get" | "menu.add" | "panel.show" | "panel.hide";

export const NEEDS: Record<Op, ExtensionPermission> = {
  "tree.get": "tree:read",
  "menu.add": "menu:context",
  "panel.show": "panel:own",
  "panel.hide": "panel:own",
};

export const PERMISSION_TEXT: Record<ExtensionPermission, string> = {
  "tree:read": "Read your folders' names, types and statuses",
  "menu:context": "Add items to the context menu",
  "panel:own": "Show its own panel",
};

export const describePermission = (p: string): string => PERMISSION_TEXT[p as ExtensionPermission] ?? p;

export type Incoming = { kind: "call"; id: number; op: Op; args: unknown } | { kind: "error"; message: string };

export interface NodeSummary {
  name: string;
  relParts: string[];
  depth: number;
  type: string | null;
  status: string | null;
  favorite: boolean;
  children: NodeSummary[] | null;
}

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null;

// Structured clone accepts cycles and huge payloads, so the size is measured before anything is trusted.
function sizeOf(data: unknown): number {
  try {
    return JSON.stringify(data)?.length ?? 0;
  } catch {
    return Infinity;
  }
}

export function parseIncoming(data: unknown): Incoming | string {
  if (sizeOf(data) > MAX_MESSAGE) return "message too large";
  if (!isRecord(data) || data.ant !== 1) return "malformed message";
  if (data.kind === "error") return { kind: "error", message: String(data.message).slice(0, 300) };
  if (data.kind !== "call" || !Number.isSafeInteger(data.id)) return "malformed message";
  if (typeof data.op !== "string" || !Object.hasOwn(NEEDS, data.op)) return `unknown call ${String(data.op).slice(0, 40)}`;
  return { kind: "call", id: data.id as number, op: data.op as Op, args: data.args };
}

export function parseMenuItem(args: unknown): { id: string; label: string } | string {
  if (!isRecord(args) || typeof args.id !== "string" || typeof args.label !== "string") return "menu.add expects { id, label }";
  if (!MENU_ID.test(args.id)) return "menu id must match [a-z0-9-]{1,30}";
  const label = args.label.replace(/\s+/g, " ").trim();
  if (label === "" || label.length > MAX_LABEL) return `menu label must be 1-${MAX_LABEL} characters`;
  return { id: args.id, label };
}

export function summarize(nodes: TreeNode[]): NodeSummary[] {
  return nodes.map((n) => ({
    name: n.name,
    relParts: [...n.relParts],
    depth: n.depth,
    type: n.nodeType?.base ?? null,
    status: n.status,
    favorite: n.favorite,
    children: n.children ? summarize(n.children) : null,
  }));
}
