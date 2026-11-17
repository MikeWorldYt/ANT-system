// The batch grid's menu: one file can be revealed, renamed to help the batch, or deleted; several can be unticked or deleted together.
import { closingItem, separator } from "../../molecules/menu";
import { openMenuWith } from "../../organisms/menus/context-menu";
import { promptDialog } from "../../organisms/dialogs/action-dialog/form-dialog";
import { warningConfirmDialog } from "../../organisms/dialogs/info-dialog/info-dialog";
import { reportError } from "../../organisms/bars/status-line";
import * as api from "../../../services/ipc/api";
import { batch, replanBatch } from "../../../hooks/automations/batch";
import { plural } from "../../../services/plural";
import { inDownloads } from "../../../services/ipc/types";
import { state } from "../../../services/store/state";
import { setRowTicked, type BatchRow } from "./batch-row";

// A change on disk is read again by a fresh dry run with the same picks.
async function change(work: () => Promise<unknown>): Promise<void> {
  try {
    await work();
    if (batch) await replanBatch(batch.picks);
  } catch (e) {
    reportError("error", e);
  }
}

const rename = (parts: string[], name: string): Promise<unknown> => (inDownloads(parts) ? api.renameDownload(parts, name) : api.renameFile(parts, name));

async function remove(all: string[][]): Promise<void> {
  const what = all.length === 1 ? `"${all[0]?.at(-1) ?? ""}" goes` : `${plural(all.length, "file")} go`;
  if (state.settings.confirmDelete && !(await warningConfirmDialog("Delete", [`${what} to the Recycle Bin`], "Delete", "danger"))) return;
  const downloads = all.filter(inDownloads);
  if (downloads.length > 0) await api.deleteDownloads(downloads);
  for (const parts of all.filter((p) => !inDownloads(p))) await api.deleteFile(parts);
}

export function tileMenu(ev: MouseEvent, picked: { parts: string[]; row: BatchRow }[], restore: () => void): void {
  ev.preventDefault();
  const [one] = picked;
  if (!one) return;
  openMenuWith(
    (close) => {
      const action = closingItem(close);
      if (picked.length > 1) {
        // The larger side decides: mostly ticked offers Deselect, otherwise Select; a tie selects.
        const live = picked.filter((p) => !p.row.box.disabled);
        const tick = live.filter((p) => p.row.box.checked).length * 2 <= live.length;
        const untick = action(tick ? "Select" : "Deselect", live.length > 0, () =>
          live.forEach((p) => {
            setRowTicked(p.row, tick);
            // The tile, its group box and the header box follow the change event, not the property.
            p.row.box.dispatchEvent(new Event("change"));
          }),
        );
        const del = action("Delete selected", true, () => void change(() => remove(picked.map((p) => p.parts))));
        return { items: [untick, del], subs: [], content: [untick, separator(), del] };
      }
      const name = one.row.file.file;
      const reveal = action("Open folder location", true, () => void api.revealNode(one.parts).catch((e: unknown) => reportError("error", e)));
      const renameItem = action("Rename", true, () =>
        void promptDialog("Rename", "New name for the file.", name, "Rename").then((next) => {
          if (next && next.trim() !== name) void change(() => rename(one.parts, next.trim()));
        }),
      );
      const del = action("Delete", true, () => void change(() => remove([one.parts])));
      return { items: [reveal, renameItem, del], subs: [], content: [reveal, renameItem, separator(), del] };
    },
    picked.length > 1 ? `${picked.length} files` : one.row.file.file,
    ev.clientX,
    ev.clientY,
    restore,
  );
}
