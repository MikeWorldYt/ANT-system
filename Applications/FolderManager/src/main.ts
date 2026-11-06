// Entry point: mounts the main window, then wires the store, backend events, shortcuts and startup work.
import "./styles/index.css";
import { createMainWindow } from "./components/windows/main-window";
import { openSettings } from "./components/windows/settings";
import { onFileDrag, onReviewIndex, onStorageChanged, onWindowFocus } from "./services/ipc/events";
import { rememberSelection, state, subscribe } from "./services/store/state";
import { clearStatus, currentOperationStatus, setStatus, reportError } from "./components/organisms/bars/status-line";
import { init, refresh, relistLevels } from "./hooks/ops";
import { autoRefresh } from "./hooks/auto-refresh";
import { startIdleRefresh } from "./hooks/idle-refresh";
import { handleFileDrag } from "./hooks/file-drop";
import { installShortcuts } from "./hooks/shortcuts";
import { restoreFocus, trackFieldFocus } from "./hooks/focus-return";
import { checkForUpdates } from "./hooks/updates";
import { startBackups } from "./hooks/backups";
import { installDebugLog, startDebugLog } from "./hooks/debug-log";
import { applyAppearance } from "./hooks/appearance";
import { startExtensions } from "./extensions/host";
import { plural } from "./services/plural";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const { views, toolbar, navigator, search, panel, board, openSelected } = createMainWindow(app);
installShortcuts({
  focusSearch: () => search.focus("folders"),
  searchFiles: () => search.focus("files"),
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
}).catch((e) => reportError("warning", e, "Review index updates unavailable"));
trackFieldFocus();
const FOCUS_GAP_MS = 5_000;
let lastFocusAt = 0;
onWindowFocus(() => {
  const quick = Date.now() - lastFocusAt < FOCUS_GAP_MS;
  lastFocusAt = Date.now();
  void (quick ? Promise.resolve() : relistLevels(null)).finally(() => restoreFocus(() => views.current().focus()));
}).catch((e) => reportError("warning", e, "Refresh on focus unavailable"));

onStorageChanged((e) => void (e.rescan ? autoRefresh(() => refresh(false)) : relistLevels(e.folders))).catch((e) =>
  reportError("warning", e, "Live storage updates unavailable"),
);

onFileDrag(handleFileDrag).catch((e) => reportError("warning", e, "File drop unavailable"));

installDebugLog();

void init().then(() => {
  if (state.settings.debugLogging) void startDebugLog();
  startBackups();
  startIdleRefresh(() => refresh(false));
  startExtensions();
  if (state.settings.startView !== "continue") {
    views.switchTo(state.settings.startView);
    toolbar.render();
  }
  views.current().focus();
  if (state.settings.autoUpdate) void checkForUpdates();
});
