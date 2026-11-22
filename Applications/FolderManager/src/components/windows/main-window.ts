// Main window layout: the navigation row with toolbar and search, the views beside the board and side panel, and the status line.
import { el } from "../dom";
import { createNavigator } from "../organisms/bars/navigator";
import { createSearchBar } from "../organisms/bars/search-bar";
import { createSidePanel } from "../organisms/bars/side-panel";
import { createStatusLine } from "../organisms/bars/status-line";
import { createTabBar } from "../organisms/bars/tab-bar";
import { createToolbar, type ViewPicker } from "../organisms/bars/toolbar";
import { installContextMenu } from "../organisms/menus/context-menu";
import { installDragOut } from "../../hooks/drag-out";
import { installFolderMove } from "../../hooks/folder-move";
import { installMarquee } from "../../hooks/marquee";
import { trackPasteTarget } from "../../hooks/paste-target";
import { bindPanes, focusedViews } from "../../hooks/split";
import { bindTabs } from "../../hooks/tabs";
import { createViewSwitcher, rememberView } from "../../hooks/view-switch";
import { openFolder } from "../../hooks/path-actions";
import { selectedNode } from "../../services/store/state";
import { openSettings } from "./settings";
import { createBatchView } from "./views/batch-view";
import { createBoardView } from "./views/board-view";
import { createColumnsView } from "./views/columns-view";
import { createCardsView, createDetailsView, createGridView, createListView } from "./views/grid-view";
import { createGraphView } from "./views/graph-view";
import { createNavPane } from "../organisms/bars/nav-pane";

export function createMainWindow(app: HTMLElement) {
  const host = el("section", { class: "tree-host", "aria-label": "Storage" });
  const rightHost = el("section", { class: "tree-host", "aria-label": "Split storage" });
  const panes = el("div", { class: "panes" }, host, rightHost);
  const leftViews = createViewSwitcher(host, [createCardsView(), createColumnsView(), createGridView(), createListView(), createDetailsView(), createGraphView()]);
  const rightViews = createViewSwitcher(rightHost, [createCardsView(), createColumnsView(), createGridView(), createListView(), createDetailsView(), createGraphView()]);
  const views = focusedViews(leftViews.modes);
  const nav = createNavPane();
  for (const pane of [host, rightHost, nav.element]) {
    trackPasteTarget(pane);
    installDragOut(pane);
    installFolderMove(pane);
    installMarquee(pane);
  }
  const viewPicker: ViewPicker = {
    modes: views.modes,
    active: () => views.current().id,
    folder: () => views.current().openFolder?.(),
    onSelect: (id) => {
      views.switchTo(id);
      rememberView(id);
      toolbar.render();
      // The batch draws itself as a list or a grid, and nothing else asks it to redraw on a view change.
      batch.render();
    },
  };
  const toolbar = createToolbar(viewPicker);
  const navigator = createNavigator();
  bindPanes({ host, views: leftViews }, { host: rightHost, views: rightViews }, navigator);
  bindTabs(navigator, views);
  const tabBar = createTabBar();
  const search = createSearchBar();
  navigator.element.prepend(...toolbar.navLead);
  navigator.element.append(search.element, ...toolbar.trailing);
  const openSelected = () => {
    const n = selectedNode();
    if (n?.actions.open) void openFolder(n);
  };
  const panel = createSidePanel({ openSelected, openSettings: (pageId) => void openSettings(pageId) });
  // The board stands beside the views, outside the host, so their drag, drop and marquee never reach its cards.
  const board = createBoardView(panes, panel.editDetails);
  const batchViews = [createBatchView(panes, false), createBatchView(panes, true)];
  const batch = { render: () => batchViews.forEach((b) => b.render()) };
  const main = el("main", { class: "app-main" }, nav.element, panes, board.element, ...batchViews.map((b) => b.element), ...panel.elements);
  app.append(tabBar.element, navigator.element, main, createStatusLine());
  installContextMenu(panes, () => views.current().focus(), viewPicker);
  return { views, toolbar, navigator, tabBar, search, panel, board, batch, nav, openSelected };
}
