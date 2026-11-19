// The transfer window's panel: the running copy or move with its graph, progress, pause and cancel.
import { el } from "../../dom";
import { cancelTransfer, pauseTransfer } from "../../../services/ipc/api";
import { onTransfer } from "../../../services/ipc/events";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { reportError } from "../bars/status-line";
import { plural } from "../../../services/plural";
import { SAMPLES, transferChart, type Sample } from "./transfer-chart";
import type { TransferProgress } from "../../../services/ipc/types";

const SAMPLE_MS = 1000;
const ETA_SAMPLES = 5;
const UNITS = ["B", "KB", "MB", "GB", "TB"];

function bytesText(n: number): string {
  let i = 0;
  while (n >= 1024 && i < UNITS.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${UNITS[i]}`;
}

function timeText(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  const pad = (v: number) => String(v).padStart(2, "0");
  return `${pad(Math.floor(s / 3600))}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
}

function stat(label: string): [HTMLElement, HTMLElement] {
  const value = el("strong", { class: "transfer-stat-value" }, "-");
  return [el("div", { class: "transfer-stat" }, value, el("span", { class: "transfer-stat-label" }, label)), value];
}

export function installTransferPanel(host: HTMLElement): void {
  const chart = transferChart();
  const title = el("h2", { class: "transfer-title" });
  const route = el("p", { class: "transfer-route" });
  const [current, currentValue] = stat("Current");
  const [peak, peakValue] = stat("Peak");
  const [total, totalValue] = stat("Total");
  const [files, filesValue] = stat("Files");
  const percent = el("span", { class: "transfer-percent" });
  const eta = el("span", { class: "transfer-eta" });
  const bar = el("progress", { class: "transfer-progress", max: "100", value: "0", "aria-label": "Transfer progress" }) as HTMLProgressElement;
  const name = el("p", { class: "transfer-current" });
  const pause = el("button", { type: "button", class: "transfer-btn" }, "Pause");
  const cancel = el("button", { type: "button", class: "transfer-btn danger" }, "Cancel");
  const minimize = el("button", { type: "button", class: "transfer-btn" }, "Minimize");
  const legend = el("div", { class: "transfer-legend" }, el("span", { class: "legend-bars" }, "Data"), el("span", { class: "legend-line" }, "Files"));
  const panel = el(
    "section",
    { class: "transfer-panel", role: "region", "aria-label": "File transfer" },
    el("div", { class: "transfer-graph", "data-tauri-drag-region": "" }, chart.element, legend),
    el("div", { class: "transfer-side" }, title, route, el("div", { class: "transfer-stats" }, current, peak, total, files), el("div", { class: "transfer-row" }, percent, eta), bar, name, el("div", { class: "transfer-actions" }, minimize, pause, cancel)),
  );
  host.replaceChildren(panel);

  let job = -1;
  let last: TransferProgress | null = null;
  let mark = { at: 0, bytes: 0, files: 0 };
  let samples: Sample[] = [];
  let top = 0;

  function reset(p: TransferProgress): void {
    job = p.id;
    mark = { at: performance.now(), bytes: 0, files: 0 };
    samples = [];
    top = 0;
    chart.draw(samples);
  }

  function sample(p: TransferProgress, now: number): void {
    if (now - mark.at < SAMPLE_MS) return;
    const secs = (now - mark.at) / 1000;
    samples = [...samples, { bytes: (p.bytesDone - mark.bytes) / secs, files: (p.filesDone - mark.files) / secs }].slice(-SAMPLES);
    mark = { at: now, bytes: p.bytesDone, files: p.filesDone };
    top = Math.max(top, samples.at(-1)?.bytes ?? 0);
    chart.draw(samples);
  }

  function render(p: TransferProgress): void {
    const recent = samples.slice(-ETA_SAMPLES);
    const speed = recent.reduce((a, s) => a + s.bytes, 0) / Math.max(1, recent.length);
    const share = p.bytesTotal > 0 ? (p.bytesDone / p.bytesTotal) * 100 : 0;
    title.textContent = p.paused ? "Paused" : "Transferring";
    route.textContent = `${plural(p.filesTotal, "file")} from ${p.from || "outside"} to ${p.to}`;
    currentValue.textContent = `${bytesText(samples.at(-1)?.bytes ?? 0)}/s`;
    peakValue.textContent = `${bytesText(top)}/s`;
    totalValue.textContent = `${bytesText(p.bytesDone)} / ${bytesText(p.bytesTotal)}`;
    filesValue.textContent = `${p.filesDone} / ${p.filesTotal}`;
    percent.textContent = `${Math.floor(share)}%`;
    eta.textContent = speed > 0 && !p.paused ? `${timeText((p.bytesTotal - p.bytesDone) / speed)} remaining` : "Estimating...";
    bar.value = share;
    name.textContent = p.current;
    pause.textContent = p.paused ? "Resume" : "Pause";
  }

  pause.addEventListener("click", () => {
    if (last) void pauseTransfer(!last.paused).catch((e: unknown) => reportError("warning", e, "Pause"));
  });
  minimize.addEventListener("click", () => void getCurrentWindow().minimize().catch((e: unknown) => reportError("warning", e, "Minimize")));
  cancel.addEventListener("click", () => void cancelTransfer().catch((e: unknown) => reportError("warning", e, "Cancel")));

  onTransfer((p) => {
    if (p.id !== job) reset(p);
    last = p;
    sample(p, performance.now());
    render(p);
  }).catch((e: unknown) => reportError("warning", e, "Transfer progress unavailable"));
}
