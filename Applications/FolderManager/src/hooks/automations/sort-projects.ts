// Groups a review folder's loose files into untracked workboards; nothing moves until the user approves the preview.
import * as api from "../../services/ipc/api";
import { confirmDialog } from "../../components/organisms/dialogs/info-dialog/info-dialog";
import { projectsDialog } from "../../components/organisms/dialogs/action-dialog/projects-dialog";
import { reportError, setStatus } from "../../components/organisms/bars/status-line";
import { plural } from "../../services/plural";
import { applyMutation, refresh } from "../ops";
import type { ReviewScope } from "./scope/review-scope";

export async function sortProjects(scope: ReviewScope): Promise<void> {
  const [folder] = scope.folders;
  if (!folder) return;
  try {
    const groups = await api.previewProjectGroups(folder);
    if (groups.length === 0) return setStatus("info", "No files look like one project.");
    const picked = await projectsDialog(groups);
    if (picked.length === 0) return;
    await applyMutation(await api.applyProjectGroups(folder, picked));
    setStatus("info", `Sorted ${plural(picked.length, "project")}.`);
  } catch (e) {
    reportError("error", e);
    await refresh();
  }
}

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
