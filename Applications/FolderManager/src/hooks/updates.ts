// Offers a newer published version, at launch or on demand; failures only reach the status line.
import { checkUpdate, skipVersion, startUpdate } from "../services/ipc/api";
import { choiceDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { setStatus, reportError } from "../components/organisms/bars/status-line";

export type UpdateOutcome = "latest" | "offered" | "failed";

export async function checkForUpdates(announceCurrent = false): Promise<UpdateOutcome> {
  try {
    const info = await checkUpdate(announceCurrent);
    if (!info) {
      if (announceCurrent) setStatus("info", "You have the latest version");
      return "latest";
    }
    const choice = await choiceDialog("New version available", `Current: ${info.current}\nLatest: ${info.latest}\nThe app will close to apply the update.`, [
      ["later", "Remind me later"],
      ["skip", "Skip version"],
      ["update", "Update"],
    ]);
    if (choice === "update") await startUpdate();
    else if (choice === "skip") await skipVersion(info.latest);
    return "offered";
  } catch (e) {
    reportError("warning", e, "Update check");
    return "failed";
  }
}
