// The Details view's columns: date modified, type, size and date created beside each name, under a header that sorts by them.
import { el } from "../../dom";
import { setColumnSort, sortOf, type SortBy } from "../../../services/store/state";
import type { TreeNode } from "../../../services/ipc/types";

export function fileKind(n: TreeNode): string {
  if (!n.file) return "Folder";
  const dot = n.name.lastIndexOf(".");
  return dot > 0 ? `${n.name.slice(dot + 1).toUpperCase()} file` : "File";
}

const stamp = (ms: number | null): string => (ms === null ? "" : new Date(ms).toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" }));

// Rounded up to whole KB, as Explorer shows it, so a tiny file never reads as 0.
const size = (bytes: number | null): string => (bytes === null ? "" : `${Math.max(1, Math.ceil(bytes / 1024)).toLocaleString()} KB`);

export const detailCells = (n: TreeNode): HTMLElement[] => [
  el("span", { class: "detail-cell detail-date" }, stamp(n.stamps.modified)),
  el("span", { class: "detail-cell detail-type" }, fileKind(n)),
  el("span", { class: "detail-cell detail-size" }, size(n.stamps.size)),
  el("span", { class: "detail-cell detail-date" }, stamp(n.stamps.created)),
];

const COLUMNS: [string, string, SortBy | null][] = [
  ["Name", "detail-name", "name"],
  ["Date modified", "detail-date", "modified"],
  ["Type", "detail-type", null],
  ["Size", "detail-size", "size"],
  ["Date created", "detail-date", "created"],
];

export function detailsHeader(columnKey: string): HTMLElement {
  const { by, dir } = sortOf(columnKey);
  const cells = COLUMNS.map(([label, cls, key]) => {
    if (!key) return el("span", { class: `detail-cell ${cls}` }, label);
    const on = by === key;
    const button = el("button", { type: "button", class: `detail-cell detail-sort ${cls}`, "aria-pressed": String(on) }, label, on ? (dir === "asc" ? " ↑" : " ↓") : "");
    // A second click on the sorted column flips its direction, as in Explorer.
    button.addEventListener("click", () => setColumnSort(columnKey, on ? { dir: dir === "asc" ? "desc" : "asc" } : { by: key, dir: "asc" }));
    return button;
  });
  return el("div", { class: "details-head", role: "presentation" }, ...cells);
}
