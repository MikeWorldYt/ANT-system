// Entry point: assembles the shell and wires store, events and views.
import "./styles/theme.css";
import { describeError } from "./api";
import { createColumnsView } from "./columns-view";
import { installContextMenu } from "./context-menu";
import { createDetailsPanel } from "./details-panel";
import { el } from "./dom";
import { createNavigator } from "./navigator";
import { createSearchBar } from "./search-bar";
import { onReviewIndex, onWindowFocus } from "./events";
import { init, refresh } from "./ops";
import { readPref, savePref } from "./prefs";
import { createStatusLine, clearStatus, setStatus } from "./status";
import { subscribe } from "./state";
import { createToolbar } from "./toolbar";
import { createTreeView } from "./tree-view";
import { checkForUpdates } from "./updates";
import { createViewSwitcher } from "./view-switch";

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
navigator.element.append(createSearchBar());
const panel = createDetailsPanel();
panel.element.hidden = !detailsVisible;
const main = el("main", { class: "app-main" }, host, panel.element);
app.append(toolbar.element, navigator.element, main, createStatusLine());
installContextMenu(host, () => views.current().focus());

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
