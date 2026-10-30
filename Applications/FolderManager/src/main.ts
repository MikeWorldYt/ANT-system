// Entry point: assembles the shell and wires store, events and views.
import "./styles/theme.css";
import "./styles/themes.css";
import { describeError } from "./ipc/api";
import { createColumnsView } from "./views/columns-view";
import { createGridView } from "./views/grid-view";
import { installContextMenu } from "./views/context-menu";
import { createDetailsPanel } from "./shell/details-panel";
import { el } from "./ui/dom";
import { createNavigator } from "./shell/navigator";
import { createSearchBar } from "./shell/search-bar";
import { onReviewIndex, onStorageChanged, onWindowFocus } from "./ipc/events";
import { init, refresh } from "./actions/ops";
import { installShortcuts } from "./actions/shortcuts";
import { installDragOut } from "./actions/drag-out";
import { readPref, savePref } from "./store/prefs";
import { createStatusLine, clearStatus, currentOperationStatus, setStatus } from "./ui/status";
import { rememberSelection, state, subscribe } from "./store/state";
import { createToolbar } from "./shell/toolbar";
import { openSettings } from "./shell/settings-dialog";
import { createTreeView } from "./views/tree-view";
import { checkForUpdates } from "./actions/updates";
import { startBackups } from "./actions/backups";
import { startExtensions } from "./extensions/host";
import { installDebugLog, startDebugLog } from "./actions/debug-log";
import { applyAppearance } from "./ui/appearance";
import { createViewSwitcher } from "./views/view-switch";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const host = el("section", { class: "tree-host", "aria-label": "Storage" });
const views = createViewSwitcher(host, [createTreeView(), createColumnsView(), createGridView()]);
installDragOut(host);
const PANEL_PREF = "fm.details";
let detailsVisible = readPref(PANEL_PREF) !== "hidden";
const toolbar = createToolbar(
  {
    modes: views.modes,
    active: () => views.current().id,
    onSelect: (id) => {
      views.switchTo(id);
      toolbar.render();
    },
  },
  {
    visible: () => detailsVisible,
    toggle: () => {
      detailsVisible = !detailsVisible;
      savePref(PANEL_PREF, detailsVisible ? "shown" : "hidden");
      panel.element.hidden = !detailsVisible;
      toolbar.render();
    },
  },
);
const navigator = createNavigator();
const search = createSearchBar();
navigator.element.append(search.element);
const panel = createDetailsPanel();
panel.element.hidden = !detailsVisible;
const main = el("main", { class: "app-main" }, host, panel.element);
app.append(toolbar.element, navigator.element, main, createStatusLine());
installContextMenu(host, () => views.current().focus());
installShortcuts({
  focusSearch: search.focus,
  editPath: navigator.editPath,
  openSettings: () => void openSettings(),
  goUp: navigator.goUp,
  // The views already open on Enter, so the shortcut hands the key to whichever one is showing.
  openSelected: () => {
    views.current().focus();
    document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  },
});

subscribe(() => {
  toolbar.render();
  navigator.render();
  views.current().render();
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
onWindowFocus(() => void refresh()).catch((e) => setStatus("warning", `Refresh on focus unavailable: ${describeError(e)}`));

// Watcher batches can arrive while a refresh runs; one pending rerun covers them all.
let refreshing = false;
let pending = false;
async function refreshForWatcher(): Promise<void> {
  if (refreshing) {
    pending = true;
    return;
  }
  refreshing = true;
  try {
    do {
      pending = false;
      await refresh();
    } while (pending);
  } finally {
    refreshing = false;
  }
}
onStorageChanged(() => void refreshForWatcher()).catch((e) =>
  setStatus("warning", `Live storage updates unavailable: ${describeError(e)}`),
);

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
