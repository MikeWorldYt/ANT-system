// Drops in Batch mode: a file let go on a group is pinned to that group's dictionary entry; files from other apps land in the batch folder first.
import * as api from "../../services/ipc/api";
import { inDownloads } from "../../services/ipc/types";
import { state } from "../../services/store/state";
import { reportError, setStatus } from "../../components/organisms/bars/status-line";
import { dropFiles } from "../ops";
import { moveDragged } from "../folder-move";
import { batch, replanBatch } from "./batch";

// A group that came from a dictionary entry carries it; the loose files and the empty space outside the groups take drops that unpin.
// Groups the project sort made take none.
export const groupAt = (target: Element | null): HTMLElement | null =>
  target?.closest<HTMLElement>(".batch-box[data-entry], .batch-loose") ?? (target?.closest(".batch-box") ? null : (target?.closest<HTMLElement>(".batch-scroll") ?? null));

// No entry unpins: the files leave every `### files` list and the rules place them again.
export async function pinTo(entry: string | null, files: string[]): Promise<void> {
  if (!batch) return;
  try {
    for (const file of files) await api.pinDictionaryFile(batch.folder, entry, file);
    const what = files.length === 1 ? files[0] : `${files.length} files`;
    setStatus("info", entry ? `${what} now go to ${entry}.` : `${what} left their pinned group; the dictionary rules place them now.`);
    await replanBatch(batch.picks);
  } catch (e) {
    reportError("error", e, "Dictionary");
  }
}

const baseName = (path: string): string => path.split(/[\\/]/).pop() ?? path;

// By default a drop on a group pins the files there too; the preference leaves them to the dictionary rules instead.
export async function dropIntoBatch(paths: string[], group: HTMLElement | null): Promise<void> {
  if (!batch) return;
  const folder = batch.folder;
  try {
    const added = inDownloads(folder) ? await api.dropIntoDownloads(folder, paths) : (await dropFiles(folder, paths), paths.map(baseName));
    const entry = group?.dataset.entry;
    if (entry && state.settings.batchDropToGroup && added.length > 0) await pinTo(entry, added);
    else if (batch) await replanBatch(batch.picks);
  } catch (e) {
    reportError("error", e, "Drop");
  }
}

// A tile dropped on a folder of the other pane moves the file itself there, under its current name; it leaves the batch with it.
export async function moveOut(files: string[][], target: string[]): Promise<void> {
  if (!batch) return;
  try {
    await moveDragged(files, target, false);
    await replanBatch(batch.picks);
  } catch (e) {
    reportError("error", e, "Move");
  }
}
