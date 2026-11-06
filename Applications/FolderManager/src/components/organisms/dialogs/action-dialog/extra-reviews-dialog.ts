// Merging moves the user's files between folders, so older builds' leftover review folders wait for this answer.
import { el } from "../../../dom";
import { dialogText } from "../dialog-shell";
import { actionDialog, colHeader, dialogTable } from "./action-dialog";
import { pathButton, pathTrail } from "./invalid-dialog";
import type { ExtraReview } from "../../../../services/ipc/types";

const INTRO = ["An older version left these extra review folders. Merging moves their contents into the review folder of the same node and removes the emptied folder.", "A file whose name is already taken gets a number, so nothing is overwritten."];

// Resolves to true on Merge, false when the folders stay as they are.
export function extraReviewsDialog(folders: ExtraReview[], showFolder: (relParts: string[]) => void): Promise<boolean> {
  const rows = folders.map((folder) => el("tr", {}, el("th", { scope: "row" }, pathButton(folder.relParts, el("strong", {}, folder.name), showFolder)), el("td", {}, pathTrail(folder.relParts), el("strong", {}, folder.keep))));
  const { view } = dialogTable("Extra review folders", [colHeader(["Folder"]), colHeader(["Merges into"])], rows);
  const { closed } = actionDialog("Extra review folders", INTRO.map(dialogText), view, undefined, { later: "Keep them", apply: "Merge" });
  return closed.then((value) => value === "ok");
}
