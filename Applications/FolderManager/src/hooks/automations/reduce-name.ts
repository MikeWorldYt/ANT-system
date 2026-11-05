// Files saved with a long hexadecimal name keep only its first four characters, so a review folder reads at a glance.
import * as api from "../../services/ipc/api";
import { refresh } from "../ops";
import { plural } from "../../services/plural";
import { reportError, setStatus } from "../../components/organisms/bars/status-line";
import { scopeFiles, type ReviewScope } from "./scope/review-scope";

const KEEP = 4;
// Shorter runs are more likely ordinary names that happen to be hex, like "cafe" or "2024".
const HEX_NAME = /^([0-9a-f]{8,})(\.[^.]+)?$/i;

export function reducedName(name: string, taken: Set<string>): string | null {
  const match = HEX_NAME.exec(name);
  if (!match) return null;
  const [, hex = "", ext = ""] = match;
  const base = hex.slice(0, KEEP);
  let candidate = `${base}${ext}`;
  for (let n = 2; taken.has(candidate.toLowerCase()); n++) candidate = `${base} (${n})${ext}`;
  return candidate;
}

export async function reduceNames(scope: ReviewScope): Promise<void> {
  try {
    const files = await scopeFiles(scope);
    // Names clash only inside one folder, so each folder keeps its own set of taken names.
    const taken = new Map<string, Set<string>>();
    const takenIn = async (parent: string[]): Promise<Set<string>> => {
      const key = parent.join("/");
      let names = taken.get(key);
      if (!names) {
        names = new Set((await api.listChildren(parent)).map((c) => c.name.toLowerCase()));
        taken.set(key, names);
      }
      return names;
    };
    let renamed = 0;
    for (const file of files) {
      const names = await takenIn(file.relParts.slice(0, -1));
      const next = reducedName(file.name, names);
      if (next === null) continue;
      await api.renameFile(file.relParts, next);
      names.delete(file.name.toLowerCase());
      names.add(next.toLowerCase());
      renamed++;
    }
    await refresh();
    setStatus("info", renamed > 0 ? `Reduced the name of ${plural(renamed, "file")}.` : "No file has a long hexadecimal name.");
  } catch (e) {
    reportError("error", e);
    await refresh();
  }
}
