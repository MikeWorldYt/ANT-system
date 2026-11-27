// Creates a worknode template in ANT.TEMPLATES and stands on it once the folder is listed.
import { createTemplate } from "../services/ipc/api";
import { reportError } from "../components/organisms/bars/status-line";
import { templateDialog } from "../components/organisms/dialogs/action-dialog/template-dialog";
import { select } from "../services/store/state";
import { relist } from "./relist";

export async function newTemplate(home: string[]): Promise<void> {
  const answer = await templateDialog();
  if (!answer) return;
  try {
    const made = await createTemplate(answer.identifier, answer.name, answer.description);
    await relist([home], true);
    select([...home, made.at(-1) ?? ""]);
  } catch (e) {
    reportError("error", e, "New template");
  }
}
