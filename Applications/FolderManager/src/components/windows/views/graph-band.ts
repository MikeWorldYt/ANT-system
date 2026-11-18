// A band dragged over a graph box's empty space marks the files it covers, as the other views do over their items.
import { el } from "../../dom";
import { selectKeys, state } from "../../../services/store/state";

// The resize grip sits in the box's corner, so a press there drags the size, not a band.
const GRIP = 16;

export function bandSelect(box: HTMLElement): void {
  box.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0 || (ev.target instanceof Element && ev.target.closest(".graph-file"))) return;
    const r = box.getBoundingClientRect();
    if (ev.clientX > r.right - GRIP && ev.clientY > r.bottom - GRIP) return;
    ev.stopPropagation();
    const start = { x: ev.clientX, y: ev.clientY };
    const base = ev.ctrlKey ? [...state.selectedKeys] : [];
    const rows = [...box.querySelectorAll<HTMLElement>(".graph-file[data-key]")];
    let band: HTMLElement | null = null;
    let picked: string[] = base;
    const move = (m: PointerEvent): void => {
      if (!band) {
        if (Math.hypot(m.clientX - start.x, m.clientY - start.y) < 4) return;
        band = el("div", { class: "marquee", "aria-hidden": "true" });
        document.body.append(band);
      }
      const left = Math.min(start.x, m.clientX);
      const top = Math.min(start.y, m.clientY);
      const right = Math.max(start.x, m.clientX);
      const bottom = Math.max(start.y, m.clientY);
      Object.assign(band.style, { left: `${left}px`, top: `${top}px`, width: `${right - left}px`, height: `${bottom - top}px` });
      const hit = rows.filter((row) => {
        const b = row.getBoundingClientRect();
        return b.right > left && b.left < right && b.bottom > top && b.top < bottom;
      });
      picked = [...new Set([...base, ...hit.map((row) => row.dataset.key ?? "")])];
      // The graph redraws on every change, so the rows are marked here and the selection is set once, on release.
      for (const row of rows) row.classList.toggle("selected", picked.includes(row.dataset.key ?? ""));
    };
    const up = (): void => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      if (!band) return;
      band.remove();
      // The click that ends a band must not then select the box's folder over the marked files.
      box.dataset.banded = "1";
      selectKeys(picked);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  });
}
