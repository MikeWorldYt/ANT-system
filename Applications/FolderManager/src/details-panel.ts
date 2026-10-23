// Read-only details of the selected node; renaming happens in place and type changes in the context menu.
import { el } from "./dom";
import { nodeDetails } from "./node-info";
import { selectedNode } from "./state";

export function createDetailsPanel(): { element: HTMLElement; render(): void } {
  const list = el("dl", { class: "details" });
  const element = el("aside", { class: "details-panel", "aria-labelledby": "details-title" }, el("h2", { id: "details-title" }, "Details"), list);

  return {
    element,
    render() {
      const n = selectedNode();
      list.replaceChildren(...(n ? nodeDetails(n) : []).flatMap(([k, v]) => [el("dt", {}, k), el("dd", {}, v)]));
    },
  };
}
