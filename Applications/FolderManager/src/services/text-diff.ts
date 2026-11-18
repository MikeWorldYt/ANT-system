// Character-level diff of two short names, so the batch view can mark what an automation removed and added.
export interface Run {
  text: string;
  changed: boolean;
}

function push(runs: Run[], ch: string, changed: boolean): void {
  const last = runs[runs.length - 1];
  if (last && last.changed === changed) last.text += ch;
  else runs.push({ text: ch, changed });
}

export interface MixRun {
  text: string;
  mark: "same" | "del" | "ins";
}

function pushMix(runs: MixRun[], ch: string, mark: MixRun["mark"]): void {
  const last = runs[runs.length - 1];
  if (last && last.mark === mark) last.text += ch;
  else runs.push({ text: ch, mark });
}

export function diffRuns(before: string, after: string): { before: Run[]; after: Run[]; mixed: MixRun[] } {
  const a = Array.from(before);
  const b = Array.from(after);
  // len[i][j] = longest common subsequence of a[i..] and b[j..]
  const len = Array.from({ length: a.length + 1 }, () => new Array<number>(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) len[i][j] = a[i] === b[j] ? len[i + 1][j + 1] + 1 : Math.max(len[i + 1][j], len[i][j + 1]);
  }
  // One name that keeps both sides: removed text where it was, added text where it lands.
  const out = { before: [] as Run[], after: [] as Run[], mixed: [] as MixRun[] };
  let i = 0;
  let j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && a[i] === b[j]) {
      pushMix(out.mixed, a[i], "same");
      push(out.before, a[i++], false);
      push(out.after, b[j++], false);
    } else if (j >= b.length || (i < a.length && len[i + 1][j] >= len[i][j + 1])) {
      pushMix(out.mixed, a[i], "del");
      push(out.before, a[i++], true);
    } else {
      pushMix(out.mixed, b[j], "ins");
      push(out.after, b[j++], true);
    }
  }
  return out;
}
