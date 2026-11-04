// Entry point: mounts the main window, then wires the store, backend events, shortcuts and startup work.
import "./styles/theme.css";
import "./styles/themes.css";
import { createMainWindow } from "./components/windows/main-window";
import { openSettings } from "./components/windows/settings";
import { describeError } from "./services/ipc/api";
import { onFileDrag, onReviewIndex, onStorageChanged, onWindowFocus } from "./services/ipc/events";
import { rememberSelection, state, subscribe } from "./services/store/state";
import { clearStatus, currentOperationStatus, setStatus } from "./components/organisms/bars/status-line";
import { init, refresh } from "./hooks/ops";
import { autoRefresh } from "./hooks/auto-refresh";
import { handleFileDrag } from "./hooks/file-drop";
import { installShortcuts } from "./hooks/shortcuts";
import { restoreFocus, trackFieldFocus } from "./hooks/focus-return";
import { checkForUpdates } from "./hooks/updates";
import { startBackups } from "./hooks/backups";
import { installDebugLog, startDebugLog } from "./hooks/debug-log";
import { applyAppearance } from "./hooks/appearance";
import { startExtensions } from "./extensions/host";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const { views, toolbar, navigator, search, panel, board, openSelected } = createMainWindow(app);
installShortcuts({
  focusSearch: search.focus,
  editPath: navigator.editPath,
  openSettings: () => void openSettings(),
  goUp: navigator.goUp,
  openSelected,
  toggleSidePanel: panel.toggleCollapsed,
  editDetails: panel.editDetails,
});

subscribe(() => {
  toolbar.render();
  navigator.render();
  views.current().render();
  board.render();
  panel.render();
  rememberSelection();
  applyAppearance();
});

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

let indexing = false;
onReviewIndex((e) => {
  if (e.phase === "started") {
    indexing = true;
    const operation = currentOperationStatus();
    setStatus("info", operation ? `${operation} Updating review index…` : "Rebuilding review index");
  } else if (e.failures.length > 0) {
    indexing = false;
    setStatus("warning", `Review index: ${plural(e.failures.length, "shortcut")} failed (${e.failures[0]})`);
  } else if (indexing) {
    indexing = false;
    const operation = currentOperationStatus();
    if (operation) setStatus("info", operation);
    else clearStatus();
  }
}).catch((e) => setStatus("warning", `Review index updates unavailable: ${describeError(e)}`));
trackFieldFocus();
onWindowFocus(() => void autoRefresh(refresh).finally(() => restoreFocus(() => views.current().focus()))).catch((e) => setStatus("warning", `Refresh on focus unavailable: ${describeError(e)}`));

onStorageChanged(() => void autoRefresh(refresh)).catch((e) =>
  setStatus("warning", `Live storage updates unavailable: ${describeError(e)}`),
);

onFileDrag(handleFileDrag).catch((e) => setStatus("warning", `File drop unavailable: ${describeError(e)}`));

installDebugLog();

void init().then(() => {
  if (state.settings.debugLogging) void startDebugLog();
  startBackups();
  startExtensions();
  if (state.settings.startView !== "continue") {
    views.switchTo(state.settings.startView);
    toolbar.render();
  }
  views.current().focus();
  if (state.settings.autoUpdate) void checkForUpdates();
});
