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
import { bindTabs } from "../../hooks/tabs";
import { createViewSwitcher } from "../../hooks/view-switch";
import { openFolder } from "../../hooks/path-actions";
import { selectedNode } from "../../services/store/state";
import { openSettings } from "./settings";
import { createBoardView } from "./views/board-view";
import { createColumnsView } from "./views/columns-view";
import { createGridView } from "./views/grid-view";
import { createTreeView } from "./views/tree-view";

export function createMainWindow(app: HTMLElement) {
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
  bindTabs(navigator);
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
  const board = createBoardView(host, panel.editDetails);
  const main = el("main", { class: "app-main" }, host, board.element, ...panel.elements);
  app.append(tabBar.element, navigator.element, main, createStatusLine());
  installContextMenu(host, () => views.current().focus(), viewPicker);
  return { views, toolbar, navigator, tabBar, search, panel, board, openSelected };
}
