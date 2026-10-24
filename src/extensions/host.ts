// Starts the enabled extensions, answers their permission-checked requests and owns their menu items.
import { describeError, listExtensions, readExtensionEntry, type ExtensionInfo } from "../ipc/api";
import type { TreeNode } from "../ipc/types";
import { state } from "../store/state";
import { setStatus } from "../ui/status";
import { addSection, type PanelSection } from "./panel";
import { MAX_MENU_ITEMS, NEEDS, parseIncoming, parseMenuItem, summarize, type Op } from "./protocol";
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

function warn(ext: Running, text: string): void {
  if (ext.warned) return;
  ext.warned = true;
  setStatus("warning", `Extension ${ext.info.name}: ${text}`);
}

const post = (ext: Running, msg: Record<string, unknown>) => ext.frame.contentWindow?.postMessage({ ant: 1, ...msg }, "*");

function handle(ext: Running, op: Op, args: unknown): unknown {
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
  const needed = NEEDS[msg.op];
  if (!ext.info.permissions.includes(needed)) return post(ext, { kind: "reply", id: msg.id, error: `Missing permission: ${needed}` });
  try {
    post(ext, { kind: "reply", id: msg.id, value: handle(ext, msg.op, msg.args) });
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
}

async function load(): Promise<void> {
  const gen = ++generation;
  stopAll();
  let list: ExtensionInfo[];
  try {
    list = await listExtensions();
  } catch (e) {
    setStatus("warning", `Extensions: ${describeError(e)}`);
    return;
  }
  for (const info of list) {
    if (info.status !== "ok" || !info.enabled) continue;
    try {
      const code = await readExtensionEntry(info.id);
      if (gen !== generation) return;
      launch(info, code);
    } catch (e) {
      setStatus("warning", `Extension ${info.name}: ${describeError(e)}`);
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
