// Picks the id a moved row takes so the fewest folders are renamed: a free id between its neighbours, else the shortest shifted run.
import { idProblem, keyFor, padId, type IdKind } from "./id-order";

export interface IdFit {
  kind: IdKind;
  root: boolean;
  order: string[];
  ids: Map<string, string>;
  // Keys of the ids other siblings hold, as `takenKey` builds them.
  taken: Set<string>;
}

const WORKNODE_MAX = 999_999;

const limits = (kind: IdKind, root: boolean) =>
  root && kind === "node" ? { min: 100, max: 800, step: 100 } : { min: 1, max: kind === "node" ? 99 : WORKNODE_MAX, step: 1 };

// Resolves to the ids after `moved` reached its place in `order`; only rows that need it change.
export function fitMoved(fit: IdFit, moved: string): Map<string, string> {
  const { kind, root, order, ids, taken } = fit;
  const { min, max, step } = limits(kind, root);
  const at = order.indexOf(moved);
  const numOf = (m: Map<string, string>, name: string | undefined): number | null => {
    const digits = name === undefined ? undefined : m.get(name);
    return digits !== undefined && idProblem(digits, kind, root) === null ? Number(digits) : null;
  };
  const freeFor = (m: Map<string, string>, name: string, n: number, others: string[]): boolean => {
    const key = keyFor(name, kind, String(n));
    return !taken.has(key) && !others.some((o) => o !== name && m.has(o) && keyFor(o, kind, m.get(o) ?? "") === key);
  };
  // Candidates ascending or descending inside [lo, hi], on the level's step.
  const scan = (m: Map<string, string>, name: string, lo: number, hi: number, others: string[], down = false): number | null => {
    const [from, to] = [Math.ceil(Math.max(lo, min) / step) * step, Math.floor(Math.min(hi, max) / step) * step];
    for (let n = down ? to : from; down ? n >= from : n <= to; n += down ? -step : step) if (freeFor(m, name, n, others)) return n;
    return null;
  };
  const prev = numOf(ids, order[at - 1]);
  const next = numOf(ids, order[at + 1]);
  const lo = prev === null ? min : prev + 1;
  const hi = next === null ? max : next - 1;
  const own = numOf(ids, moved);
  if (own !== null && own >= lo && own <= hi && freeFor(ids, moved, own, order)) return ids;
  const gap = scan(ids, moved, lo, hi, order);
  if (gap !== null) return new Map(ids).set(moved, padId(String(gap), kind, root));

  // No gap: the moved row sits next to a neighbour and the run beyond it steps one id aside.
  const shift = (down: boolean): Map<string, string> | null => {
    const m = new Map(ids);
    const first = down ? scan(m, moved, min, hi, [], true) : scan(m, moved, lo, max, []);
    if (first === null) return null;
    m.set(moved, padId(String(first), kind, root));
    for (let i = down ? at - 1 : at + 1; down ? i >= 0 : i < order.length; i += down ? -1 : 1) {
      const name = order[i] ?? "";
      const settled = down ? order.slice(i + 1) : order.slice(0, i);
      const edge = numOf(m, order[down ? i + 1 : i - 1]) ?? (down ? max + step : min - step);
      const cur = numOf(m, name);
      if (cur !== null && (down ? cur < edge : cur > edge) && freeFor(m, name, cur, settled)) break;
      const n = down ? scan(m, name, min, edge - 1, settled, true) : scan(m, name, edge + 1, max, settled);
      if (n === null) return null;
      m.set(name, padId(String(n), kind, root));
    }
    return m;
  };
  const changed = (m: Map<string, string> | null) => (m === null ? Infinity : order.filter((n) => m.get(n) !== ids.get(n)).length);
  const [after, before] = [shift(false), shift(true)];
  const best = changed(before) < changed(after) ? before : after;
  return best ?? ids;
}
