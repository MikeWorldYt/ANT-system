// Ctrl+Z puts back the newest deletion while it waits in the trash, and otherwise undoes the last project sort.
import * as api from "../services/ipc/api";
import { reportError, setStatus } from "../components/organisms/bars/status-line";
import { undoProjectSort } from "./automations/sort-projects";
import { refresh } from "./ops";
import { select } from "../services/store/state";

let deletedLast = false;

export const noteDeleted = (): void => {
  deletedLast = true;
};

export async function undoLast(): Promise<void> {
  if (!deletedLast) return undoProjectSort();
  try {
    const back = await api.undoDelete();
    if (!back) {
      deletedLast = false;
      return undoProjectSort();
    }
    deletedLast = back.left > 0;
    await refresh();
    select(back.relParts);
    const more = back.left > 0 ? ` Ctrl+Z again restores ${back.left} more.` : "";
    setStatus("info", `Restored "${back.relParts.at(-1) ?? ""}".${more}`);
  } catch (e) {
    reportError("error", e, "Undo");
  }
}
