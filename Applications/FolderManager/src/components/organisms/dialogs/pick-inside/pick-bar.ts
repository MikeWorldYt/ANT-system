// The picker's navigation bar: back, forward and up, an address bar that never reaches past the worknode, refresh and search.
import { el } from "../../../dom";
import { iconButton } from "../../../atoms/icon-button";
import { guiIcon } from "../../../atoms/gui-icons";
import { textInput } from "../../../atoms/text-input";

export interface BarEvents {
  back(): void;
  forward(): void;
  up(): void;
  // A typed path, relative to the worknode; resolves false when it names no folder there.
  go(path: string[]): Promise<boolean>;
  search(query: string): void;
}

export interface PickBar {
  element: HTMLElement;
  show(root: string, path: string[], canBack: boolean, canForward: boolean): void;
  clearSearch(): void;
}

export function pickBar(events: BarEvents): PickBar {
  const back = iconButton("Back", "back", events.back);
  const forward = iconButton("Forward", "forward", events.forward);
  const up = iconButton("Up one folder", "up", events.up);
  const crumbs = el("div", { class: "pick-crumbs" });
  const typed = textInput({ class: "pick-typed", "aria-label": "Folder path inside the worknode" });
  typed.hidden = true;
  const address = el("div", { class: "pick-address" }, crumbs, typed);
  const search = textInput({ class: "pick-search-input", type: "search", "aria-label": "Search in this folder" });
  const searchBox = el("label", { class: "pick-search" }, guiIcon("search"), search);
  let current: string[] = [];

  // A click on the bar's empty part turns it into a field with the path, as Explorer's address bar does.
  const editPath = (): void => {
    typed.value = current.join("/");
    crumbs.hidden = true;
    typed.hidden = false;
    typed.focus();
    typed.select();
  };
  const closeEdit = (): void => {
    typed.hidden = true;
    crumbs.hidden = false;
  };
  address.addEventListener("click", (ev) => {
    if (ev.target === address || ev.target === crumbs) editPath();
  });
  typed.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape") {
      ev.preventDefault();
      ev.stopPropagation();
      closeEdit();
    } else if (ev.key === "Enter") {
      ev.preventDefault();
      const parts = typed.value.split(/[\\/]+/).map((p) => p.trim()).filter(Boolean);
      void events.go(parts).then((ok) => {
        if (ok) closeEdit();
        else typed.setCustomValidity("No such folder in this worknode");
        typed.reportValidity();
      });
    }
  });
  typed.addEventListener("input", () => typed.setCustomValidity(""));
  typed.addEventListener("blur", closeEdit);
  search.addEventListener("input", () => events.search(search.value.trim()));

  function show(root: string, path: string[], canBack: boolean, canForward: boolean): void {
    current = path;
    back.disabled = !canBack;
    forward.disabled = !canForward;
    up.disabled = path.length === 0;
    const parts = [root, ...path];
    crumbs.replaceChildren(
      ...parts.flatMap((part, i) => {
        const crumb = el("button", { type: "button", class: "pick-crumb" }, part);
        crumb.addEventListener("click", () => void events.go(path.slice(0, i)));
        return i === 0 ? [crumb] : [el("span", { class: "pick-sep", "aria-hidden": "true" }, "›"), crumb];
      }),
    );
    search.placeholder = `Search ${parts[parts.length - 1] ?? root}`;
  }

  return {
    element: el("div", { class: "pick-bar" }, back, forward, up, address, searchBox),
    show,
    clearSearch: () => {
      search.value = "";
    },
  };
}
