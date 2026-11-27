// One AI client in the Connect AI dialog: a single sign-in style button to connect it, its state and its own error.
import { el } from "../../../dom";
import { textButton } from "../../../atoms/button";
import { guiIcon, type GuiIcon } from "../../../atoms/gui-icons";
import { aiConnect, aiDisconnect, aiRestartClient, describeError } from "../../../../services/ipc/api";
import type { AiClient, AiClientId } from "../../../../services/ipc/types";

const ICONS = { claudeDesktop: "claude", claudeCode: "claudeCode", codex: "codex", opencode: "opencode" } as const satisfies Record<AiClientId, GuiIcon>;

export function clientRow(initial: AiClient): HTMLElement {
  let client = initial;
  const label = el("span", { class: "ai-connect-label" });
  const icon = el("span", { class: "ai-connect-icon", "aria-hidden": "true" });
  const note = el("p", { class: "ai-client-note" });
  const error = el("p", { class: "ai-client-error", role: "alert" });
  const connect = el("button", { type: "button", class: "ai-connect-btn", "data-client": client.id }, icon, label);
  // A connected client's button turns into its Disconnect on hover or focus, so one button does both.
  connect.addEventListener("click", () => void run(() => (client.connected ? aiDisconnect : aiConnect)(client.id)));
  const showLabel = (): void => {
    const off = client.connected && (connect.matches(":hover") || connect.matches(":focus-visible"));
    if (client.installed && client.connected && !off) label.replaceChildren(el("span", { class: "ai-connected-word" }, "Connected"), ` with ${client.name}`);
    else if (!client.installed) label.replaceChildren(client.name, el("span", { class: "ai-hint" }, "Not installed"));
    else label.textContent = off ? `Disconnect ${client.name}` : `Connect with ${client.name}`;
  };
  for (const type of ["pointerenter", "pointerleave", "focus", "blur"]) connect.addEventListener(type, showLabel);
  const restart = textButton(`Restart ${client.name}`, undefined, () => void run(() => aiRestartClient(client.id)));
  const restartBox = el("div", { class: "ai-client-restart" }, note, restart);

  function render(): void {
    showLabel();
    icon.replaceChildren(guiIcon(ICONS[client.id]));
    connect.classList.toggle("is-connected", client.connected);
    connect.disabled = !client.installed;
    connect.classList.toggle("is-missing", !client.installed);
    connect.setAttribute("aria-label", !client.installed ? `${client.name} is not installed` : client.connected ? `Connected with ${client.name}, press to disconnect` : `Connect with ${client.name}`);
    restart.disabled = false;
    note.textContent = `${client.name} loads ANT only when it starts. Restarting closes all its windows, including chats still answering.`;
    restartBox.hidden = !client.restartNeeded;
  }

  async function run(action: () => Promise<AiClient>): Promise<void> {
    connect.disabled = restart.disabled = true;
    connect.setAttribute("aria-busy", "true");
    error.textContent = "";
    try {
      client = await action();
    } catch (e) {
      error.textContent = describeError(e);
    }
    connect.removeAttribute("aria-busy");
    render();
  }

  render();
  return el("li", { class: "ai-client" }, el("div", { class: "ai-client-main" }, connect), restartBox, error);
}
