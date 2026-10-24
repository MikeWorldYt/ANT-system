// Storage page: the Storage folder's name and location, reindexing and the daily check for outside changes.
import { localStamp } from "../../../actions/stamp";
import { changeSetting, refresh } from "../../../actions/ops";
import { startDebugLog } from "../../../actions/debug-log";
import { backupSchedule, describeError, renameStorage, revertStorageRename, storageRenameStatus, syncSchedule } from "../../../ipc/api";
import { state } from "../../../store/state";
import { confirmDialog, promptDialog } from "../../../ui/dialogs";
import { setStatus } from "../../../ui/status";
import { actions, buttonControl, pathControl, switchControl, valueControl, type RowHandle, type SettingsPage } from "../page";

const WARNING =
  "Renaming the Storage folder changes its path. A program that is reading it can stop the rename until you close it, and will break once the path changes. A log is saved so you can revert it.";

const folderName = (root: string | undefined) => root?.split(/[\/]/).filter(Boolean).pop() ?? "Unknown";

async function toggleWatch(on: boolean): Promise<boolean> {
  if (!(await changeSetting("watchExternal", on))) return false;
  try {
    await syncSchedule(on);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
  return true;
}

// The scheduled tasks and the open debug log still point at the old path.
async function followNewPath(): Promise<void> {
  try {
    if (state.settings.backupAuto) await backupSchedule(true);
    if (state.settings.watchExternal) await syncSchedule(true);
  } catch (e) {
    setStatus("warning", `Scheduled tasks: ${describeError(e)}`);
  }
  if (state.settings.debugLogging) await startDebugLog();
}

async function rename(current: string, row: RowHandle, done: () => void): Promise<void> {
  const typed = await promptDialog("Rename Storage", "New name for the Storage folder.", current, "Continue");
  if (typed === null || typed === "" || typed === current) return;
  if (!(await confirmDialog("Rename the Storage folder?", WARNING, "Rename"))) return;
  try {
    await renameStorage(typed, localStamp());
    await refresh();
    await followNewPath();
    done();
  } catch (e) {
    row.describe(describeError(e));
  }
}

async function revert(row: RowHandle, done: () => void): Promise<void> {
  try {
    await revertStorageRename(localStamp());
    await refresh();
    await followNewPath();
    done();
  } catch (e) {
    row.describe(describeError(e));
  }
}

function renameControl(root: string | undefined, row: RowHandle, done: () => void): HTMLElement {
  const name = folderName(root);
  const undo = buttonControl("Revert last rename", () => void revert(row, done));
  undo.disabled = true;
  void storageRenameStatus().then(
    (status) => {
      if (!status) return;
      undo.disabled = !status.revertible;
      row.describe(`Last rename: ${status.old} → ${status.new}`);
    },
    (e) => row.describe(describeError(e)),
  );
  return actions(valueControl(name), buttonControl("Rename", () => void rename(name, row, done)), undo);
}

export const storagePage: SettingsPage = {
  id: "storage",
  group: "Data",
  title: "Storage",
  icon: "pageStorage",
  rows: (ctx) => [
    {
      title: "Storage name",
      description: "The name of the Storage folder.",
      control: (row: RowHandle) => renameControl(ctx.storage?.root, row, ctx.refresh),
    },
    { title: "Storage location", description: "Folder where ANT keeps your Workboards.", control: () => pathControl(ctx.storage?.root ?? "Unknown") },
    {
      title: "Reindex Storage",
      description: "Use this if you moved files outside of ANT.",
      control: (row: RowHandle) =>
        buttonControl("Reindex", () => {
          row.describe("Reindexing…");
          void refresh().then((ok) => row.describe(ok ? "Reindexed." : "The reindex failed."));
        }),
    },
    {
      title: "Watch for external changes",
      description: "Check every day if files changed on disk and update the Storage.",
      control: () => switchControl("Watch for external changes", state.settings.watchExternal, toggleWatch),
    },
  ],
};
