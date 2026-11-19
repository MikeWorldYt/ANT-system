// The transfer graph: blue bars for the data moved each second, an accent line for the files finished each second.
const SVG = "http://www.w3.org/2000/svg";
export const SAMPLES = 60;
const W = 600;
const H = 120;

export interface Sample {
  bytes: number;
  files: number;
}

function node<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string>): SVGElementTagNameMap[K] {
  const n = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) n.setAttribute(k, v);
  return n;
}

export function transferChart(): { element: SVGSVGElement; draw: (samples: Sample[]) => void } {
  const element = node("svg", { class: "transfer-chart", viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: "none", "aria-hidden": "true" });
  const bars = node("g", { class: "transfer-bars" });
  const line = node("polyline", { class: "transfer-line" });
  element.append(bars, line);
  function draw(samples: Sample[]): void {
    const step = W / SAMPLES;
    const peak = Math.max(1, ...samples.map((s) => s.bytes));
    const filesPeak = Math.max(1, ...samples.map((s) => s.files));
    const start = SAMPLES - samples.length;
    bars.replaceChildren(
      ...samples.map((s, i) => {
        const h = Math.max(1, (s.bytes / peak) * (H - 8));
        return node("rect", { x: String((start + i) * step + 1), y: String(H - h), width: String(step - 2), height: String(h) });
      }),
    );
    line.setAttribute("points", samples.map((s, i) => `${(start + i + 0.5) * step},${H - 2 - (s.files / filesPeak) * (H - 12)}`).join(" "));
  }
  return { element, draw };
}
