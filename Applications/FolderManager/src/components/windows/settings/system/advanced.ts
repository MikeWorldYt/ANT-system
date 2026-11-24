// Advanced page: the developer debug log, its folder, the webview console, AI agent write access, id administration and the way back to default preferences.
import { resetPreferences, changeSetting } from "../../../../hooks/ops";
import { startDebugLog } from "../../../../hooks/debug-log";
import { openDevtools, openLogsFolder, stopDebugLog } from "../../../../services/ipc/api";
import { state } from "../../../../services/store/state";
import { confirmDialog } from "../../../organisms/dialogs/info-dialog/info-dialog";
import { reportError } from "../../../organisms/bars/status-line";
import { buttonControl } from "../../../atoms/button";
import { switchControl } from "../../../atoms/switch";
import type { SettingsPage } from "../page";

async function report(fn: () => Promise<void>): Promise<void> {
  try {
    await fn();
  } catch (e) {
    reportError("warning", e);
  }
}

async function toggleLogging(on: boolean): Promise<boolean> {
  if (!(await changeSetting("debugLogging", on))) return false;
  if (on) await startDebugLog();
  else await report(stopDebugLog);
  return true;
}

async function toggleAgentWrites(on: boolean): Promise<boolean> {
  const message = "AI agents connected through MCP will be able to create, rename, move and change folders in this storage. Deletes, moves and changes to many items still wait for your OK in this window.";
  if (on && !(await confirmDialog("Let AI agents change this storage?", message, "Allow changes"))) return false;
  return changeSetting("mcpWrites", on);
}

export const advancedPage: SettingsPage = {
  id: "advanced",
  group: "System",
  title: "Advanced",
  icon: "pageAdvanced",
  rows: (ctx) => [
    { title: "Developer tools", control: () => buttonControl("Open devtools", () => void report(openDevtools)) },
    {
      title: "Debug logging",
      description: "Save more detail to help track down errors",
      control: () => switchControl("Debug logging", state.settings.debugLogging, toggleLogging),
    },
    { title: "Log files", control: () => buttonControl("Open logs folder", () => void report(openLogsFolder)) },
    {
      title: "Let AI agents change this storage",
      description: "Off: agents connected through MCP can only read. On: they can change folders, and deletes, moves and batches still ask you here first",
      control: () => switchControl("Let AI agents change this storage", state.settings.mcpWrites, toggleAgentWrites),
    },
    {
      title: "Administrate identifiers",
      description: "Adds \"Reorder by id…\" to the context menu to renumber a folder's items",
      control: () => switchControl("Administrate identifiers", state.settings.adminIdentifiers, (on) => changeSetting("adminIdentifiers", on)),
    },
    {
      title: "Reset preferences",
      description: "Does not delete your Worknodes, only the settings in this panel",
      control: () =>
        buttonControl(
          "Reset",
          async () => {
            if (!(await confirmDialog("Reset preferences", "Every setting in this window goes back to its default. Your Worknodes are not touched.", "Reset"))) return;
            if (await resetPreferences()) ctx.refresh();
          },
          "danger",
        ),
    },
  ],
};
