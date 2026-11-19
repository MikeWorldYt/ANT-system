// One AI client in the Connect AI dialog: its state, the connect toggle and its own error.
import { el } from "../../../dom";
import { textButton } from "../../../atoms/button";
import { aiConnect, aiDisconnect, aiRestartClient, describeError } from "../../../../services/ipc/api";
import type { AiClient } from "../../../../services/ipc/types";

const statusText = (c: AiClient): string => (!c.installed ? "Not installed" : c.connected ? "Connected" : "Not connected");

export function clientRow(initial: AiClient): HTMLElement {
  let client = initial;
  const status = el("span", { class: "ai-client-status" });
  const note = el("p", { class: "ai-client-note" });
  const error = el("p", { class: "ai-client-error", role: "alert" });
  const toggle = textButton("", undefined, () => void run(() => (client.connected ? aiDisconnect : aiConnect)(client.id)));
  const restart = textButton(`Restart ${client.name}`, undefined, () => void run(() => aiRestartClient(client.id)));
  const restartBox = el("div", { class: "ai-client-restart" }, note, restart);

  function render(): void {
    status.textContent = statusText(client);
    toggle.textContent = client.connected ? "Disconnect" : "Connect";
    toggle.disabled = !client.installed;
    toggle.setAttribute("aria-label", `${toggle.textContent} ${client.name}`);
    restart.disabled = false;
    note.textContent = `${client.name} loads ANT only when it starts. Restarting closes all its windows, including chats still answering.`;
    restartBox.hidden = !client.restartNeeded;
  }

  async function run(action: () => Promise<AiClient>): Promise<void> {
    toggle.disabled = restart.disabled = true;
    toggle.setAttribute("aria-busy", "true");
    error.textContent = "";
    try {
      client = await action();
    } catch (e) {
      error.textContent = describeError(e);
    }
    toggle.removeAttribute("aria-busy");
    render();
  }

  render();
  return el("li", { class: "ai-client" }, el("div", { class: "ai-client-main" }, el("strong", {}, client.name), status, toggle), restartBox, error);
}
