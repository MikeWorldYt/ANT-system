// Before the tree loads: offers the way out when the Storage has no usable database or cannot be found.
import * as api from "../services/ipc/api";
import { choiceDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { promptDialog } from "../components/organisms/dialogs/action-dialog/form-dialog";
import { setStatus, reportError } from "../components/organisms/bars/status-line";
import { localStamp } from "../services/stamp";

// The settings are not loaded yet at this point, so the new Storage's own values decide which tasks follow it.
async function followNewPath(): Promise<void> {
  try {
    const saved = await api.getSettings();
    if (saved.backupAuto === "true") await api.backupSchedule(true);
    if (saved.watchExternal !== "false") await api.syncSchedule(true);
  } catch (e) {
    reportError("warning", e, "Scheduled tasks");
  }
}

async function changePath(current: string): Promise<void> {
  const typed = await promptDialog("Change path", "Full path of the Storage folder.", current, "Use this path");
  if (typed === null || typed === "") return;
  try {
    await api.changeStoragePath(typed);
  } catch (e) {
    reportError("error", e);
    return;
  }
  await followNewPath();
}

async function scan(): Promise<boolean> {
  try {
    await api.prepareScan(localStamp());
    return true;
  } catch (e) {
    reportError("error", e);
    return false;
  }
}

async function useBackup(): Promise<void> {
  try {
    await api.useLatestBackup(localStamp());
  } catch (e) {
    reportError("error", e);
  }
}

// True when the tree can be loaded now; false leaves the app empty with a message in the status line.
export async function checkStorage(): Promise<boolean> {
  for (;;) {
    const health = await api.storageHealth();
    if (health.state === "ok") return true;
    if (health.state === "noDatabase" && health.backup) {
      const choice = await choiceDialog(
        "Database problem",
        `I detected a database failure, but I found the backup ${health.backup.label}.`,
        [
          ["scan", "Scan Storage"],
          ["backup", "Use backup"],
        ],
      );
      if (choice === "scan") return scan();
      if (choice === "backup") await useBackup();
      else return false;
      continue;
    }
    const noDatabase = health.state === "noDatabase";
    const choice = await choiceDialog(
      noDatabase ? "Database problem" : "Storage not found",
      noDatabase
        ? `I found no usable database and no backup. Scan Storage builds one if it is missing or damaged, and keeps one that is only locked or from a newer version.\n\nPath: ${health.path}`
        : `Hmm, it looks like ANT has not been initialized or the Storage is wrong.\n\nPath: ${health.path}`,
      [
        ["change", "Change path"],
        ["scan", "Scan Storage"],
      ],
    );
    if (choice === "change") await changePath(health.path);
    else if (choice === "scan") {
      if (noDatabase) return scan();
      setStatus("warning", `No Storage found at ${health.path}`);
      return false;
    } else return false;
  }
}
