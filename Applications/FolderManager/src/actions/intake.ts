// Runs the backend intake after a refresh and reports loose and moved files to the user.
import * as api from "../ipc/api";
import { notifyDesktop } from "../ui/desktop-notify";
import { setStatus } from "../ui/status";
import { waitingIntakeDialog } from "../ui/waiting-intake-dialog";
import type { Moved, WaitingNode } from "../ipc/types";

// Focus and manual refreshes can overlap; a second intake would repeat the same report.
let running = false;
// Refreshes repeat while loose files remain; report once until the Storage is clear again.
let looseNotified = false;

async function notifyMoved(moved: Moved[]): Promise<void> {
  for (const m of moved) {
    const files = `${m.count} file${m.count === 1 ? "" : "s"}`;
    await notifyDesktop(`${files} from ${m.name} moved to ${m.review} for review.`);
  }
}

function looseFilesMessage(files: { node: string; file: string }[]): string {
  const inRoot = files.every(({ node }) => node.length === 0);
  const shown = files.slice(0, 5).map(({ node, file }) => (node ? `${node}/${file}` : `Storage root/${file}`));
  const omitted = files.length - shown.length;
  if (omitted > 0) shown.push(`...and ${omitted} more`);
  const subject = files.length === 1 ? "file" : "files";
  const pronoun = files.length === 1 ? "it" : "them";
  const location = inRoot ? " in Storage root" : " in your Storage";
  return `I detected ${files.length} unclassified ${subject}${location}. Please move ${pronoun} to a category:\n${shown.join("\n")}`;
}

// Nodes the app has never managed wait for the answer; returns whether any answer was applied.
async function answerWaiting(nodes: WaitingNode[], reload: () => Promise<void>): Promise<boolean> {
  const decisions = await waitingIntakeDialog(nodes);
  if (!decisions) return false;
  let applied = false;
  for (const { relParts, choice } of decisions) {
    try {
      if (choice === "data") await api.setUserFlag(relParts, true);
      else await api.acknowledgeIntake(relParts);
      applied = true;
    } catch (e) {
      setStatus("warning", api.describeError(e));
    }
  }
  if (applied) await reload();
  return applied;
}

export async function runIntake(reload: () => Promise<void>): Promise<void> {
  if (running) return;
  running = true;
  try {
    let report = await api.runIntake();
    if (report.waiting.length > 0 && (await answerWaiting(report.waiting, reload))) {
      report = await api.runIntake();
    }
    if (report.warnings.length > 0) setStatus("warning", report.warnings.join(". "));
    if (report.loose.length === 0) {
      looseNotified = false;
    } else if (!looseNotified) {
      looseNotified = true;
      console.warn("[intake] Unclassified files detected:", report.loose);
      await notifyDesktop(looseFilesMessage(report.loose));
    }
    await notifyMoved(report.moved);
  } catch (e) {
    setStatus("warning", api.describeError(e));
  } finally {
    running = false;
  }
}
