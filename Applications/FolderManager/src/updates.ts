// Offers a newer published version once at launch; failures only reach the status line.
import { checkUpdate, describeError, skipVersion, startUpdate } from "./api";
import { choiceDialog } from "./dialogs";
import { setStatus } from "./status";

export async function checkForUpdates(): Promise<void> {
  try {
    const info = await checkUpdate();
    if (!info) return;
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
