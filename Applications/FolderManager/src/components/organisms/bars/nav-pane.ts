// The navigation pane left of the views: quick access entries above the storage tree, shown or hidden with Ctrl+Left.
import { el } from "../../dom";
import { createTreeView } from "../../windows/views/tree-view";
import { readPref, savePref } from "../../../services/store/prefs";

const PREF = "fm.navPane";

export interface NavPane {
  element: HTMLElement;
  render(): void;
  toggle(): void;
}

export function createNavPane(): NavPane {
  const element = el("aside", { class: "nav-pane", "aria-label": "Navigation" });
  const tree = createTreeView();
  tree.mount(element);
  element.hidden = readPref(PREF) === "hidden";

  // A hidden pane skips its rows, so it costs nothing while closed.
  const render = (): void => {
    if (!element.hidden) tree.render();
  };

  function toggle(): void {
    const hadFocus = element.contains(document.activeElement);
    element.hidden = !element.hidden;
    savePref(PREF, element.hidden ? "hidden" : "shown");
    render();
    if (!element.hidden) tree.focus();
    else if (hadFocus) document.querySelector<HTMLElement>(".tree-host.focused, .tree-host")?.focus();
  }

  return { element, render, toggle };
}
