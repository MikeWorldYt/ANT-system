// Brings the app onto another Storage after the picker or the switcher changed it: reload, then re-point the tasks tied to the path.
import { localStamp } from "../services/stamp";
import { backupSchedule, prepareScan, syncSchedule } from "../services/ipc/api";
import { state } from "../services/store/state";
import { reportError } from "../components/organisms/bars/status-line";
import type { PickResult } from "../components/organisms/dialogs/storage-picker/storage-picker";
import { startDebugLog } from "./debug-log";
import { refresh } from "./ops";

// The scheduled tasks and the open debug log still point at the old path.
export async function followNewPath(): Promise<void> {
  try {
    if (state.settings.backupAuto) await backupSchedule(true);
    if (state.settings.watchExternal) await syncSchedule(true);
  } catch (e) {
    reportError("warning", e, "Scheduled tasks");
  }
  if (state.settings.debugLogging) await startDebugLog();
}

export async function afterPick(picked: Exclude<PickResult, null>): Promise<void> {
  if (picked === "created") await prepareScan(localStamp());
  await refresh();
  await followNewPath();
}
