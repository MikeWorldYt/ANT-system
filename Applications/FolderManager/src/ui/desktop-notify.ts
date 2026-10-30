// Shared desktop notification sender so every caller asks for permission the same way.
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

async function notificationPermissionGranted(): Promise<boolean> {
  return (await isPermissionGranted()) || (await requestPermission()) === "granted";
}

export async function notifyDesktop(body: string): Promise<void> {
  if (!(await notificationPermissionGranted())) return;
  sendNotification({ title: "ANT System Manager", body });
}
