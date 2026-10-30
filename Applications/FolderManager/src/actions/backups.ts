// Automatic backups while the app is open: one check at launch, then one every hour.
import { backupIfDue, describeError } from "../ipc/api";
import { state } from "../store/state";
import { notifyDesktop } from "../ui/desktop-notify";
import { setStatus } from "../ui/status";
import { localStamp } from "./stamp";

const HOUR_MS = 3_600_000;

async function runIfDue(): Promise<void> {
  if (!state.settings.backupAuto) return;
  try {
    const saved = await backupIfDue(localStamp());
    if (saved) await notifyDesktop(`Backup saved: ${saved.split(/[\\/]/).pop()}`);
  } catch (e) {
    setStatus("warning", `Backup: ${describeError(e)}`);
  }
}

export function startBackups(): void {
  void runIfDue();
  setInterval(() => void runIfDue(), HOUR_MS);
}
