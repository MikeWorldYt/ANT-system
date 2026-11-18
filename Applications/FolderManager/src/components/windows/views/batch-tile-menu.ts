// The batch grid's menu: one file can be opened, revealed, renamed to help the batch, or deleted; several can be unticked, tagged with a prefix or suffix, or deleted together.
import { closingItem, separator } from "../../molecules/menu";
import { openMenuWith } from "../../organisms/menus/context-menu";
import { promptDialog } from "../../organisms/dialogs/action-dialog/form-dialog";
import { renameInPlace } from "./batch-tile-rename";
import { warningConfirmDialog } from "../../organisms/dialogs/info-dialog/info-dialog";
import { reportError } from "../../organisms/bars/status-line";
import * as api from "../../../services/ipc/api";
import { batch, replanBatch } from "../../../hooks/automations/batch";
import { plural } from "../../../services/plural";
import { inDownloads } from "../../../services/ipc/types";
import { keyOf, state } from "../../../services/store/state";
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

// A suffix goes before the extension and any " (2)" copy counter, so the file keeps its type and its place among its copies.
function affixed(name: string, prefix: string, suffix: string): string {
  const dot = name.lastIndexOf(".");
  const ext = dot > 0 ? name.slice(dot) : "";
  const stem = name.slice(0, name.length - ext.length);
  const counter = / \(\d+\)$/.exec(stem)?.[0] ?? "";
  return `${prefix}${stem.slice(0, stem.length - counter.length)}${suffix}${counter}${ext}`;
}

function affix(all: string[][], side: "prefix" | "suffix"): void {
  const label = side === "prefix" ? "Add prefix" : "Add suffix";
  void promptDialog(label, `Text added to the ${side === "prefix" ? "start" : "end"} of ${plural(all.length, "file")}.`, "", label).then((text) => {
    if (!text) return;
    void change(async () => {
      for (const parts of all) {
        const name = parts.at(-1) ?? "";
        await rename(parts, side === "prefix" ? affixed(name, text, "") : affixed(name, "", text));
      }
    });
  });
}

export const renameTile = (tile: HTMLElement, parts: string[]): void => renameInPlace(tile, parts.at(-1) ?? "", (next) => void change(() => rename(parts, next)));

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
        const all = picked.map((p) => p.parts);
        const prefix = action("Add prefix…", true, () => affix(all, "prefix"));
        const suffix = action("Add suffix…", true, () => affix(all, "suffix"));
        const del = action("Delete selected", true, () => void change(() => remove(all)));
        return { items: [untick, prefix, suffix, del], subs: [], content: [untick, separator(), prefix, suffix, separator(), del] };
      }
      const open = action("Open", true, () => void api.openNode(one.parts).catch((e: unknown) => reportError("error", e)));
      const reveal = action("Open folder location", true, () => void api.revealNode(one.parts).catch((e: unknown) => reportError("error", e)));
      // The menu hands focus back to the tile as it closes, so the editor opens after that or it would close at once.
      const renameItem = action("Rename", true, () =>
        setTimeout(() => {
          const tile = document.querySelector<HTMLElement>(`.batch-tile[data-key="${CSS.escape(keyOf(one.parts))}"]`);
          if (tile) renameTile(tile, one.parts);
        }),
      );
      const del = action("Delete", true, () => void change(() => remove([one.parts])));
      return { items: [open, reveal, renameItem, del], subs: [], content: [open, reveal, renameItem, separator(), del] };
    },
    picked.length > 1 ? `${picked.length} files` : one.row.file.file,
    ev.clientX,
    ev.clientY,
    restore,
  );
}
