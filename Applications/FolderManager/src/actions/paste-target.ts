// Ctrl+V target: the folder of the last click in the views, else the folder the path bar shows.
import { pasteFiles } from "./ops";
import { state } from "../store/state";

// Space beside the columns belongs to the deepest one, as in its background menu.
function clickedKey(t: Element): string | undefined {
  const hit = t.closest("[data-key]") ?? (t.matches(".columns") ? t.lastElementChild : null);
  return hit instanceof HTMLElement ? hit.dataset.key : undefined;
}

export function trackPasteTarget(host: HTMLElement): void {
  host.addEventListener("click", (ev) => {
    if (!(ev.target instanceof Element)) return;
    const key = clickedKey(ev.target);
    if (key !== undefined) state.pasteKey = key;
  });
}

// An empty key is the storage root; it goes to the backend as an empty target for its refusal message.
export function pasteHere(): void {
  const key = state.pasteKey ?? state.selectedKey ?? "";
  void pasteFiles(key === "" ? [] : key.split("/"));
}
