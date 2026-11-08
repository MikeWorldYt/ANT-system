// Runs the interface actions an MCP agent sends through the UI bridge; nothing here moves unless an agent calls a ui_* tool.
import { answerUiRequest, describeError } from "../services/ipc/api";
import type { UiCall, UiRequest } from "../services/ipc/types";
import { notify, state } from "../services/store/state";
import { reportError } from "../components/organisms/bars/status-line";
import { openInSplit } from "./path-actions";
import { currentSplit, openSplit } from "./split";
import { openTab, tabStates } from "./tabs";
import { goToReal } from "./virtual-nav";
import type { ViewSwitcher } from "./view-switch";

const parts = (key: string | null) => key?.split("/") ?? null;

function uiState(views: ViewSwitcher) {
  const tabs = tabStates();
  const split = currentSplit();
  return {
    root: state.tree?.root ?? null,
    tabs,
    activeTab: tabs.findIndex((t) => t.active),
    split: split && { focused: split.right ? "right" : "left", otherRelParts: parts(split.other.selectedKey), otherView: split.view },
    view: views.current().id,
    virtual: state.virtual,
    selection: parts(state.selectedKey),
    selected: state.selectedKeys.map((k) => k.split("/")),
  };
}

async function run(views: ViewSwitcher, call: UiCall): Promise<void> {
  if (call.action !== "getState" && state.busy) throw new Error("Folder Manager is busy with an operation; call the tool again in a moment");
  switch (call.action) {
    case "getState":
      return;
    case "reveal":
      return goToReal(call.relParts);
    case "openTab": {
      const before = tabStates().length;
      await openTab(call.relParts ?? undefined);
      if (tabStates().length === before) throw new Error("A tab change was in progress; call the tool again");
      return;
    }
    case "split":
      if (call.relParts) return openInSplit({ relParts: call.relParts });
      return openSplit();
    case "setView":
      if (!views.modes.some((m) => m.id === call.view)) throw new Error(`Unknown view: ${call.view}`);
      views.switchTo(call.view);
      return notify();
  }
}

export async function answerAgent(views: ViewSwitcher, { id, call }: UiRequest): Promise<void> {
  let value: ReturnType<typeof uiState> | null = null;
  let error: string | null = null;
  try {
    await run(views, call);
    value = uiState(views);
  } catch (e) {
    error = e instanceof Error ? e.message : describeError(e);
  }
  await answerUiRequest(id, value, error).catch((e: unknown) => reportError("warning", e, "Could not answer the agent"));
}
