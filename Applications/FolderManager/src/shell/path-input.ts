// Typed-path editing for the navigation bar: turns the breadcrumb into an input and reveals the node it names.
import { describeError, listChildren } from "../ipc/api";
import { el } from "../ui/dom";
import { reveal, state } from "../store/state";
import { clearStatus, setStatus } from "../ui/status";
import type { TreeNode } from "../ipc/types";

const splitPath = (text: string): string[] => text.trim().replace(/^"+|"+$/g, "").split(/[\\/]/).filter(Boolean);

export const rootName = (root: string | undefined): string => splitPath(root ?? "").pop() ?? "";

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

export function createPathInput(path: HTMLElement, render: () => void): { editing(): boolean; start(): void } {
  let editing = false;

  function stopEditing(): void {
    editing = false;
    path.classList.remove("editing");
    render();
  }

  function start(): void {
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
    try {
      await reveal(target);
    } catch (e) {
      setStatus("error", describeError(e));
    }
    render();
  }

  return { editing: () => editing, start };
}
