// Navigation bar: back/forward/up over selection history plus a breadcrumb of the selected path.
import { el } from "./dom";
import { findNode, hydrate, keyOf, notify, select, selectedNode, state } from "./state";

function navButton(label: string, glyph: string, onClick: () => void): HTMLButtonElement {
  const b = el("button", { type: "button", class: "icon-btn", "aria-label": label, title: label }, glyph);
  b.addEventListener("click", onClick);
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
    if (!findNode(parts)) {
      history.splice(index, 1);
      if (index <= at) at -= 1;
      notify();
      return;
    }
    at = index;
    for (let i = 1; i < parts.length; i++) state.expanded.add(keyOf(parts.slice(0, i)));
    await hydrate();
    select(parts);
  }

  const parentOf = (): string[] | null => {
    const parts = state.selectedKey?.split("/") ?? [];
    return parts.length > 1 ? parts.slice(0, -1) : null;
  };

  const back = navButton("Back", "←", () => void go(at - 1));
  const forward = navButton("Forward", "→", () => void go(at + 1));
  const up = navButton("Up one level", "↑", () => select(parentOf()));
  const path = el("nav", { class: "breadcrumb", "aria-label": "Current location" });
  const element = el("div", { class: "nav-bar" }, back, forward, up, path);

  document.addEventListener("keydown", (ev) => {
    if (!ev.altKey || ev.ctrlKey || ev.shiftKey || ev.metaKey) return;
    if (ev.key === "ArrowLeft") void go(at - 1);
    else if (ev.key === "ArrowRight") void go(at + 1);
    else if (ev.key === "ArrowUp") select(parentOf());
    else return;
    ev.preventDefault();
  });

  function crumbs(): (Node | string)[] {
    const out: (Node | string)[] = [];
    const sep = () => el("span", { class: "crumb-sep", "aria-hidden": "true" }, "›");
    const rootParts = (state.tree?.root ?? "").split(/[\/]/).filter(Boolean);
    const parts = state.selectedKey?.split("/") ?? [];
    rootParts.forEach((p, i) => out.push(el("span", { class: "crumb crumb-root" }, p), ...(i < rootParts.length - 1 || parts.length > 0 ? [sep()] : [])));
    parts.forEach((p, i) => {
      const leaf = i === parts.length - 1;
      const b = el("button", { type: "button", class: leaf ? "crumb crumb-btn current" : "crumb crumb-btn" }, p);
      if (leaf) b.setAttribute("aria-current", "page");
      b.addEventListener("click", () => select(parts.slice(0, i + 1)));
      out.push(b);
      if (!leaf) out.push(sep());
    });
    return out;
  }

  return {
    element,
    render() {
      record();
      back.disabled = at <= 0;
      forward.disabled = at >= history.length - 1;
      up.disabled = state.selectedKey === null || selectedNode() === null;
      path.replaceChildren(...crumbs());
      path.scrollLeft = path.scrollWidth;
    },
  };
}
