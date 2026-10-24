// Builds the isolated iframe one extension runs in: opaque origin, strict CSP, postMessage only.
import { convertFileSrc } from "@tauri-apps/api/core";
import { el } from "../ui/dom";

export const SANDBOX_SCHEME = "extsandbox";

const CSP = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:";

// Runs before the extension code and is the only bridge it gets to the host.
const BOOTSTRAP = `(() => {
  const pending = new Map();
  const listeners = { menu: [], "details-action": [] };
  let provider = null;
  let seq = 0;
  const send = (msg) => parent.postMessage(Object.assign({ ant: 1 }, msg), "*");
  const report = (e) => send({ kind: "error", message: String((e && e.message) || e) });
  const call = (op, args) => new Promise((resolve, reject) => {
    const id = ++seq;
    pending.set(id, { resolve, reject });
    send({ kind: "call", id, op, args });
  });
  addEventListener("message", (ev) => {
    const m = ev.data;
    if (ev.source !== parent || !m || m.ant !== 1) return;
    if (m.kind === "reply") {
      const p = pending.get(m.id);
      if (!p) return;
      pending.delete(m.id);
      if (m.error) p.reject(new Error(m.error));
      else p.resolve(m.value);
    } else if (m.kind === "menu" || m.kind === "details-action") {
      for (const fn of listeners[m.kind]) {
        try { fn({ id: m.id, relParts: m.relParts }); } catch (e) { report(e); }
      }
    } else if (m.kind === "details-request") {
      const reply = (args) => send({ kind: "call", op: "details.reply", id: m.id, args });
      Promise.resolve()
        .then(() => (provider ? provider(m.node) : []))
        .then((fields) => reply({ fields }), (e) => reply({ error: String((e && e.message) || e) }));
    }
  });
  addEventListener("error", (ev) => report(ev.error || ev.message));
  addEventListener("unhandledrejection", (ev) => report(ev.reason));
  const ant = Object.freeze({
    tree: Object.freeze({ get: () => call("tree.get") }),
    menu: Object.freeze({ add: (item) => call("menu.add", item) }),
    panel: Object.freeze({ show: () => call("panel.show"), hide: () => call("panel.hide") }),
    details: Object.freeze({
      provide: (fn) => {
        if (typeof fn !== "function") return Promise.reject(new Error("details.provide expects a function"));
        provider = fn;
        return call("details.provide");
      },
    }),
    on: (name, fn) => {
      if (!Object.hasOwn(listeners, name) || typeof fn !== "function") throw new Error("Unknown event: " + String(name));
      listeners[name].push(fn);
    },
  });
  Object.defineProperty(window, "ant", { value: ant });
})();`;

// A literal "</script" in the code would close the tag and let the rest be parsed as markup.
const inline = (code: string): string => code.replace(/<\/(script)/gi, "<\\/$1");

export function createSandbox(name: string, code: string): HTMLIFrameElement {
  const doc = `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${CSP}"><script>${BOOTSTRAP}</script></head><body><script>${inline(code)}</script></body></html>`;
  const frame = el("iframe", { class: "ext-frame", sandbox: "allow-scripts", referrerpolicy: "no-referrer", title: `Extension ${name}` });
  // The app CSP would block inline scripts in a srcdoc, so the frame loads a loader page from a scheme with its own CSP.
  frame.src = convertFileSrc("sandbox", SANDBOX_SCHEME);
  frame.addEventListener("load", () => frame.contentWindow?.postMessage({ ant: 1, kind: "doc", html: doc }, "*"), { once: true });
  return frame;
}
