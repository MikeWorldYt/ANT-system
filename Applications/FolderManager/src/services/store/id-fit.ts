// Picks the ids a moved block of rows takes so the fewest folders are renamed: free ids between its neighbours, else the shortest shifted run.
import { idProblem, padId, type IdKind } from "./id-order";

export interface IdFit {
  kind: IdKind;
  root: boolean;
  order: string[];
  ids: Map<string, string>;
  // Keys of the ids other siblings hold, as `takenKey` builds them.
  taken: Set<string>;
  // The uniqueness key of a row holding these digits, with the tracker it will have.
  key: (name: string, digits: string) => string;
}

const WORKNODE_MAX = 999_999;

const limits = (kind: IdKind, root: boolean) =>
  root && kind === "node" ? { min: 100, max: 800, step: 100 } : { min: 1, max: kind === "node" ? 99 : WORKNODE_MAX, step: 1 };

// Resolves to the ids after `block` (rows next to each other in `order`, top to bottom) reached its place; only rows that need it change.
export function fitBlock(fit: IdFit, block: string[]): Map<string, string> {
  const { kind, root, order, ids, taken, key } = fit;
  const { min, max, step } = limits(kind, root);
  const first = order.indexOf(block[0] ?? "");
  const last = first + block.length - 1;
  const outside = order.filter((n) => !block.includes(n));
  const pad = (n: number) => padId(String(n), kind, root);
  const numOf = (m: Map<string, string>, name: string | undefined): number | null => {
    const digits = name === undefined ? undefined : m.get(name);
    return digits !== undefined && idProblem(digits, kind, root) === null ? Number(digits) : null;
  };
  const freeFor = (m: Map<string, string>, name: string, n: number, others: string[]): boolean => {
    const k = key(name, String(n));
    return !taken.has(k) && !others.some((o) => o !== name && m.has(o) && key(o, m.get(o) ?? "") === k);
  };
  // The first free candidate inside [lo, hi] on the level's step, scanning up or down.
  const scan = (m: Map<string, string>, name: string, lo: number, hi: number, others: string[], down = false): number | null => {
    const [from, to] = [Math.ceil(Math.max(lo, min) / step) * step, Math.floor(Math.min(hi, max) / step) * step];
    for (let n = down ? to : from; down ? n >= from : n <= to; n += down ? -step : step) if (freeFor(m, name, n, others)) return n;
    return null;
  };
  // Gives the block ascending ids from `lo` up, or descending ones from `hi` down; false when they do not fit.
  const place = (m: Map<string, string>, lo: number, hi: number, others: string[], down: boolean): boolean => {
    let edge = down ? hi : lo;
    for (const name of down ? [...block].reverse() : block) {
      const n = down ? scan(m, name, lo, edge, others, true) : scan(m, name, edge, hi, others);
      if (n === null) return false;
      m.set(name, pad(n));
      edge = down ? n - 1 : n + 1;
    }
    return true;
  };
  const prev = numOf(ids, order[first - 1]);
  const next = numOf(ids, order[last + 1]);
  const lo = prev === null ? min : prev + 1;
  const hi = next === null ? max : next - 1;
  const own = block.map((n) => numOf(ids, n));
  const fits = own.every((n, i) => n !== null && n >= lo && n <= hi && (i === 0 || n > (own[i - 1] ?? Infinity)) && freeFor(ids, block[i] ?? "", n, outside));
  if (fits) return ids;
  const gapped = new Map(ids);
  if (place(gapped, lo, hi, outside, false)) return gapped;

  // No room: the block sits next to a neighbour and the run beyond it steps aside.
  const shift = (down: boolean): Map<string, string> | null => {
    const m = new Map(ids);
    if (!place(m, down ? min : lo, down ? hi : max, down ? order.slice(last + 1) : order.slice(0, first), down)) return null;
    for (let i = down ? first - 1 : last + 1; down ? i >= 0 : i < order.length; i += down ? -1 : 1) {
      const name = order[i] ?? "";
      const settled = down ? order.slice(i + 1) : order.slice(0, i);
      const edge = numOf(m, order[down ? i + 1 : i - 1]) ?? (down ? max + step : min - step);
      const cur = numOf(m, name);
      if (cur !== null && (down ? cur < edge : cur > edge) && freeFor(m, name, cur, settled)) break;
      const n = down ? scan(m, name, min, edge - 1, settled, true) : scan(m, name, edge + 1, max, settled);
      if (n === null) return null;
      m.set(name, pad(n));
    }
    return m;
  };
  const changed = (m: Map<string, string> | null) => (m === null ? Infinity : order.filter((n) => m.get(n) !== ids.get(n)).length);
  const [after, before] = [shift(false), shift(true)];
  return (changed(before) < changed(after) ? before : after) ?? ids;
}
