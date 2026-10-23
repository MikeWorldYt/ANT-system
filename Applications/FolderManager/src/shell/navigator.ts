// Navigation bar: back/forward/up over selection history plus a breadcrumb of the selected path.
import { describeError, listChildren } from "../ipc/api";
import { el } from "../ui/dom";
import { guiIcon, type GuiIcon } from "../ui/gui-icons";
import { findNode, reveal, select, selectedNode, state } from "../store/state";
import { clearStatus, setStatus } from "../ui/status";
import type { TreeNode } from "../ipc/types";

const splitPath = (text: string): string[] => text.trim().replace(/^"+|"+$/g, "").split(/[\\/]/).filter(Boolean);

// Maps a typed absolute path to the canonical relParts of an existing node, or null.
async function resolveTyped(text: string): Promise<string[] | null | undefined> {
  const root = splitPath(state.tree?.root ?? "");
  const typed = splitPath(text);
  const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
  if (typed.length < root.length || !root.every((p, i) => same(p, typed[i] ?? ""))) return undefined;
  const found: string[] = [];
  let level: TreeNode[] = state.tree?.nodes ?? [];
  for (const part of typed.slice(root.length)) {
    const node = level.find((n) => same(n.name, part));
    if (!node) return null;
    found.push(node.name);
    if (node.children === null && node.hasChildren) node.children = await listChildren(node.relParts);
    level = node.children ?? [];
  }
  return found;
}

function navButton(label: string, icon: GuiIcon, onClick: () => void): HTMLButtonElement {
  const b = el("button", { type: "button", class: "icon-btn", "aria-label": label, title: label }, guiIcon(icon));
  // aria-disabled keeps the button focusable, so the click is ignored here instead.
  b.addEventListener("click", () => {
    if (b.getAttribute("aria-disabled") !== "true") onClick();
  });
  return b;
}

export function createNavigator(): { element: HTMLElement; render(): void } {
  const history: string[] = [];
  let at = -1;

  function record(): void {
    const key = state.selectedKey;
    if (key === null || key === history[at]) return;
    history.splice(at + 1);
    history.push(key);
    at = history.length - 1;
  }

  async function go(index: number): Promise<void> {
    const key = history[index];
    if (key === undefined) return;
    const parts = key.split("/");
    const previous = at;
    at = index;
    try {
      await reveal(parts);
    } catch (e) {
      at = previous;
      setStatus("error", describeError(e));
      return;
    }
    if (findNode(parts)) return;
    history.splice(index, 1);
    at = previous >= index ? previous - 1 : previous;
    // reveal selected the missing folder, so the selection goes back to the entry now current.
    select(history[at]?.split("/") ?? null);
  }

  let editing = false;

  const parentOf = (): string[] | null => {
    const parts = state.selectedKey?.split("/") ?? [];
    return parts.length > 1 ? parts.slice(0, -1) : null;
  };

  const back = navButton("Back", "back", () => void go(at - 1));
  const forward = navButton("Forward", "forward", () => void go(at + 1));
  const up = navButton("Up one level", "up", () => select(parentOf()));
  const path = el("nav", { class: "breadcrumb", "aria-label": "Current location", title: "Double-click or press Ctrl+L to type a path" });
  const element = el("div", { class: "nav-bar" }, back, forward, up, path);

  function stopEditing(): void {
    editing = false;
    path.classList.remove("editing");
    render();
  }

  function startEditing(): void {
    if (editing || !state.tree) return;
    editing = true;
    path.classList.add("editing");
    const parts = state.selectedKey?.split("/") ?? [];
    const input = el("input", { class: "path-input", type: "text", spellcheck: "false", autocomplete: "off", "aria-label": "Path" });
    input.value = [state.tree.root.replace(/[\\/]+$/, ""), ...parts].join("\\");
    input.addEventListener("keydown", (ev) => {
      if (ev.key === "Escape") {
        ev.stopPropagation();
        clearStatus();
        stopEditing();
      } else if (ev.key === "Enter") {
        ev.preventDefault();
        void submit(input);
      }
    });
    input.addEventListener("input", () => input.removeAttribute("aria-invalid"));
    input.addEventListener("blur", () => {
      if (editing) stopEditing();
    });
    path.replaceChildren(input);
    input.focus();
    input.select();
  }

  async function submit(input: HTMLInputElement): Promise<void> {
    let target: string[] | null | undefined;
    try {
      target = await resolveTyped(input.value);
    } catch (e) {
      target = null;
      console.warn("Path lookup failed", e);
    }
    if (target === undefined || target === null) {
      input.setAttribute("aria-invalid", "true");
      setStatus("error", target === undefined ? "That path is outside the storage root" : "That folder does not exist in the storage");
      return;
    }
    clearStatus();
    editing = false;
    path.classList.remove("editing");
    await reveal(target);
    render();
  }

  path.addEventListener("dblclick", startEditing);
  document.addEventListener("keydown", (ev) => {
    if (ev.ctrlKey && !ev.altKey && !ev.shiftKey && !ev.metaKey && ev.key.toLowerCase() === "l") {
      ev.preventDefault();
      startEditing();
      return;
    }
    if (editing || !ev.altKey || ev.ctrlKey || ev.shiftKey || ev.metaKey) return;
    if (ev.key === "ArrowLeft") void go(at - 1);
    else if (ev.key === "ArrowRight") void go(at + 1);
    else if (ev.key === "ArrowUp") select(parentOf());
    else return;
    ev.preventDefault();
  });

  function crumbs(): (Node | string)[] {
    const out: (Node | string)[] = [];
    const sep = () => el("span", { class: "crumb-sep", "aria-hidden": "true" }, guiIcon("separator"));
    const rootName = splitPath(state.tree?.root ?? "").pop() ?? "";
    const parts = state.selectedKey?.split("/") ?? [];
    const rootBtn = el("button", { type: "button", class: "crumb crumb-btn crumb-root", title: state.tree?.root ?? "" }, rootName);
    rootBtn.addEventListener("click", () => select(null));
    out.push(rootBtn);
    if (parts.length > 0) out.push(sep());
    parts.forEach((p, i) => {
      const leaf = i === parts.length - 1;
      const b = el("button", { type: "button", class: leaf ? "crumb crumb-btn current" : "crumb crumb-btn" }, p);
      if (leaf) b.setAttribute("aria-current", "location");
      b.addEventListener("click", () => select(parts.slice(0, i + 1)));
      out.push(b);
      if (!leaf) out.push(sep());
    });
    return out;
  }

  function render(): void {
    record();
    back.setAttribute("aria-disabled", String(at <= 0));
    forward.setAttribute("aria-disabled", String(at >= history.length - 1));
    up.setAttribute("aria-disabled", String(state.selectedKey === null || selectedNode() === null));
    if (editing) return;
    path.replaceChildren(...crumbs());
    path.scrollLeft = path.scrollWidth;
  }

  return { element, render };
}
