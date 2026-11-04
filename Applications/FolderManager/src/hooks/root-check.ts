// Folders outside the id format are announced on every check and settled in one dialog.
import * as api from "../services/ipc/api";
import { invalidDialog, type Decision } from "../components/organisms/dialogs/action-dialog/invalid-dialog";
import { notifyDesktop } from "../services/desktop-notify";
import { currentOperationStatus, reportError } from "../components/organisms/bars/status-line";
import { state } from "../services/store/state";
import { folderMoveActive } from "./folder-move";
import type { InvalidFolder, Resolution } from "../services/ipc/types";

type ResolveInvalid = (relParts: string[], resolution: Resolution) => Promise<boolean>;
type DeleteInvalid = (relParts: string[]) => Promise<boolean>;

let checking = false;
// Later and Escape give the user this long before the dialog and its announcement come back.
const SNOOZE_MS = 5 * 60_000;
let snoozedUntil = 0;

const dialogOpen = () => document.querySelector("dialog[open]") !== null;

function resolutionOf(decision: Decision): Resolution | null {
  if (decision.choice === "review") return decision.destination ? { kind: "review", destination: decision.destination } : null;
  if (decision.choice === "node") return { kind: "node" };
  if (decision.choice === "workboard") return { kind: "workboard", tracker: decision.tracker };
  return null;
}

export async function checkRootFolders(resolve: ResolveInvalid, remove: DeleteInvalid): Promise<void> {
  // An open dialog or a running drag or operation would stack a second modal; the next focus or refresh checks again.
  if (checking || dialogOpen() || folderMoveActive() || state.busy || currentOperationStatus() !== null) return;
  if (Date.now() < snoozedUntil) return;
  checking = true;
  try {
    const { invalid } = await api.checkRoot();
    if (invalid.length === 0) return;
    for (const folder of invalid) {
      await announce(folder);
    }
    const applyOne = async (decision: Decision): Promise<boolean> => {
      const resolution = resolutionOf(decision);
      if (!resolution) return true;
      try {
        return await resolve(decision.relParts, resolution);
      } catch (e) {
        reportError("warning", e);
        return false;
      }
    };
    const decisions = await invalidDialog(invalid, (relParts) => void api.openNode(relParts).catch((e: unknown) => reportError("warning", e)), previewNames, applyOne, remove);
    if (decisions === null) snoozedUntil = Date.now() + SNOOZE_MS;
    // Deepest first, so renaming or moving a parent never changes the path of a row still waiting.
    const ordered = [...(decisions ?? [])].sort((a, b) => b.relParts.length - a.relParts.length);
    for (const decision of ordered) {
      const resolution = resolutionOf(decision);
      if (!resolution) continue;
      try {
        await resolve(decision.relParts, resolution);
      } catch (e) {
        reportError("warning", e);
      }
    }
  } catch (e) {
    reportError("warning", e, "Storage check");
  } finally {
    checking = false;
  }
}

// A failed preview keeps the names as they are; the dialog still works, and the error goes to the status line.
async function previewNames(rows: { relParts: string[]; resolution: Resolution }[]): Promise<(string | null)[]> {
  try {
    return await api.previewInvalid(rows);
  } catch (e) {
    reportError("warning", e, "Preview");
    return rows.map(() => null);
  }
}

async function announce({ relParts }: InvalidFolder): Promise<void> {
  const name = relParts[relParts.length - 1] ?? "";
  try {
    await notifyDesktop(`Invalid directory in your Storage found in ${name} (level ${relParts.length})`);
  } catch (e) {
    reportError("warning", e, "Notification");
  }
}