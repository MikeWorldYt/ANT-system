// Backup page: automatic copies of the ANT data, where they go, a copy on demand and restoring one.
import { localStamp } from "../../../actions/stamp";
import { changeSetting, restoreBackupNamed } from "../../../actions/ops";
import { backupNow, backupSchedule, describeError, listBackups } from "../../../ipc/api";
import { state, type BackupFrequency } from "../../../store/state";
import { promptDialog, selectDialog } from "../../../ui/dialogs";
import { setStatus } from "../../../ui/status";
import { actions, buttonControl, pathControl, selectControl, switchControl, type RowHandle, type SettingsPage } from "../page";

const DEFAULT_FOLDER = "999.Settings/03.Backup";

async function report(fn: () => Promise<void>): Promise<void> {
  try {
    await fn();
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

// The in-app backups keep working where the OS task cannot be set, so its failure is only reported.
const schedule = (on: boolean) => report(() => backupSchedule(on));

async function toggleAuto(on: boolean): Promise<boolean> {
  if (!(await changeSetting("backupAuto", on))) return false;
  await schedule(on);
  return true;
}

async function changeFrequency(value: BackupFrequency): Promise<void> {
  if (!(await changeSetting("backupFrequency", value))) return;
  if (state.settings.backupAuto) await schedule(true);
}

async function chooseFolder(refresh: () => void): Promise<void> {
  const typed = await promptDialog("Backup folder", "Full path of the folder for the backups. It is created if it does not exist.", state.settings.backupFolder, "Save");
  if (typed === null) return;
  if (await changeSetting("backupFolder", typed)) refresh();
}

async function exportNow(row: RowHandle): Promise<void> {
  row.describe("Saving a copy…");
  try {
    const path = await backupNow(localStamp());
    row.describe(`Saved ${path.split(/[\\/]/).pop()}.`);
  } catch (e) {
    row.describe(describeError(e));
  }
}

async function importOne(row: RowHandle): Promise<void> {
  try {
    const found = await listBackups();
    if (found.length === 0) return row.describe("There are no backups in the backup folder.");
    const name = await selectDialog(
      "Restore a backup",
      "The ANT data and settings go back to that copy. A copy of the current state is saved first, so this can be undone.",
      found.map((b) => [b.name, b.name]),
      "Restore",
    );
    if (name === null) return;
    row.describe((await restoreBackupNamed(name, localStamp())) ? `Restored ${name}.` : "The restore failed.");
  } catch (e) {
    row.describe(describeError(e));
  }
}

export const backupPage: SettingsPage = {
  id: "backup",
  group: "Data",
  title: "Backup",
  icon: "pageBackup",
  rows: (ctx) => [
    {
      title: "Automatic backup",
      description: "Save a copy of the Storage at regular intervals.",
      control: () => switchControl("Automatic backup", state.settings.backupAuto, toggleAuto),
    },
    {
      title: "Frequency",
      control: () =>
        selectControl<BackupFrequency>(
          "Frequency",
          [
            ["day", "Every day"],
            ["week", "Every week"],
            ["month", "Every month"],
          ],
          state.settings.backupFrequency,
          (value) => void changeFrequency(value),
        ),
    },
    {
      title: "Backup folder",
      control: () =>
        actions(
          pathControl(state.settings.backupFolder || `${ctx.storage?.root ?? ""}/${DEFAULT_FOLDER}`),
          buttonControl("Change", () => void chooseFolder(ctx.refresh)),
          buttonControl("Default", () => void changeSetting("backupFolder", "").then(ctx.refresh)),
        ),
    },
    {
      title: "Export or import",
      description: "Move your Workboards to another computer.",
      control: (row: RowHandle) => actions(buttonControl("Export", () => void exportNow(row)), buttonControl("Import", () => void importOne(row))),
    },
  ],
};
