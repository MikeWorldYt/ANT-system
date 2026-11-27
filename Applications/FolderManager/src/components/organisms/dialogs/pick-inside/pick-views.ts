// The picker's Columns view, which lists every level down to the current folder and so loads its own.
import { el } from "../../../dom";
import { guiIcon } from "../../../atoms/gui-icons";
import { entryRow, type EntryEvents, type PickEntry } from "./pick-entries";

export type Loader = (path: string[]) => Promise<PickEntry[]>;

// One column per level from the worknode down to the current folder; the folder open in each column stays marked.
export async function columnLists(path: string[], current: PickEntry[], load: Loader, events: EntryEvents): Promise<HTMLElement[]> {
  const columns: HTMLElement[] = [];
  for (let i = 0; i <= path.length; i++) {
    const entries = i === path.length ? current : await load(path.slice(0, i));
    const onPath = path[i];
    columns.push(
      el("div", { class: "pick-column" }, ...entries.map((entry) => {
        const row = entryRow(entry, "columns", events);
        if (!entry.node.file) row.append(guiIcon("chevronRight"));
        if (entry.node.name === onPath && i < path.length) row.classList.add("on-path");
        return row;
      })),
    );
  }
  return columns;
}
