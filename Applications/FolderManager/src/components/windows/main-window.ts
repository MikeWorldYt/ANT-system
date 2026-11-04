// Main window layout: the navigation row with toolbar and search, the views beside the board and side panel, and the status line.
import { el } from "../dom";
import { createNavigator } from "../organisms/bars/navigator";
import { createSearchBar } from "../organisms/bars/search-bar";
import { createSidePanel } from "../organisms/bars/side-panel";
import { createStatusLine } from "../organisms/bars/status-line";
import { createToolbar, type ViewPicker } from "../organisms/bars/toolbar";
import { installContextMenu } from "../organisms/menus/context-menu";
import { installDragOut } from "../../hooks/drag-out";
import { installFolderMove } from "../../hooks/folder-move";
import { installMarquee } from "../../hooks/marquee";
import { trackPasteTarget } from "../../hooks/paste-target";
import { createViewSwitcher } from "../../hooks/view-switch";
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
  return { views, toolbar, navigator, search, panel, board, openSelected };
}
