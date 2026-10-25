// Extension fields for the selected folder: asks every provider, keeps only the latest timely answer.
import type { TreeNode } from "../ipc/types";
import { keyOf } from "../store/state";
import { setStatus } from "../ui/status";
import { parseFields, summarizeOne, type DetailField } from "./protocol";

export interface DetailsClient {
  name: string;
  send(msg: Record<string, unknown>): void;
  warn(text: string): void;
}

interface Slot {
  client: DetailsClient;
  seq: number;
  key: string | null;
  deadline: number;
  fields: DetailField[];
  warned: boolean;
}

const TIMEOUT_MS = 1500;
const slots = new Map<string, Slot>();
let current: TreeNode | null = null;
let seq = 0;
let changed: (() => void) | null = null;

export const onDetailsChange = (fn: () => void): void => {
  changed = fn;
};

function request(slot: Slot, node: TreeNode): void {
  const key = keyOf(node.relParts);
  // Same folder after a tree refresh keeps its old fields until the new ones arrive, so the rows do not flicker.
  if (slot.key !== key) slot.fields = [];
  slot.seq = ++seq;
  slot.key = key;
  slot.deadline = Date.now() + TIMEOUT_MS;
  slot.client.send({ kind: "details-request", id: slot.seq, node: summarizeOne(node) });
}

export function provide(extId: string, client: DetailsClient): void {
  if (slots.has(extId)) return;
  const slot: Slot = { client, seq: 0, key: null, deadline: 0, fields: [], warned: false };
  slots.set(extId, slot);
  if (current) request(slot, current);
}

export function clearProviders(): void {
  slots.clear();
  changed?.();
}

export function receive(extId: string, id: number, args: unknown): void {
  const slot = slots.get(extId);
  if (!slot || id !== slot.seq || Date.now() > slot.deadline) return;
  slot.deadline = 0;
  const parsed = parseFields(args);
  if (typeof parsed === "string") {
    slot.fields = [];
    slot.client.warn(parsed);
  } else {
    slot.fields = parsed.fields;
    if (parsed.dropped > 0 && !slot.warned) {
      slot.warned = true;
      setStatus("warning", `Extension ${slot.client.name}: ${parsed.dropped} invalid details field(s) were ignored`);
    }
  }
  changed?.();
}

// Requests go out only when the node object changes, because every reply re-renders the panel.
export function detailFields(node: TreeNode | null): { extId: string; name: string; fields: DetailField[] }[] {
  if (node !== current) {
    current = node;
    slots.forEach((slot) => {
      if (node) return request(slot, node);
      slot.fields = [];
      slot.key = null;
    });
  }
  if (!node) return [];
  const key = keyOf(node.relParts);
  return [...slots]
    .filter(([, s]) => s.key === key && s.fields.length > 0)
    .map(([extId, s]) => ({ extId, name: s.client.name, fields: s.fields }));
}

export function runDetailsAction(extId: string, actionId: string, node: TreeNode): void {
  const slot = slots.get(extId);
  if (!slot?.fields.some((f) => f.action?.id === actionId)) return;
  slot.client.send({ kind: "details-action", id: actionId, relParts: [...node.relParts] });
  request(slot, node);
}
