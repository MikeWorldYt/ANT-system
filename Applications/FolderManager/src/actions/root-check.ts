// Level-1 folders must use the category format; each stray one is announced and can be moved into a review folder.
import * as api from "../ipc/api";
import { choiceDialog, selectDialog } from "../ui/dialogs";
import { notifyDesktop } from "../ui/desktop-notify";
import { currentOperationStatus, setStatus } from "../ui/status";
import { state } from "../store/state";
import { folderMoveActive } from "./folder-move";

type MoveToReview = (name: string, destination: string[]) => Promise<boolean>;

const TITLE = "Invalid directory found";

let checking = false;

const dialogOpen = () => document.querySelector("dialog[open]") !== null;

async function askAbout(name: string, reviews: string[][], moveToReview: MoveToReview): Promise<void> {
  const message = `"${name}" is a level-1 folder outside the category format (100, 200, … 800; the 900 range belongs to the system).`;
  if (reviews.length === 0) {
    await choiceDialog(TITLE, `${message} No Review folder exists yet.`, [["ignore", "Ignore"]]);
    return;
  }
  const answer = await choiceDialog(TITLE, message, [["ignore", "Ignore"], ["move", "Move to a Review Node"]]);
  if (answer !== "move") return;
  const options = reviews.map((parts, i): [string, string] => [String(i), parts.join("/")]);
  const picked = await selectDialog("Move to a Review Node", `Choose the review folder that takes "${name}".`, options, "Move");
  const destination = picked === null ? undefined : reviews[Number(picked)];
  if (destination) await moveToReview(name, destination);
}

export async function checkRootFolders(moveToReview: MoveToReview): Promise<void> {
  // An open dialog or a running drag or operation would stack a second modal; the next focus or refresh checks again.
  if (checking || dialogOpen() || folderMoveActive() || state.busy || currentOperationStatus() !== null) return;
  checking = true;
  try {
    const { invalid, reviews } = await api.checkRoot();
    for (const name of invalid) {
      try {
        await notifyDesktop(`Invalid directory in your Storage found in ${name} (level 1)`);
      } catch (e) {
        setStatus("warning", `Notification: ${api.describeError(e)}`);
      }
      await askAbout(name, reviews, moveToReview);
    }
  } catch (e) {
    setStatus("warning", `Storage check: ${api.describeError(e)}`);
  } finally {
    checking = false;
  }
}
