// A pretend file explorer window for the tour: its files drag onto a folder of the demo Storage and nothing on disk is read or moved.
import { el } from "../../dom";
import { fileIcon } from "../../../services/demo/demo-storage";

export interface FakeFile {
  name: string;
  size: number;
}

export const SAMPLE_FILES: FakeFile[] = [
  { name: "Brief.pdf", size: 248_000 },
  { name: "Logo.png", size: 96_000 },
  { name: "Budget.xlsx", size: 54_000 },
  { name: "Meeting-notes.docx", size: 31_000 },
];

const kb = (size: number): string => `${Math.round(size / 1000)} KB`;

function cursorIcon(): SVGElement {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("class", "fx-cursor");
  const path = document.createElementNS(ns, "path");
  path.setAttribute("d", "M4 2l14 10-6.5 1.2L15 21l-3 1.3-3.5-7.8L4 19z");
  svg.append(path);
  return svg;
}

export interface FakeExplorer {
  element: HTMLElement;
  remove(): void;
}

// The webview hands native drags to Tauri for real file drops, so this drag runs on pointer events with a ghost that follows the pointer.
// `hintTarget` is where the idle hint shows a file going when the user has not dragged one for a while.
export function fakeExplorer(accepts: (target: Element) => boolean, dropped: (file: FakeFile) => void, hintTarget: () => Element | null): FakeExplorer {
  const list = el("ul", { class: "fx-list", "aria-label": "Files" });
  let ghost: HTMLElement | null = null;
  let idle = 0;
  let hint: HTMLElement | null = null;
  const stopHint = (): void => {
    hint?.remove();
    hint = null;
  };
  // A pretend cursor carries the first file to the project and fades out; nothing is dropped.
  function playHint(): void {
    const row = list.querySelector(".fx-file");
    const target = hintTarget();
    if (!row || !target || hint) return;
    const from = row.getBoundingClientRect();
    const to = target.getBoundingClientRect();
    const label = row.querySelector(".fx-name")?.textContent ?? "";
    hint = el("div", { class: "fx-hint", "aria-hidden": "true" }, cursorIcon(), el("span", { class: "fx-ghost" }, label));
    document.body.append(hint);
    const [x0, y0, x1, y1] = [from.left + 40, from.top + from.height / 2, to.left + Math.min(to.width / 2, 80), to.top + to.height / 2];
    const run = hint.animate(
      [
        { transform: `translate(${x0}px, ${y0}px)`, opacity: 0 },
        { transform: `translate(${x0}px, ${y0}px)`, opacity: 1, offset: 0.15 },
        { transform: `translate(${x1}px, ${y1}px)`, opacity: 1, offset: 0.8 },
        { transform: `translate(${x1}px, ${y1}px)`, opacity: 0 },
      ],
      { duration: 2200, easing: "ease-in-out" },
    );
    run.onfinish = () => {
      stopHint();
      wait();
    };
  }
  function wait(): void {
    window.clearTimeout(idle);
    idle = window.setTimeout(playHint, 6000);
  }
  for (const file of SAMPLE_FILES) {
    const row = el("li", { class: "fx-file", title: "Drag onto your project" }, el("img", { class: "fx-icon", src: fileIcon(file.name), alt: "" }), el("span", { class: "fx-name" }, file.name), el("span", { class: "fx-size" }, kb(file.size)));
    row.addEventListener("pointerdown", (ev) => {
      ev.preventDefault();
      stopHint();
      window.clearTimeout(idle);
      row.setPointerCapture(ev.pointerId);
      ghost = el("div", { class: "fx-ghost", "aria-hidden": "true" }, file.name);
      document.body.append(ghost);
      move(ev);
    });
    const under = (ev: PointerEvent): Element | null => document.elementFromPoint(ev.clientX, ev.clientY);
    const move = (ev: PointerEvent): void => {
      if (!ghost) return;
      Object.assign(ghost.style, { left: `${ev.clientX + 12}px`, top: `${ev.clientY + 12}px` });
      const target = under(ev);
      ghost.classList.toggle("is-over", target !== null && accepts(target));
    };
    row.addEventListener("pointermove", move);
    row.addEventListener("pointerup", (ev) => {
      if (!ghost) return;
      ghost.remove();
      ghost = null;
      wait();
      const target = under(ev);
      if (!target || !accepts(target)) return;
      row.remove();
      dropped(file);
    });
    row.addEventListener("pointercancel", () => {
      ghost?.remove();
      ghost = null;
    });
    list.append(row);
  }
  const title = el("div", { class: "fx-title", title: "Drag to move the window" }, el("span", {}, "Your File Explorer"), el("span", { class: "fx-controls", "aria-hidden": "true" }, "— ☐ ✕"));
  const element = el("div", { class: "fx-window", role: "dialog", "aria-label": "Your File Explorer" }, title, list);
  // Like a real window it moves by its title bar, in case it sits over the folder the user wants.
  title.addEventListener("pointerdown", (start) => {
    const box = element.getBoundingClientRect();
    const [dx, dy] = [start.clientX - box.left, start.clientY - box.top];
    title.setPointerCapture(start.pointerId);
    const follow = (ev: PointerEvent): void => {
      Object.assign(element.style, { left: `${ev.clientX - dx}px`, top: `${ev.clientY - dy}px`, right: "auto", bottom: "auto" });
      window.dispatchEvent(new Event("tour-explorer-moved"));
    };
    title.addEventListener("pointermove", follow);
    title.addEventListener("pointerup", () => title.removeEventListener("pointermove", follow), { once: true });
  });
  document.body.append(element);
  window.dispatchEvent(new Event("tour-explorer-moved"));
  wait();
  return {
    element,
    remove() {
      window.clearTimeout(idle);
      stopHint();
      ghost?.remove();
      element.remove();
    },
  };
}
