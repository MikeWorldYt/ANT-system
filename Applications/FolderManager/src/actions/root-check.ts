// Folders outside the id format are announced on every check and settled in one dialog.
import * as api from "../ipc/api";
import { invalidDialog, type Decision } from "../ui/invalid-dialog";
import { notifyDesktop } from "../ui/desktop-notify";
import { currentOperationStatus, setStatus } from "../ui/status";
import { state } from "../store/state";
import { folderMoveActive } from "./folder-move";
import type { InvalidFolder, Resolution } from "../ipc/types";

type ResolveInvalid = (relParts: string[], resolution: Resolution) => Promise<unknown>;

let checking = false;

const dialogOpen = () => document.querySelector("dialog[open]") !== null;

function resolutionOf(decision: Decision): Resolution | null {
  if (decision.choice === "review") return decision.destination ? { kind: "review", destination: decision.destination } : null;
  if (decision.choice === "node") return { kind: "node" };
  if (decision.choice === "workboard") return { kind: "workboard", tracker: decision.tracker };
  return null;
}

export async function checkRootFolders(resolve: ResolveInvalid): Promise<void> {
  // An open dialog or a running drag or operation would stack a second modal; the next focus or refresh checks again.
  if (checking || dialogOpen() || folderMoveActive() || state.busy || currentOperationStatus() !== null) return;
  checking = true;
  try {
    const { invalid } = await api.checkRoot();
    if (invalid.length === 0) return;
    for (const folder of invalid) {
      await announce(folder);
    }
    const decisions = await invalidDialog(invalid, (relParts) => void api.openNode(relParts).catch((e: unknown) => setStatus("warning", api.describeError(e))));
    // Deepest first, so renaming or moving a parent never changes the path of a row still waiting.
    const ordered = [...(decisions ?? [])].sort((a, b) => b.relParts.length - a.relParts.length);
    for (const decision of ordered) {
      const resolution = resolutionOf(decision);
      if (!resolution) continue;
      try {
        await resolve(decision.relParts, resolution);
      } catch (e) {
        setStatus("warning", api.describeError(e));
      }
    }
  } catch (e) {
    setStatus("warning", `Storage check: ${api.describeError(e)}`);
  } finally {
    checking = false;
  }
}

async function announce({ relParts }: InvalidFolder): Promise<void> {
  const name = relParts[relParts.length - 1] ?? "";
  try {
    await notifyDesktop(`Invalid directory in your Storage found in ${name} (level ${relParts.length})`);
  } catch (e) {
    setStatus("warning", `Notification: ${api.describeError(e)}`);
  }
}
