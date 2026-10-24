// Automatic backups while the app is open: one check at launch, then one every hour.
import { backupIfDue, describeError } from "../ipc/api";
import { state } from "../store/state";
import { setStatus } from "../ui/status";
import { localStamp } from "./stamp";

const HOUR_MS = 3_600_000;

async function runIfDue(): Promise<void> {
  if (!state.settings.backupAuto) return;
  try {
    const saved = await backupIfDue(localStamp());
    if (saved) setStatus("info", `Backup saved: ${saved.split(/[\\/]/).pop()}`);
  } catch (e) {
    setStatus("warning", `Backup: ${describeError(e)}`);
  }
}

export function startBackups(): void {
  void runIfDue();
  setInterval(() => void runIfDue(), HOUR_MS);
}
