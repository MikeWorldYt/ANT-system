// Connect AI dialog: links FolderManager's MCP server to the user's AI clients from the rail.
import { el } from "../../../dom";
import { cancelButton, textButton } from "../../../atoms/button";
import { aiClients, aiConfigSnippet, describeError } from "../../../../services/ipc/api";
import { openDialog } from "../dialog-shell";
import { clientRow } from "./client-row";

function otherClients(): HTMLElement {
  const feedback = el("p", { class: "ai-feedback", role: "status" });
  const copy = textButton("Copy config", "ai-link", async () => {
    feedback.classList.remove("is-error");
    try {
      await navigator.clipboard.writeText(await aiConfigSnippet());
      feedback.textContent = "Copied";
    } catch (e) {
      feedback.classList.add("is-error");
      feedback.textContent = describeError(e);
    }
  });
  copy.setAttribute("aria-describedby", "ai-other-caption");
  return el("section", { class: "ai-other", "aria-label": "Other MCP clients" }, el("p", { class: "ai-other-caption", id: "ai-other-caption" }, "Other MCP clients: add the server entry by hand."), copy, feedback);
}

export async function aiConnectDialog(): Promise<void> {
  const list = el("ul", { class: "ai-clients" });
  const loading = el("p", { role: "status" }, "Looking for AI clients...");
  const close = cancelButton("Close");
  const { dialog } = openDialog({ title: "Connect with your AI Client", body: [loading, list, otherClients()], buttons: [close], focus: () => close.focus() });
  dialog.classList.add("ai-connect");
  try {
    list.replaceChildren(...(await aiClients()).map(clientRow));
    loading.remove();
  } catch (e) {
    loading.textContent = describeError(e);
    loading.setAttribute("role", "alert");
  }
}
