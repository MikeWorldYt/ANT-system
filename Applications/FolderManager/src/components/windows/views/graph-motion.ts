// The graph is rebuilt on every change; this animates the step between two layouts so groups resize smoothly instead of jumping.
const FADE_MS = 160;
const MOVE_MS = 260;

export interface Snapshot {
  rects: Map<string, DOMRect>;
  ghosts: Map<string, HTMLElement>;
  unit: number;
}

const keyed = (canvas: HTMLElement) => canvas.querySelectorAll<HTMLElement>("[data-key]");

export function snapshot(canvas: HTMLElement): Snapshot {
  const unit = canvas.getBoundingClientRect().width / (canvas.offsetWidth || 1);
  const rects = new Map<string, DOMRect>();
  const ghosts = new Map<string, HTMLElement>();
  for (const e of keyed(canvas)) {
    const key = e.dataset.key ?? "";
    rects.set(key, e.getBoundingClientRect());
    ghosts.set(key, e);
  }
  return { rects, ghosts, unit };
}

// Shrinking: what left fades out, then the rest closes in. Growing: the rest makes room, then what is new fades in.
export function play(canvas: HTMLElement, before: Snapshot): number {
  if (before.rects.size === 0 || matchMedia("(prefers-reduced-motion: reduce)").matches) return 0;
  const origin = canvas.getBoundingClientRect();
  const unit = origin.width / (canvas.offsetWidth || 1);
  const now = new Map<string, HTMLElement>();
  for (const e of keyed(canvas)) now.set(e.dataset.key ?? "", e);

  const gone = [...before.rects.keys()].filter((k) => !now.has(k));
  const moveAt = gone.length > 0 ? FADE_MS : 0;
  let moved = false;
  for (const [key, e] of now) {
    const old = before.rects.get(key);
    if (!old) continue;
    const r = e.getBoundingClientRect();
    // Measured on screen, not in the canvas: the view scrolls to hold the selected card still, and that card must not travel.
    const dx = (old.left - r.left) / unit;
    const dy = (old.top - r.top) / unit;
    if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
    moved = true;
    e.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "translate(0, 0)" }], { duration: MOVE_MS, delay: moveAt, easing: "ease-in-out", fill: "backwards", composite: "add" });
  }
  const showAt = moveAt + (moved ? MOVE_MS : 0);
  for (const [key, e] of now) {
    if (!before.rects.has(key)) e.animate([{ opacity: 0 }, { opacity: 1 }], { duration: FADE_MS, delay: showAt, fill: "backwards" });
  }
  // What left is put back as a ghost where it stood, so it can fade before the space closes.
  for (const key of gone) {
    const old = before.rects.get(key);
    const ghost = before.ghosts.get(key);
    if (!old || !ghost) continue;
    ghost.classList.add("graph-ghost");
    ghost.classList.remove("shift", "on-path", "selected");
    ghost.removeAttribute("data-key");
    ghost.removeAttribute("id");
    ghost.style.left = `${(old.left - origin.left) / unit}px`;
    ghost.style.top = `${(old.top - origin.top) / unit}px`;
    ghost.style.width = `${old.width / before.unit}px`;
    canvas.append(ghost);
    ghost.animate([{ opacity: 1 }, { opacity: 0 }], { duration: FADE_MS, fill: "forwards" }).finished.then(
      () => ghost.remove(),
      () => ghost.remove(),
    );
  }
  return showAt + (now.size > before.rects.size ? FADE_MS : 0);
}
