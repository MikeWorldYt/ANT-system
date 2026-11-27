// Offers a newer published version, at launch or on demand, or installs a stable one unasked when the user lets it; failures only reach the status line.
import { checkUpdate, skipVersion, startUpdate } from "../services/ipc/api";
import { state } from "../services/store/state";
import { choiceDialog } from "../components/organisms/dialogs/info-dialog/info-dialog";
import { setStatus, reportError } from "../components/organisms/bars/status-line";

export type UpdateOutcome = "latest" | "offered" | "failed";

export async function checkForUpdates(announceCurrent = false): Promise<UpdateOutcome> {
  try {
    const beta = state.settings.betaUpdates;
    const info = await checkUpdate(announceCurrent, beta);
    if (!info) {
      if (announceCurrent) setStatus("info", "You have the latest version");
      return "latest";
    }
    // The launch check installs a stable version at once, before any work is open; a beta or a manual check still asks.
    const stable = !info.latest.includes("-");
    if (!announceCurrent && stable && state.settings.autoUpdate && state.settings.installUpdates) {
      setStatus("info", `Updating to ${info.latest}…`);
      await startUpdate(beta);
      return "offered";
    }
    const choice = await choiceDialog(stable ? "New version available" : "New beta version available", `Current: ${info.current}\nLatest: ${info.latest}\nThe app will close to apply the update.`, [
      ["later", "Remind me later"],
      ["skip", "Skip version"],
      ["update", "Update"],
    ]);
    if (choice === "update") await startUpdate(beta);
    else if (choice === "skip") await skipVersion(info.latest);
    return "offered";
  } catch (e) {
    reportError("warning", e, "Update check");
    return "failed";
  }
}
