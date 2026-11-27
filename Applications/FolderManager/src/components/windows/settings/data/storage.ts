// Storage page: the Storage folder's name and location, the tree depth, reindexing and the daily check for outside changes.
import { localStamp } from "../../../../services/stamp";
import { changeSetting, changeTreeDepth, refresh } from "../../../../hooks/ops";
import { afterPick, followNewPath } from "../../../../hooks/follow-storage";
import { describeError, renameStorage, revertStorageRename, storageRenameStatus, syncSchedule } from "../../../../services/ipc/api";
import { state } from "../../../../services/store/state";
import { confirmDialog } from "../../../organisms/dialogs/info-dialog/info-dialog";
import { promptDialog } from "../../../organisms/dialogs/action-dialog/form-dialog";
import { storagePicker } from "../../../organisms/dialogs/storage-picker/storage-picker";
import { reportError } from "../../../organisms/bars/status-line";
import type { StorageInfo } from "../../../../services/ipc/types";
import { buttonControl } from "../../../atoms/button";
import { pathControl, valueControl } from "../../../atoms/label";
import { selectControl } from "../../../atoms/select";
import { switchControl } from "../../../atoms/switch";
import { actions } from "../../../molecules/button-row";
import type { RowHandle } from "../../../molecules/setting-row";
import type { SettingsPage } from "../page";

const WARNING =
  "Renaming the Storage folder changes its path. A program that is reading it can stop the rename until you close it, and will break once the path changes. A log is saved so you can revert it.";

const folderName = (root: string | undefined) => root?.split(/[\\/]/).filter(Boolean).pop() ?? "Unknown";

async function toggleWatch(on: boolean): Promise<boolean> {
  if (!(await changeSetting("watchExternal", on))) return false;
  try {
    await syncSchedule(on);
  } catch (e) {
    reportError("warning", e);
  }
  return true;
}

async function rename(current: string, row: RowHandle, done: () => void): Promise<void> {
  const typed = await promptDialog("Rename Storage", "New name for the Storage folder.", current, "Continue");
  if (typed === null || typed === "" || typed === current) return;
  if (!(await confirmDialog("Rename the Storage folder?", WARNING, "Rename"))) return;
  try {
    await renameStorage(typed, localStamp());
    await refresh();
    await followNewPath();
    done();
  } catch (e) {
    row.describe(describeError(e));
  }
}

async function revert(row: RowHandle, done: () => void): Promise<void> {
  try {
    await revertStorageRename(localStamp());
    await refresh();
    await followNewPath();
    done();
  } catch (e) {
    row.describe(describeError(e));
  }
}

async function switchStorage(current: string | null, row: RowHandle, done: () => void): Promise<void> {
  const picked = await storagePicker(current);
  if (!picked) return;
  try {
    await afterPick(picked);
    done();
  } catch (e) {
    row.describe(describeError(e));
  }
}

const fixed = (button: HTMLButtonElement): HTMLButtonElement => {
  button.classList.add("sw-fixed");
  return button;
};

function renameControl(root: string | undefined, row: RowHandle, done: () => void): HTMLElement {
  const name = folderName(root);
  const undo = fixed(buttonControl("Revert last rename", () => void revert(row, done)));
  undo.disabled = true;
  void storageRenameStatus().then(
    (status) => {
      if (!status) return;
      undo.disabled = !status.revertible;
      row.describe(`Last rename: ${status.old} → ${status.new}`);
    },
    (e) => row.describe(describeError(e)),
  );
  const chip = valueControl(name);
  chip.classList.add("sw-name");
  chip.title = name;
  return actions(chip, fixed(buttonControl("Rename", () => void rename(name, row, done))), undo);
}

function depthControl(storage: StorageInfo | null, row: RowHandle, done: () => void): HTMLElement {
  if (!storage) return valueControl("Unknown");
  if (storage.depthWarning) row.describe(storage.depthWarning);
  const [fallback] = storage.depthOptions;
  const options = storage.depthOptions.map((n): [string, string] => [String(n), n === fallback ? `${n} levels (default)` : `${n} levels`]);
  return selectControl("Index", options, String(storage.maxDepth), (value) => {
    row.describe("Reloading the tree…");
    // Re-reading the storage info shows the depth that was really saved, also after a failure.
    void changeTreeDepth(Number(value)).then(done);
  });
}

export const storagePage: SettingsPage = {
  id: "storage",
  group: "Data",
  title: "Storage",
  icon: "pageStorage",
  rows: (ctx) => [
    {
      section: "Storage",
      title: "Storage name",
      control: (row: RowHandle) => renameControl(ctx.storage?.root, row, ctx.refresh),
    },
    {
      section: "Storage",
      title: "Storage location",
      description: "Folder where the app opens your storage",
      control: (row: RowHandle) => actions(pathControl(ctx.storage?.root ?? "Unknown"), fixed(buttonControl("Switch", () => void switchStorage(ctx.storage?.root ?? null, row, ctx.refresh)))),
    },
    {
      section: "Index",
      title: "Index",
      description: "How many tree levels are indexed in the database; more levels read more folders on each refresh",
      control: (row: RowHandle) => depthControl(ctx.storage, row, ctx.refresh),
    },
    {
      section: "Index",
      title: "Reindex Storage",
      description: "Rebuild the index from disk",
      control: (row: RowHandle) =>
        fixed(
          buttonControl("Reindex", () => {
            row.describe("Reindexing…");
            void refresh().then((ok) => row.describe(ok ? "Reindexed." : "The reindex failed."));
          }),
        ),
    },
    {
      section: "External changes",
      title: "Watch for external changes",
      description: "Check once a day for changes made on disk and update the database",
      control: () => switchControl("Watch for external changes", state.settings.watchExternal, toggleWatch),
    },
  ],
};
