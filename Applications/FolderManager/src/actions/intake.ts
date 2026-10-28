// Runs the backend intake after a refresh and reports loose and moved files to the user.
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import * as api from "../ipc/api";
import { setStatus } from "../ui/status";
import type { Moved } from "../ipc/types";

const LOOSE_MESSAGE = "I detected a unclassified file in your Storage, please move it to a category";

// Focus and manual refreshes can overlap; a second intake would repeat the same report.
let running = false;
// Refreshes repeat while loose files remain; report once until the Storage is clear again.
let looseNotified = false;

async function notificationPermissionGranted(): Promise<boolean> {
  return (await isPermissionGranted()) || (await requestPermission()) === "granted";
}

async function notifyMoved(moved: Moved[]): Promise<void> {
  if (moved.length === 0) return;
  if (!(await notificationPermissionGranted())) return;
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
    if (report.loose.length === 0) {
      looseNotified = false;
    } else if (!looseNotified) {
      looseNotified = true;
      if (await notificationPermissionGranted()) {
        sendNotification({ title: "ANT System Manager", body: LOOSE_MESSAGE });
      }
    }
    await notifyMoved(report.moved);
  } catch (e) {
    setStatus("warning", api.describeError(e));
  } finally {
    running = false;
  }
}
