// Entry point: assembles the shell and wires store, events and views.
import "./styles/theme.css";
import "./styles/themes.css";
import "./styles/invalid-dialog.css";
import "./styles/virtual.css";
import "./styles/board.css";
import { describeError } from "./ipc/api";
import { createColumnsView } from "./views/columns-view";
import { createGridView } from "./views/grid-view";
import { installContextMenu } from "./views/context-menu";
import { el } from "./ui/dom";
import { createNavigator } from "./shell/navigator";
import { createSearchBar } from "./shell/search-bar";
import { onFileDrag, onReviewIndex, onStorageChanged, onWindowFocus } from "./ipc/events";
import { init, refresh } from "./actions/ops";
import { autoRefresh } from "./actions/auto-refresh";
import { handleFileDrag } from "./actions/file-drop";
import { installShortcuts } from "./actions/shortcuts";
import { restoreFocus, trackFieldFocus } from "./actions/focus-return";
import { trackPasteTarget } from "./actions/paste-target";
import { installDragOut } from "./actions/drag-out";
import { installFolderMove } from "./actions/folder-move";
import { installMarquee } from "./actions/marquee";
import { createStatusLine, clearStatus, currentOperationStatus, setStatus } from "./ui/status";
import { rememberSelection, state, subscribe } from "./store/state";
import { createSidePanel } from "./shell/side-panel";
import { createToolbar, type ViewPicker } from "./shell/toolbar";
import { openSettings } from "./shell/settings-dialog";
import { createTreeView } from "./views/tree-view";
import { checkForUpdates } from "./actions/updates";
import { startBackups } from "./actions/backups";
import { startExtensions } from "./extensions/host";
import { installDebugLog, startDebugLog } from "./actions/debug-log";
import { applyAppearance } from "./ui/appearance";
import { createViewSwitcher } from "./views/view-switch";
import { createBoardView } from "./views/board-view";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const host = el("section", { class: "tree-host", "aria-label": "Storage" });
const views = createViewSwitcher(host, [createTreeView(), createColumnsView(), createGridView()]);
trackPasteTarget(host);
installDragOut(host);
installFolderMove(host);
installMarquee(host);
const viewPicker: ViewPicker = {
  modes: views.modes,
  active: () => views.current().id,
  onSelect: (id) => {
    views.switchTo(id);
    toolbar.render();
  },
};
const toolbar = createToolbar(viewPicker);
const navigator = createNavigator();
const search = createSearchBar();
navigator.element.prepend(...toolbar.navLead);
navigator.element.append(search.element, ...toolbar.trailing);
// The views already open on Enter, so the shortcut hands the key to whichever one is showing.
const openSelected = () => {
  views.current().focus();
  document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
};
const panel = createSidePanel(openSelected);
// The board stands beside the views, outside the host, so their drag, drop and marquee never reach its cards.
const board = createBoardView(host, panel.editDetails);
const main = el("main", { class: "app-main" }, host, board.element, ...panel.elements);
app.append(navigator.element, main, createStatusLine());
installContextMenu(host, () => views.current().focus(), viewPicker);
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
