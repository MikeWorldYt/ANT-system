// Developer debug log: the backend writes the file, this names it with the local time and feeds it the errors the UI shows.
import { describeError, logDebug as writeLog, startDebugLog as startLog } from "../ipc/api";
import { state } from "../store/state";
import { localStamp } from "./stamp";
import { onStatus, setStatus } from "../ui/status";

export async function startDebugLog(): Promise<boolean> {
  try {
    await startLog(localStamp());
    return true;
  } catch (e) {
    setStatus("warning", `Debug log: ${describeError(e)}`);
    return false;
  }
}

function logDebug(level: "warning" | "error", message: string): void {
  if (!state.settings.debugLogging) return;
  writeLog(level, message).catch((e) => console.warn("Debug log write failed", e));
}

export function installDebugLog(): void {
  onStatus(logDebug);
  window.addEventListener("error", (ev) => logDebug("error", `${ev.message} (${ev.filename}:${ev.lineno})`));
  window.addEventListener("unhandledrejection", (ev) => logDebug("error", `Unhandled rejection: ${describeError(ev.reason)}`));
}
