// One-time move to the word support ids: the names settle every folder, and only resource folders wait for the user.
import * as api from "../services/ipc/api";
import { supportUpgradeDialog } from "../components/organisms/dialogs/action-dialog/support-upgrade-dialog";
import { setStatus, reportError } from "../components/organisms/bars/status-line";
import type { SupportCheck } from "../services/ipc/types";

type Apply = (relParts: string[]) => Promise<boolean>;
type Finish = () => Promise<boolean>;

// Later asks again after this long in the same session; nothing is stored, so the next launch asks anyway.
const LATER_MS = 60 * 60_000;
let running = false;

// A folder that becomes a regular node opens up the support folders inside it, so the check runs until none is left.
async function settleByName(apply: Apply): Promise<SupportCheck | null> {
  for (;;) {
    const check = await api.checkSupportUpgrade();
    const folders = check.folders.filter((folder) => !folder.notice);
    if (folders.length === 0) return check;
    for (const folder of folders) if (!(await apply(folder.relParts))) return null;
  }
}

export async function checkSupportUpgrade(apply: Apply, finish: Finish): Promise<void> {
  if (running) return;
  running = true;
  try {
    const check = await settleByName(apply);
    if (!check?.needed) return;
    if (check.folders.length > 0) {
      const showFolder = (relParts: string[]) => void api.revealNode(relParts).catch((e: unknown) => reportError("warning", e));
      const accepted = await supportUpgradeDialog(check.folders, showFolder);
      let settled = accepted;
      for (const folder of accepted ? check.folders : []) settled = (await apply(folder.relParts)) && settled;
      if (!settled) {
        setTimeout(() => void checkSupportUpgrade(apply, finish), LATER_MS);
        return;
      }
    }
    setStatus("info", "Updating the storage root to the new support nodes…");
    await finish();
  } catch (e) {
    reportError("warning", e, "Support nodes update");
  } finally {
    running = false;
  }
}
