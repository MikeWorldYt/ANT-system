// A PDF in the side panel: one page at a time, turned with the buttons below it or the arrow keys.
import { el } from "../../dom";
import { pdfPage } from "../../../services/ipc/api";
import { reportError } from "./status-line";
import type { TreeNode } from "../../../services/ipc/types";

export function pdfPages(n: TreeNode, still: () => boolean): HTMLElement {
  const img = el("img", { alt: `${n.name}, page 1`, draggable: "false" });
  const prev = el("button", { type: "button", class: "pdf-turn", "aria-label": "Previous page", title: "Previous page", disabled: "" }, "‹");
  const next = el("button", { type: "button", class: "pdf-turn", "aria-label": "Next page", title: "Next page", disabled: "" }, "›");
  const count = el("span", { class: "pdf-count", "aria-live": "polite" }, "…");
  const bar = el("div", { class: "pdf-bar", hidden: "" }, prev, count, next);
  const box = el("div", { class: "pdf-pages", tabindex: "0", "aria-label": `Pages of ${n.name}` }, img, bar);
  let page = 0;
  let pages = 1;

  function show(at: number): void {
    pdfPage(n.relParts, at)
      .then((drawn) => {
        if (!still() || !drawn) return;
        page = Math.min(at, drawn.count - 1);
        pages = drawn.count;
        img.src = drawn.url;
        img.alt = `${n.name}, page ${page + 1} of ${pages}`;
        count.textContent = `${page + 1} / ${pages}`;
        bar.hidden = pages < 2;
        prev.disabled = page === 0;
        next.disabled = page >= pages - 1;
      })
      .catch((e: unknown) => reportError("warning", e, "PDF preview"));
  }

  const turn = (by: number): void => {
    const to = page + by;
    if (to >= 0 && to < pages) show(to);
  };
  prev.addEventListener("click", () => turn(-1));
  next.addEventListener("click", () => turn(1));
  box.addEventListener("keydown", (ev) => {
    if (ev.key !== "ArrowLeft" && ev.key !== "ArrowRight") return;
    ev.preventDefault();
    ev.stopPropagation();
    turn(ev.key === "ArrowLeft" ? -1 : 1);
  });
  show(0);
  return box;
}
