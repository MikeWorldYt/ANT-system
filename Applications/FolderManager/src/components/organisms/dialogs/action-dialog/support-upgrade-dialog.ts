// Resource folders may be linked from other programs, so their rename is announced and timed by the user.
import { el } from "../../../dom";
import { dialogText } from "../dialog-shell";
import { actionDialog, colHeader, dialogTable } from "./action-dialog";
import { pathButton, pathTrail } from "./invalid-dialog";
import type { SupportFolder } from "../../../../services/ipc/types";

const INTRO = ["These resource folders will be renamed with the new identifier. `90.*` → `RES.*`", "Programs that use them may lose their paths, so please link them again afterwards."];

// Resolves to true on Apply, false when it is left for later.
export function supportUpgradeDialog(folders: SupportFolder[], showFolder: (relParts: string[]) => void): Promise<boolean> {
  const rows = folders.map((folder) => el("tr", {}, el("th", { scope: "row" }, pathButton(folder.relParts, el("strong", {}, folder.name), showFolder)), el("td", {}, pathTrail(folder.relParts), el("strong", {}, folder.proposed))));
  const { view } = dialogTable("Resource folders", [colHeader(["Current Name"]), colHeader(["New Name"])], rows);
  const { closed } = actionDialog("Support Nodes identifier update", INTRO.map(dialogText), view, undefined, { later: "Do it later", apply: "Apply" });
  return closed.then((value) => value === "ok");
}
