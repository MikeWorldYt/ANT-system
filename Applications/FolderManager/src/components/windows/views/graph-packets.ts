// Files a batch sent out of Downloads travel the graph as one packet per destination, along the graph's own lines, from the Downloads card to where they landed.
import { el } from "../../dom";

// Pixels per millisecond, so a long trip takes longer than a short one at the same pace.
const SPEED = 1.4;
const MIN_MS = 175;

export interface Route {
  cards: HTMLElement[];
  // From the shared folder down to the one the files left; the packet runs it backwards before taking `cards`.
  back?: HTMLElement[];
}

interface Box {
  l: number;
  r: number;
  b: number;
  my: number;
  cx: number;
}

const headOf = (c: HTMLElement) => c.closest(".graph-head");
// A card stacked under another one's head (Downloads under the root, REV under its node) hangs from a line down its left side.
const hangsFrom = (child: HTMLElement, parent: HTMLElement) => child.parentElement?.classList.contains("graph-under") === true && headOf(child) === headOf(parent);

// The same turns the lines take: out of the right edge to the split, along it, into the left edge; or down the left side to a card hanging below.
function waypoints(canvas: HTMLElement, cards: HTMLElement[]): { x: number; y: number }[] {
  const origin = canvas.getBoundingClientRect();
  const unit = origin.width / (canvas.offsetWidth || 1);
  const box = (c: HTMLElement): Box => {
    const r = c.getBoundingClientRect();
    const l = (r.left - origin.left) / unit;
    const t = (r.top - origin.top) / unit;
    return { l, r: l + r.width / unit, b: t + r.height / unit, my: t + r.height / unit / 2, cx: l + r.width / unit / 2 };
  };
  const first = box(cards[0]!);
  const points = [{ x: first.cx, y: first.my }];
  for (let i = 1; i < cards.length; i++) {
    const a = cards[i - 1]!;
    const b = cards[i]!;
    const A = box(a);
    const B = box(b);
    if (hangsFrom(b, a)) {
      const x = A.l + 14;
      points.push({ x, y: A.my }, { x, y: B.my }, { x: B.l, y: B.my }, { x: B.cx, y: B.my });
    } else if (hangsFrom(a, b)) {
      const x = B.l + 14;
      points.push({ x: A.l, y: A.my }, { x, y: A.my }, { x, y: B.my });
    } else {
      const headRight = headOf(a)?.getBoundingClientRect().right;
      const tx = headRight === undefined ? A.r : (headRight - origin.left) / unit + 10;
      const split = (tx + B.l) / 2;
      points.push({ x: A.r, y: A.my }, { x: split, y: A.my }, { x: split, y: B.my }, { x: B.l, y: B.my }, { x: B.cx, y: B.my });
    }
  }
  // The trip ends against the far edge of the last card, as if the packet ran into it.
  const end = points[points.length - 1]!;
  end.x = box(cards[cards.length - 1]!).r - 8;
  return points;
}

// The packet rides on the stage, not the canvas, so a redraw of the cards under it does not wipe it out; `follow` keeps it on screen.
export async function sendPacket(stage: HTMLElement, canvas: HTMLElement, route: Route, track: (a: Animation) => Animation, follow: (packet: HTMLElement) => void): Promise<void> {
  const climb = route.back && route.back.length > 1 ? waypoints(canvas, route.back).reverse() : [];
  const descend = route.cards.length > 1 ? waypoints(canvas, route.cards) : [];
  if (climb.length === 0 && descend.length === 0) return;
  const points = climb.length > 0 ? [...climb, ...descend.slice(1)] : descend;
  const lengths = points.map((p, i) => (i === 0 ? 0 : Math.hypot(p.x - points[i - 1]!.x, p.y - points[i - 1]!.y)));
  const total = lengths.reduce((s, d) => s + d, 0) || 1;
  let run = 0;
  const frames = points.map((p, i) => {
    run += lengths[i]!;
    return { transform: `translate(${p.x}px, ${p.y}px) translate(-50%, -50%)`, offset: run / total };
  });
  // A plain dot: in the graph the count would only clutter the trip.
  const packet = el("div", { class: "graph-packet", "aria-hidden": "true" });
  stage.append(packet);
  const moves = track(packet.animate(frames, { duration: Math.max(MIN_MS, total / SPEED), easing: "linear", fill: "both" }));
  let watching = true;
  const watch = () => {
    if (!watching) return;
    follow(packet);
    requestAnimationFrame(watch);
  };
  watch();
  await moves.finished.then(() => undefined, () => undefined);
  watching = false;
  packet.remove();
  const last = route.cards[route.cards.length - 1]!;
  const landing = canvas.querySelector<HTMLElement>(`.graph-node-box[data-key="${CSS.escape(last.dataset.key ?? "")}"]`) ?? last;
  landing.classList.add("graph-landed");
  landing.addEventListener("animationend", () => landing.classList.remove("graph-landed"), { once: true });
}
