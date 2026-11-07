// Starts the enabled extensions, answers their permission-checked requests and owns their menu items.
import { listExtensions, readExtensionEntry, setExtensionEnabled, type ExtensionInfo } from "../services/ipc/api";
import type { TreeNode } from "../services/ipc/types";
import { state } from "../services/store/state";
import { setStatus, reportError } from "../components/organisms/bars/status-line";
import { warningConfirmDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { clearProviders, provide, receive } from "./details";
import { addSection, type PanelSection } from "./panel";
import { describePermission, MAX_MENU_ITEMS, NEEDS, parseIncoming, parseMenuItem, summarize, type Op } from "./protocol";
import { createSandbox } from "./sandbox";

interface Running {
  info: ExtensionInfo;
  frame: HTMLIFrameElement;
  panel: PanelSection;
  menu: Map<string, string>;
  warned: boolean;
}

const running = new Map<string, Running>();
// A newer reload makes the older one stop starting extensions halfway through.
let generation = 0;
// Each id is asked about once per run; a refusal stays off and can be changed in Settings.
const asked = new Set<string>();

function warn(ext: Running, text: string): void {
  if (ext.warned) return;
  ext.warned = true;
  setStatus("warning", `Extension ${ext.info.name}: ${text}`);
}

const post = (ext: Running, msg: Record<string, unknown>) => ext.frame.contentWindow?.postMessage({ ant: 1, ...msg }, "*");

function handle(ext: Running, op: Exclude<Op, "details.reply">, args: unknown): unknown {
  switch (op) {
    case "tree.get":
      return summarize(state.tree?.nodes ?? []);
    case "menu.add": {
      const item = parseMenuItem(args);
      if (typeof item === "string") throw new Error(item);
      if (!ext.menu.has(item.id) && ext.menu.size >= MAX_MENU_ITEMS) throw new Error(`At most ${MAX_MENU_ITEMS} menu items`);
      ext.menu.set(item.id, item.label);
      return null;
    }
    case "details.provide":
      provide(ext.info.id, { name: ext.info.name, send: (msg) => post(ext, msg), warn: (text) => warn(ext, text) });
      return null;
    case "panel.show":
    case "panel.hide":
      ext.panel.show(op === "panel.show");
      return null;
  }
}

function onMessage(ev: MessageEvent): void {
  const ext = [...running.values()].find((r) => r.frame.contentWindow !== null && ev.source === r.frame.contentWindow);
  if (!ext) return;
  const msg = parseIncoming(ev.data);
  if (typeof msg === "string") return warn(ext, msg);
  if (msg.kind === "error") return warn(ext, msg.message);
  const op = msg.op;
  const allowed = ext.info.permissions.includes(NEEDS[op]);
  // A details reply answers a host request, so it never gets a reply of its own.
  if (op === "details.reply") return allowed ? receive(ext.info.id, msg.id, msg.args) : warn(ext, `Missing permission: ${NEEDS[op]}`);
  if (!allowed) return post(ext, { kind: "reply", id: msg.id, error: `Missing permission: ${NEEDS[op]}` });
  try {
    post(ext, { kind: "reply", id: msg.id, value: handle(ext, op, msg.args) });
  } catch (e) {
    post(ext, { kind: "reply", id: msg.id, error: e instanceof Error ? e.message : String(e) });
  }
}

function launch(info: ExtensionInfo, code: string): void {
  const frame = createSandbox(info.name, code);
  const ext: Running = { info, frame, panel: addSection(info.name, frame), menu: new Map(), warned: false };
  running.set(info.id, ext);
}

function stopAll(): void {
  running.forEach((ext) => ext.panel.destroy());
  running.clear();
  clearProviders();
}

const pendingLine = (e: ExtensionInfo): string =>
  `${e.name} ${e.version} by ${e.author} (${e.trust === "validated" ? "validated" : "unvalidated"}): ${e.permissions.map(describePermission).join("; ")}`;

// One dialog covers every extension, so those turned on before this check came in need a single click.
async function confirmPending(list: ExtensionInfo[]): Promise<void> {
  const pending = list.filter((e) => e.needsConfirmation && !asked.has(e.id));
  if (pending.length === 0) return;
  pending.forEach((e) => asked.add(e.id));
  const intro = "This Storage lists these extensions as on, but they are not confirmed on this computer, so they stay off. If you turned them on yourself before this update, confirm once to keep them. If this Storage came from someone else, leave them off unless you trust their authors.";
  if (!(await warningConfirmDialog("Turn on these extensions?", [intro, ...pending.map(pendingLine)], "Turn on"))) return;
  for (const info of pending) {
    try {
      await setExtensionEnabled(info.id, true);
      info.enabled = true;
    } catch (e) {
      reportError("warning", e, `Extension ${info.name}`);
    }
  }
}

async function load(): Promise<void> {
  const gen = ++generation;
  stopAll();
  let list: ExtensionInfo[];
  try {
    list = await listExtensions();
  } catch (e) {
    reportError("warning", e, "Extensions");
    return;
  }
  await confirmPending(list);
  if (gen !== generation) return;
  for (const info of list) {
    if (info.status !== "ok" || !info.enabled) continue;
    try {
      const code = await readExtensionEntry(info.id);
      if (gen !== generation) return;
      launch(info, code);
    } catch (e) {
      reportError("warning", e, `Extension ${info.name}`);
    }
  }
}

export function startExtensions(): void {
  window.addEventListener("message", onMessage);
  void load();
}

export const reloadExtensions = (): void => void load();

export const menuItems = (): { extId: string; id: string; label: string }[] =>
  [...running.values()].flatMap((ext) => [...ext.menu].map(([id, label]) => ({ extId: ext.info.id, id, label })));

export function runMenuItem(extId: string, id: string, node: TreeNode): void {
  const ext = running.get(extId);
  if (ext?.menu.has(id)) post(ext, { kind: "menu", id, relParts: [...node.relParts] });
}
