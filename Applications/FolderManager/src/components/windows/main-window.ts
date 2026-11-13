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
import { createViewSwitcher } from "../../hooks/view-switch";
import { openFolder } from "../../hooks/path-actions";
import { selectedNode } from "../../services/store/state";
import { openSettings } from "./settings";
import { createBatchView } from "./views/batch-view";
import { createBoardView } from "./views/board-view";
import { createColumnsView } from "./views/columns-view";
import { createCardsView, createDetailsView, createGridView, createListView } from "./views/grid-view";
import { createTreeView } from "./views/tree-view";

export function createMainWindow(app: HTMLElement) {
  const host = el("section", { class: "tree-host", "aria-label": "Storage" });
  const rightHost = el("section", { class: "tree-host", "aria-label": "Split storage" });
  const panes = el("div", { class: "panes" }, host, rightHost);
  const leftViews = createViewSwitcher(host, [createCardsView(), createTreeView(), createColumnsView(), createGridView(), createListView(), createDetailsView()]);
  const rightViews = createViewSwitcher(rightHost, [createCardsView(), createTreeView(), createColumnsView(), createGridView(), createListView(), createDetailsView()]);
  const views = focusedViews(leftViews.modes);
  for (const pane of [host, rightHost]) {
    trackPasteTarget(pane);
    installDragOut(pane);
    installFolderMove(pane);
    installMarquee(pane);
  }
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
  const batch = createBatchView(panes);
  const main = el("main", { class: "app-main" }, panes, board.element, batch.element, ...panel.elements);
  app.append(tabBar.element, navigator.element, main, createStatusLine());
  installContextMenu(panes, () => views.current().focus(), viewPicker);
  return { views, toolbar, navigator, tabBar, search, panel, board, batch, openSelected };
}
