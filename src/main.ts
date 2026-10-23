// Entry point: assembles the shell and wires store, events and views.
import "./styles/theme.css";
import { describeError } from "./ipc/api";
import { createColumnsView } from "./views/columns-view";
import { installContextMenu } from "./views/context-menu";
import { createDetailsPanel } from "./shell/details-panel";
import { el } from "./ui/dom";
import { createNavigator } from "./shell/navigator";
import { createSearchBar } from "./shell/search-bar";
import { onReviewIndex, onWindowFocus } from "./ipc/events";
import { init, refresh } from "./actions/ops";
import { installShortcuts } from "./actions/shortcuts";
import { readPref, savePref } from "./store/prefs";
import { createStatusLine, clearStatus, setStatus } from "./ui/status";
import { subscribe } from "./store/state";
import { createToolbar } from "./shell/toolbar";
import { createTreeView } from "./views/tree-view";
import { checkForUpdates } from "./actions/updates";
import { createViewSwitcher } from "./views/view-switch";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const host = el("section", { class: "tree-host", "aria-label": "Storage" });
const views = createViewSwitcher(host, [createTreeView(), createColumnsView()]);
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
});

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

let indexing = false;
onReviewIndex((e) => {
  if (e.phase === "started") {
    indexing = true;
    setStatus("info", "Rebuilding review index");
  } else if (e.failures.length > 0) {
    indexing = false;
    setStatus("warning", `Review index: ${plural(e.failures.length, "shortcut")} failed (${e.failures[0]})`);
  } else if (indexing) {
    indexing = false;
    clearStatus();
  }
}).catch((e) => setStatus("warning", `Review index updates unavailable: ${describeError(e)}`));
onWindowFocus(() => void refresh()).catch((e) => setStatus("warning", `Refresh on focus unavailable: ${describeError(e)}`));

void init().then(() => {
  views.current().focus();
  void checkForUpdates();
});
