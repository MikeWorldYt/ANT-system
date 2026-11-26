// The picker's file list: one row per entry, as a plain list or as details with columns, the way Explorer's dialog shows them.
import { el } from "../../../dom";
import { kindIcon } from "../../../atoms/icons";
import { fileImage } from "../../../../services/ipc/api";
import { fileKind, formatSize } from "../../../windows/views/details-cells";
import type { TreeNode } from "../../../../services/ipc/types";

export type PickView = "details" | "list" | "grid" | "cards" | "columns";

export interface PickEntry {
  node: TreeNode;
  // Relative to the worknode, as the picker returns it.
  path: string;
  // Shown under the name while searching, so a match deep in the worknode says where it is.
  where: string | null;
  usable: boolean;
}

export interface EntryEvents {
  pick(entry: PickEntry): void;
  open(entry: PickEntry): void;
}

const stamp = (ms: number | null): string => (ms === null ? "" : new Date(ms).toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" }));

function icon(n: TreeNode): HTMLElement {
  if (!n.file) return kindIcon("folder");
  const img = el("img", { class: "icon", alt: "", width: "16", height: "16" });
  void fileImage(n.relParts, false)
    .then((src) => src && img.setAttribute("src", src))
    .catch((e: unknown) => console.warn("File icon unavailable", e));
  return img;
}

export const detailsHead = (): HTMLElement =>
  el("div", { class: "pick-row pick-head-row", role: "presentation" }, el("span", { class: "pick-name" }, "Name"), el("span", {}, "Date modified"), el("span", {}, "Type"), el("span", {}, "Size"));

// A click selects, a double click opens a folder or picks a file; disabled entries can still be opened when they are folders.
export function entryRow(entry: PickEntry, view: PickView, events: EntryEvents): HTMLElement {
  const n = entry.node;
  const name = el("span", { class: "pick-name" }, n.name, ...(entry.where ? [el("small", { class: "pick-where" }, entry.where)] : []));
  if (view === "cards") name.append(el("small", { class: "pick-where" }, fileKind(n)));
  const cells = view === "details" ? [el("span", {}, stamp(n.stamps.modified)), el("span", {}, fileKind(n)), el("span", {}, n.file ? formatSize(n.stamps.size) : "")] : [];
  const row = el("div", { class: `pick-row${entry.usable ? "" : " is-disabled"}`, role: "option", tabindex: "-1", "data-path": entry.path, "aria-selected": "false", "aria-disabled": String(!entry.usable) }, el("span", { class: "pick-name-cell" }, icon(n), name), ...cells);
  row.addEventListener("click", () => events.pick(entry));
  row.addEventListener("dblclick", () => events.open(entry));
  row.addEventListener("keydown", (ev) => {
    if (ev.key !== "Enter") return;
    ev.preventDefault();
    events.open(entry);
  });
  return row;
}
