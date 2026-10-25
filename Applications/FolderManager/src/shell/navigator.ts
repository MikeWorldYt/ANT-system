// Navigation bar: back/forward/up over selection history plus a breadcrumb of the selected path.
import { describeError } from "../ipc/api";
import { el } from "../ui/dom";
import { guiIcon } from "../ui/gui-icons";
import { iconButton } from "../ui/icon-button";
import { createPathInput, rootName } from "./path-input";
import { findNode, reveal, select, selectedNode, state } from "../store/state";
import { setStatus } from "../ui/status";

export function createNavigator(): { element: HTMLElement; render(): void; editPath(): void; goUp(): void } {
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

  const parentOf = (): string[] | null => {
    const parts = state.selectedKey?.split("/") ?? [];
    return parts.length > 1 ? parts.slice(0, -1) : null;
  };

  const back = iconButton("Back", "back", () => void go(at - 1));
  const forward = iconButton("Forward", "forward", () => void go(at + 1));
  const up = iconButton("Up one level", "up", () => select(parentOf()));
  const path = el("nav", { class: "breadcrumb", "aria-label": "Current location", title: "Double-click or press Ctrl+L to type a path" });
  const element = el("div", { class: "nav-bar" }, back, forward, up, path);

  const pathInput = createPathInput(path, render);

  path.addEventListener("dblclick", pathInput.start);
  document.addEventListener("keydown", (ev) => {
    if (pathInput.editing() || !ev.altKey || ev.ctrlKey || ev.shiftKey || ev.metaKey) return;
    if (ev.key === "ArrowLeft") void go(at - 1);
    else if (ev.key === "ArrowRight") void go(at + 1);
    else return;
    ev.preventDefault();
  });

  function crumbs(): (Node | string)[] {
    const out: (Node | string)[] = [];
    const sep = () => el("span", { class: "crumb-sep", "aria-hidden": "true" }, guiIcon("separator"));
    const parts = state.selectedKey?.split("/") ?? [];
    const rootBtn = el("button", { type: "button", class: "crumb crumb-btn crumb-root", title: state.tree?.root ?? "" }, rootName(state.tree?.root));
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
    if (pathInput.editing()) return;
    path.replaceChildren(...crumbs());
    path.scrollLeft = path.scrollWidth;
  }

  return { element, render, editPath: pathInput.start, goUp: () => select(parentOf()) };
}
