// A batch tile renames in place, like a file in any explorer: the name without its extension is selected, and the extension stays even if it is erased.
import { el, keepClicks } from "../../dom";
import { extOf, growToFit, keepExtension, selectStem } from "../../molecules/keep-extension";

export function renameInPlace(tile: HTMLElement, name: string, commit: (next: string) => void): void {
  const label = tile.querySelector<HTMLElement>(".batch-tile-name");
  if (!label) return;
  const ext = extOf(name);
  // A text area wraps the whole name and grows past the tile, over its neighbours, as an explorer shows a long name being renamed.
  const input = el("textarea", { class: "name-input batch-name-input", "aria-label": `Rename ${name}`, spellcheck: "false", rows: "1" });
  input.value = name;
  keepExtension(input, ext);
  const fit = growToFit(input);
  const shown = label.cloneNode(true);
  let done = false;
  const finish = (save: boolean): void => {
    if (done) return;
    done = true;
    const next = input.value.replace(/\s*\n\s*/g, " ").trim();
    label.classList.remove("renaming");
    label.replaceChildren(...shown.childNodes);
    if (save && next && next !== ext && next !== name) commit(next);
  };
  input.addEventListener("keydown", (ev) => {
    ev.stopPropagation();
    if (ev.key === "Enter") finish(true);
    else if (ev.key === "Escape") finish(false);
    else return;
    ev.preventDefault();
  });
  input.addEventListener("blur", () => finish(true));
  input.addEventListener("contextmenu", (ev) => ev.stopPropagation());
  keepClicks(input);
  label.classList.add("renaming");
  label.replaceChildren(input);
  fit();
  input.focus();
  selectStem(input, ext);
}
