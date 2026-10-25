// Before the tree loads: offers the way out when the Storage has no usable database or cannot be found.
import * as api from "../ipc/api";
import { choiceDialog, promptDialog } from "../ui/dialogs";
import { setStatus } from "../ui/status";
import { localStamp } from "./stamp";

async function changePath(current: string): Promise<void> {
  const typed = await promptDialog("Change path", "Full path of the Storage folder.", current, "Use this path");
  if (typed === null || typed === "") return;
  try {
    await api.changeStoragePath(typed);
  } catch (e) {
    setStatus("error", api.describeError(e));
  }
}

async function scan(): Promise<boolean> {
  try {
    await api.prepareScan(localStamp());
    return true;
  } catch (e) {
    setStatus("error", api.describeError(e));
    return false;
  }
}

async function useBackup(): Promise<void> {
  try {
    await api.useLatestBackup(localStamp());
  } catch (e) {
    setStatus("error", api.describeError(e));
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
    const choice = await choiceDialog(
      "Storage not found",
      `Hmm, it looks like ANT has not been initialized or the Storage is wrong.\n\nPath: ${health.path}`,
      [
        ["change", "Change path"],
        ["scan", "Scan Storage"],
      ],
    );
    if (choice === "change") await changePath(health.path);
    else if (choice === "scan") {
      if (health.state === "noDatabase") return scan();
      setStatus("warning", `No Storage found at ${health.path}`);
      return false;
    } else return false;
  }
}
