// Entry point: assembles the shell and wires store, events and views.
import "./styles/theme.css";
import { createActionsPanel } from "./actions-panel";
import { el } from "./dom";
import { onReviewIndex, onWindowFocus } from "./events";
import { init, refresh } from "./ops";
import { createStatusLine, clearStatus, setStatus } from "./status";
import { subscribe } from "./state";
import { createToolbar } from "./toolbar";
import { createListView } from "./tree-view";
import type { ViewMode } from "./view-mode";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Missing #app mount point");

const toolbar = createToolbar();
const panel = createActionsPanel();
const view: ViewMode = createListView();
const host = el("section", { class: "tree-host", "aria-label": "Folder tree" });
const main = el("main", { class: "app-main" }, host, panel.element);
app.append(toolbar.element, main, createStatusLine());
view.mount(host);

subscribe(() => {
  toolbar.render();
  view.render();
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

void init().then(() => view.focus());
