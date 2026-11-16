// Before the tree loads: offers the way out when the Storage has no usable database or cannot be found.
import * as api from "../services/ipc/api";
import { choiceDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { storagePicker } from "../components/organisms/dialogs/storage-picker/storage-picker";
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
    if (health.state === "notFound") {
      const picked = await storagePicker(null, true);
      if (!picked) {
        setStatus("warning", `No Storage found at ${health.path}`);
        return false;
      }
      await followNewPath();
      if (picked === "created") return scan();
      continue;
    }
    const choice = await choiceDialog(
      "Database problem",
      `I found no usable database and no backup. Scan Storage builds one if it is missing or damaged, and keeps one that is only locked or from a newer version.\n\nPath: ${health.path}`,
      [
        ["switch", "Other Storage"],
        ["scan", "Scan Storage"],
      ],
    );
    if (choice === "scan") return scan();
    if (choice !== "switch") return false;
    const picked = await storagePicker(health.path);
    if (!picked) return false;
    await followNewPath();
    if (picked === "created") return scan();
  }
}
