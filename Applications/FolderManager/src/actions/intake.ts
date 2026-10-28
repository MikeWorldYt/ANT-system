// Runs the backend intake after a refresh and reports loose and moved files to the user.
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import * as api from "../ipc/api";
import { showNotificationDialog } from "../shell/notification-dialog";
import { setStatus } from "../ui/status";
import type { Moved } from "../ipc/types";

const LOOSE_MESSAGE = "Detecte un archivo sin clasificar dentro de tu Storage, porfavor muevelo a una categoria";

// Focus and manual refreshes can overlap; a second intake would repeat the same report.
let running = false;
// Refreshes repeat while the loose file stays; keep one dialog open or queued.
let loosePending = false;

async function notifyMoved(moved: Moved[]): Promise<void> {
  if (moved.length === 0) return;
  if (!(await isPermissionGranted()) && (await requestPermission()) !== "granted") return;
  for (const m of moved) {
    sendNotification({ title: "ANT System Manager", body: `${m.count} archivos en ${m.name} fueron movidos para su revision` });
  }
}

export async function runIntake(): Promise<void> {
  if (running) return;
  running = true;
  try {
    const report = await api.runIntake();
    if (report.warnings.length > 0) setStatus("warning", report.warnings.join(". "));
    if (report.loose.length > 0 && !loosePending) {
      loosePending = true;
      void showNotificationDialog({ message: LOOSE_MESSAGE })
        .then(() => {
          loosePending = false;
        })
        .catch((e: unknown) => {
          loosePending = false;
          setStatus("warning", api.describeError(e));
        });
    }
    await notifyMoved(report.moved);
  } catch (e) {
    setStatus("warning", api.describeError(e));
  } finally {
    running = false;
  }
}
