// Advanced page: the developer debug log, its folder, the webview console and the way back to default preferences.
import { resetPreferences, changeSetting } from "../../../../hooks/ops";
import { startDebugLog } from "../../../../hooks/debug-log";
import { describeError, openDevtools, openLogsFolder, stopDebugLog } from "../../../../services/ipc/api";
import { state } from "../../../../services/store/state";
import { confirmDialog } from "../../../organisms/dialogs/info-dialog/info-dialog";
import { setStatus } from "../../../organisms/bars/status-line";
import { buttonControl } from "../../../atoms/button";
import { switchControl } from "../../../atoms/switch";
import type { SettingsPage } from "../page";

async function report(fn: () => Promise<void>): Promise<void> {
  try {
    await fn();
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

async function toggleLogging(on: boolean): Promise<boolean> {
  if (!(await changeSetting("debugLogging", on))) return false;
  if (on) await startDebugLog();
  else await report(stopDebugLog);
  return true;
}

export const advancedPage: SettingsPage = {
  id: "advanced",
  group: "System",
  title: "Advanced",
  icon: "pageAdvanced",
  rows: (ctx) => [
    {
      title: "Debug logging",
      description: "Save more detail to help track down errors.",
      control: () => switchControl("Debug logging", state.settings.debugLogging, toggleLogging),
    },
    { title: "Log files", control: () => buttonControl("Open logs folder", () => void report(openLogsFolder)) },
    { title: "Developer tools", description: "Open the app console.", control: () => buttonControl("Open", () => void report(openDevtools)) },
    {
      title: "Reset preferences",
      description: "Does not delete your Workboards, only the settings in this panel.",
      control: () =>
        buttonControl(
          "Reset",
          async () => {
            if (!(await confirmDialog("Reset preferences", "Every setting in this window goes back to its default. Your Workboards are not touched.", "Reset"))) return;
            if (await resetPreferences()) ctx.refresh();
          },
          "danger",
        ),
    },
  ],
};
