// Offers a newer published version, at launch or on demand; failures only reach the status line.
import { checkUpdate, describeError, skipVersion, startUpdate } from "../ipc/api";
import { choiceDialog } from "../ui/dialogs";
import { setStatus } from "../ui/status";

export async function checkForUpdates(announceCurrent = false): Promise<void> {
  try {
    const info = await checkUpdate(announceCurrent);
    if (!info) {
      if (announceCurrent) setStatus("info", "You have the latest version");
      return;
    }
    const choice = await choiceDialog("New version available", `Current: ${info.current}\nLatest: ${info.latest}\nThe app will close to apply the update.`, [
      ["later", "Remind me later"],
      ["skip", "Skip version"],
      ["update", "Update"],
    ]);
    if (choice === "update") await startUpdate();
    else if (choice === "skip") await skipVersion(info.latest);
  } catch (e) {
    setStatus("warning", `Update check: ${describeError(e)}`);
  }
}
