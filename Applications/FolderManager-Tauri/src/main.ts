// Entry point: assembles the shell and wires store, events and views.
import "./styles/theme.css";
import { createActionsPanel } from "./actions-panel";
import { createColumnsView } from "./columns-view";
import { installContextMenu } from "./context-menu";
import { el } from "./dom";
import { createNavigator } from "./navigator";
import { onReviewIndex, onWindowFocus } from "./events";
import { init, refresh } from "./ops";
import { createStatusLine, clearStatus, setStatus } from "./status";
import { subscribe } from "./state";
import { createToolbar } from "./toolbar";
import { createListView } from "./tree-view";
import { createViewSwitcher } from "./view-switch";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const host = el("section", { class: "tree-host", "aria-label": "Folder tree" });
const views = createViewSwitcher(host, [createListView(), createColumnsView()]);
const toolbar = createToolbar({
  modes: views.modes,
  active: () => views.current().id,
  onSelect: (id) => {
    views.switchTo(id);
    toolbar.render();
  },
});
const navigator = createNavigator();
const panel = createActionsPanel();
const main = el("main", { class: "app-main" }, host, panel.element);
app.append(toolbar.element, navigator.element, main, createStatusLine());
installContextMenu(host, () => views.current().focus());

subscribe(() => {
  toolbar.render();
  navigator.render();
  views.current().render();
  panel.render();
});

let indexing = false;
void onReviewIndex((e) => {
  if (e.phase === "started") {
    indexing = true;
    setStatus("info", "Rebuilding review index");
  } else if (e.failures.length > 0) {
    indexing = false;
    setStatus("warning", `Review index: ${e.failures.length} shortcuts failed (${e.failures[0]})`);
  } else if (indexing) {
    indexing = false;
    clearStatus();
  }
});
void onWindowFocus(() => void refresh());

void init().then(() => views.current().focus());
