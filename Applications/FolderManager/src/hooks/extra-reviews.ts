// Offers to merge the leftover review folders of older builds; refresh and intake leave them alone, so only this answer moves them.
import * as api from "../services/ipc/api";
import { extraReviewsDialog } from "../components/organisms/dialogs/action-dialog/extra-reviews-dialog";
import { reportError } from "../components/organisms/bars/status-line";

type Merge = (relParts: string[]) => Promise<boolean>;

let running = false;

export async function checkExtraReviews(merge: Merge): Promise<void> {
  if (running) return;
  running = true;
  try {
    const folders = await api.checkExtraReviews();
    if (folders.length === 0) return;
    const showFolder = (relParts: string[]) => void api.revealNode(relParts).catch((e: unknown) => reportError("warning", e));
    if (!(await extraReviewsDialog(folders, showFolder))) return;
    for (const folder of folders) if (!(await merge(folder.relParts))) return;
  } catch (e) {
    reportError("warning", e, "Extra review folders");
  } finally {
    running = false;
  }
}
