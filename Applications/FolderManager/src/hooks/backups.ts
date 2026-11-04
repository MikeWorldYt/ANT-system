// Automatic backups while the app is open: one check at launch, then one every hour.
import { backupIfDue } from "../services/ipc/api";
import { state } from "../services/store/state";
import { notifyDesktop } from "../services/desktop-notify";
import { reportError } from "../components/organisms/bars/status-line";
import { localStamp } from "../services/stamp";

const HOUR_MS = 3_600_000;

async function runIfDue(): Promise<void> {
  if (!state.settings.backupAuto) return;
  try {
    const saved = await backupIfDue(localStamp());
    if (saved) await notifyDesktop(`Backup saved: ${saved.split(/[\\/]/).pop()}`);
  } catch (e) {
    reportError("warning", e, "Backup");
  }
}

export function startBackups(): void {
  void runIfDue();
  setInterval(() => void runIfDue(), HOUR_MS);
}
