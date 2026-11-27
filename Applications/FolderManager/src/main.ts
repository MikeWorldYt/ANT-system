// Entry point: mounts the main window, then wires the store, backend events, shortcuts and startup work.
import "./styles/index.css";
import { createMainWindow } from "./components/windows/main-window";
import { openSettings } from "./components/windows/settings";
import { onDownloadsChanged, onDownloadsDictionary, onFileDrag, onReviewIndex, onStorageChanged, onUiRequest, onWindowFocus } from "./services/ipc/events";
import { rememberSelection, state, subscribe } from "./services/store/state";
import { clearStatus, currentOperationStatus, setStatus, reportError } from "./components/organisms/bars/status-line";
import { init, refresh, relistLevels } from "./hooks/ops";
import { relistSystem } from "./hooks/relist";
import { installTooltips } from "./components/organisms/tooltip/tooltip";
import { autoRefresh } from "./hooks/auto-refresh";
import { startIdleRefresh } from "./hooks/idle-refresh";
import { handleFileDrag } from "./hooks/file-drop";
import { answerAgent } from "./hooks/agent-ui";
import { installShortcuts } from "./hooks/shortcuts";
import { isRenderingOther, renderOther } from "./hooks/split";
import { closeTab, nextTab, openTab, syncTabs } from "./hooks/tabs";
import { restoreFocus, trackFieldFocus } from "./hooks/focus-return";
import { checkForUpdates } from "./hooks/updates";
import { startBackups } from "./hooks/backups";
import { installDebugLog, startDebugLog } from "./hooks/debug-log";
import { applyAppearance } from "./hooks/appearance";
import { BATCH_VIEWS, batch as batchMode, replanAfterDictionary, restoreBatch } from "./hooks/automations/batch";
import { bindReplay } from "./hooks/automations/batch-replay";
import { readPref, savePref } from "./services/store/prefs";
import { startExtensions } from "./extensions/host";
import { plural } from "./services/plural";
import { onTourStart } from "./components/organisms/tour/onboarding-tour";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const { views, toolbar, navigator, tabBar, search, panel, board, batch, nav, openSelected } = createMainWindow(app);
bindReplay({ viewChanged: () => toolbar.render() });
installShortcuts({
  focusSearch: () => search.focus("folders"),
  searchFiles: () => search.focus("files"),
  editPath: navigator.editPath,
  openSettings: () => void openSettings(),
  goUp: navigator.goUp,
  openSelected,
  toggleSidePanel: panel.toggleCollapsed,
  toggleNavPane: nav.toggle,
  editDetails: panel.editDetails,
  newTab: () => openTab(),
  closeTab: () => closeTab(),
  nextTab,
});

// Tree and Graph cannot draw a batch: Tree gives way to List, Graph to the default view when the batch can use it, and either comes back after.
let lastBatch: typeof batchMode = null;
// The view the batch was drawn in, so a restored one comes back as a list or a grid.
const BATCH_VIEW = "fm.batch.view";
function followBatch(): void {
  if (batchMode) savePref(BATCH_VIEW, views.current().id);
  if (batchMode && batchMode.viewBefore === undefined) {
    const before = views.current().id;
    batchMode.viewBefore = before;
    if (!BATCH_VIEWS.includes(before)) {
      const preferred = state.settings.startView;
      views.switchTo(before === "graph" && BATCH_VIEWS.includes(preferred) ? preferred : "list");
    }
  }
  // Only a closed batch gives a view back, and only one it had to leave; a batch parked in another tab keeps it for its return.
  const before = lastBatch?.viewBefore;
  if (!batchMode && lastBatch && !lastBatch.parked && before && !BATCH_VIEWS.includes(before) && views.current().id !== before) views.switchTo(before);
  lastBatch = batchMode;
}

subscribe(() => {
  if (isRenderingOther()) return;
  followBatch();
  // The view settles which folder it shows first, so the breadcrumb and the tab read that folder.
  views.current().render();
  toolbar.render();
  navigator.render();
  syncTabs();
  tabBar.render();
  nav.render();
  renderOther();
  board.render();
  batch.render();
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
trackFieldFocus(() => views.current().focus());
// The webview's own focus events need no backend round trip, so arrows work at once; the backend's one repeats it after the relist.
window.addEventListener("focus", restoreFocus);
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") restoreFocus();
});
const FOCUS_GAP_MS = 5_000;
let lastFocusAt = 0;
onWindowFocus(() => {
  const quick = Date.now() - lastFocusAt < FOCUS_GAP_MS;
  lastFocusAt = Date.now();
  void (quick ? Promise.resolve() : relistLevels(null)).finally(restoreFocus);
}).catch((e) => reportError("warning", e, "Refresh on focus unavailable"));

onDownloadsChanged((root) => void relistSystem(root)).catch((e) => reportError("warning", e, "Live Downloads updates unavailable"));

onDownloadsDictionary(replanAfterDictionary).catch((e) => reportError("warning", e, "Downloads dictionary updates unavailable"));

onStorageChanged((e) => {
  if (e.dictionary) replanAfterDictionary();
  void (e.rescan ? autoRefresh(() => refresh(false)) : relistLevels(e.folders));
}).catch((e) =>
  reportError("warning", e, "Live storage updates unavailable"),
);

onFileDrag(handleFileDrag).catch((e) => reportError("warning", e, "File drop unavailable"));

onUiRequest((r) => void answerAgent(views, r)).catch((e) => reportError("warning", e, "Agent window tools unavailable"));

installDebugLog();
installTooltips();

void init().then((ready) => {
  if (state.settings.debugLogging) void startDebugLog();
  startBackups();
  startIdleRefresh(() => refresh(false));
  startExtensions();
  // Continuing where the user left off keeps the last view; the default view is for a fresh start and new windows.
  if (state.settings.startView !== "continue" && state.settings.onLaunch !== "continue") {
    views.switchTo(state.settings.startView);
    toolbar.render();
  }
  views.current().focus();
  onTourStart({
    prepare: () => {
      views.switchTo("columns");
      toolbar.render();
    },
    currentView: () => views.current().id,
    showView: (id) => {
      views.switchTo(id);
      toolbar.render();
    },
  });
  // The tutorial no longer starts by itself; until it is taken, About and its Show tutorial button pulse instead.
  if (ready)
    void restoreBatch().then((open) => {
      const view = readPref(BATCH_VIEW);
      if (open && view && view !== "list" && BATCH_VIEWS.includes(view)) {
        views.switchTo(view);
        toolbar.render();
      }
    });
  // Every launch looks for a newer version; the setting only decides whether it installs unasked.
  void checkForUpdates();
});
