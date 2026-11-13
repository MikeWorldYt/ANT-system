// Puts the files of the last project sort back in their review folder.
import * as api from "../../services/ipc/api";
import { confirmDialog } from "../../components/organisms/dialogs/info-dialog/info-dialog";
import { reportError, setStatus } from "../../components/organisms/bars/status-line";
import { applyMutation, refresh } from "../ops";

export async function undoProjectSort(): Promise<void> {
  try {
    if (!(await api.canUndoProjectSort())) return;
    if (!(await confirmDialog("Undo last sort", "The files go back to the review folder and the new project folders are removed.", "Undo"))) return;
    await applyMutation(await api.undoProjectSort());
    setStatus("info", "The last sort was undone.");
  } catch (e) {
    reportError("error", e);
    await refresh();
  }
}
