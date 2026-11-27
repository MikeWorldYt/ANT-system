// Picks a worknode's output inside the app: a small Explorer-like browser that can enter its folders but never leave the worknode.
import { el } from "../../../dom";
import { cancelButton, dialogButton } from "../../../atoms/button";
import { iconButton } from "../../../atoms/icon-button";
import { selectBox } from "../../../atoms/select";
import { textInput } from "../../../atoms/text-input";
import { listChildren } from "../../../../services/ipc/api";
import { readPref, savePref } from "../../../../services/store/prefs";
import { openDialog } from "../dialog-shell";
import { pickBar } from "./pick-bar";
import { detailsHead, entryRow, type PickEntry, type PickView } from "./pick-entries";
import { columnLists } from "./pick-views";
import type { GuiIcon } from "../../../atoms/gui-icons";
import type { TreeNode } from "../../../../services/ipc/types";

const VIEW_PREF = "fm.pickView";
// The app's views that list one folder; Graph and Tree draw the whole Storage instead.
const VIEWS: [PickView, string, GuiIcon][] = [["details", "Details", "viewDetails"], ["list", "List", "viewList"], ["grid", "Grid", "viewGrid"], ["cards", "Cards", "viewCards"], ["columns", "Columns", "viewColumns"]];
const ALL = "*";
// A search walks the folders below the current one; this keeps a huge worknode from stalling the dialog.
const SEARCH_LIMIT = 400;

const extOf = (name: string): string => (name.includes(".") ? (name.split(".").pop() ?? "").toLowerCase() : "");
const visibleEntries = (nodes: TreeNode[]): TreeNode[] => nodes.filter((n) => !n.name.toLowerCase().endsWith(".lnk"));

// Resolves with the path relative to the worknode, or undefined when the user cancels.
export async function pickInsideDialog(worknode: TreeNode, directory: boolean, start: string[] = []): Promise<string | undefined> {
  let path = [...start];
  let entries: PickEntry[] = [];
  let picked: PickEntry | null = null;
  let query = "";
  let view: PickView = VIEWS.find(([id]) => id === readPref(VIEW_PREF))?.[0] ?? "details";
  const history: string[][] = [];
  const ahead: string[][] = [];

  const list = el("div", { class: "pick-list", role: "listbox", tabindex: "0", "aria-label": directory ? "Folders" : "Files and folders" });
  const status = el("p", { class: "pick-empty", role: "status" });
  const viewButtons = VIEWS.map(([id, label, icon]) => {
    const b = iconButton(label, icon, () => {
      view = id;
      savePref(VIEW_PREF, id);
      void paint();
    });
    return [id, b] as const;
  });
  const fileName = textInput({ class: "sw-input pick-file-name", "aria-label": directory ? "Folder name" : "File name" });
  const filter = selectBox("File type", [], "sw-select pick-filter");
  const open = dialogButton(directory ? "Select folder" : "Open", "ok", "primary");

  const bar = pickBar({
    back: () => history.length && go(history.pop() ?? [], "back"),
    forward: () => ahead.length && go(ahead.pop() ?? [], "forward"),
    up: () => void go(path.slice(0, -1)),
    go: (to) => go(to),
    search: (q) => {
      query = q;
      void load();
    },
  });

  const { dialog, closed } = openDialog({
    title: directory ? "Choose a folder" : "Choose a file",
    wide: true,
    body: [
      el("div", { class: "pick-tools", role: "group", "aria-label": "View" }, ...viewButtons.map(([, b]) => b)),
      bar.element,
      list,
      el("div", { class: "pick-footer" }, el("label", { class: "pick-field" }, directory ? "Folder:" : "File name:", fileName), ...(directory ? [] : [filter])),
    ],
    buttons: [cancelButton(), open],
    focus: () => list.focus(),
  });
  dialog.classList.add("pick-inside");

  const usable = (n: TreeNode): boolean => (directory ? !n.file : n.file && (filter.value === ALL || extOf(n.name) === filter.value));

  // Only the current folder can be reached through the bar, so a typed path is checked against the worknode before going.
  async function go(to: string[], how: "push" | "back" | "forward" = "push"): Promise<boolean> {
    try {
      await listChildren([...worknode.relParts, ...to]);
    } catch (e) {
      console.warn("Picker folder not found", e);
      return false;
    }
    if (how === "push" || how === "forward") history.push(path);
    if (how === "back") ahead.push(path);
    if (how === "push") ahead.length = 0;
    path = to;
    query = "";
    bar.clearSearch();
    await load();
    return true;
  }

  async function walk(from: string[], into: PickEntry[]): Promise<void> {
    const queue = [from];
    while (queue.length && into.length < SEARCH_LIMIT) {
      const at = queue.shift() ?? [];
      const nodes = visibleEntries(await listChildren([...worknode.relParts, ...at]));
      for (const n of nodes) {
        if (n.name.toLowerCase().includes(query.toLowerCase())) into.push({ node: n, path: [...at, n.name].join("/"), where: at.length > from.length ? at.slice(from.length).join("/") : null, usable: usable(n) });
        if (!n.file) queue.push([...at, n.name]);
      }
    }
  }

  async function load(): Promise<void> {
    picked = null;
    bar.show(worknode.name, path, history.length > 0, ahead.length > 0);
    try {
      if (query) {
        entries = [];
        await walk(path, entries);
      } else entries = visibleEntries(await listChildren([...worknode.relParts, ...path])).map((n) => ({ node: n, path: [...path, n.name].join("/"), where: null, usable: usable(n) }));
    } catch (e) {
      console.error("Picker listing failed", e);
      entries = [];
      status.textContent = "This folder could not be read.";
      list.replaceChildren(status);
      return;
    }
    const exts = [...new Set(entries.filter((e) => e.node.file).map((e) => extOf(e.node.name)).filter(Boolean))].sort();
    const keep = exts.includes(filter.value) ? filter.value : ALL;
    filter.replaceChildren(el("option", { value: ALL }, "All files (*.*)"), ...exts.map((x) => el("option", { value: x }, `${x.toUpperCase()} file (*.${x})`)));
    filter.value = keep;
    await paint();
  }

  const loadLevel = async (at: string[]): Promise<PickEntry[]> =>
    visibleEntries(await listChildren([...worknode.relParts, ...at])).map((n) => ({ node: n, path: [...at, n.name].join("/"), where: null, usable: usable(n) }));

  // Search results are one flat list whatever the view, as Explorer shows them.
  async function paint(): Promise<void> {
    for (const [id, b] of viewButtons) b.setAttribute("aria-pressed", String(id === view));
    const shown: PickView = query && view === "columns" ? "details" : view;
    list.className = `pick-list view-${shown}`;
    for (const e of entries) e.usable = usable(e.node);
    const events = { pick: select, open: activate };
    status.textContent = query ? "No items match your search." : "This folder is empty.";
    let rows: HTMLElement[];
    try {
      // In Columns a click on a folder opens it in the next column, as the app's Columns view does.
      if (shown === "columns") rows = await columnLists(path, entries, loadLevel, { pick: (e) => (e.node.file ? select(e) : void go(e.path.split("/"))), open: activate });
      else rows = entries.map((e) => entryRow(e, shown, events));
    } catch (e) {
      console.error("Picker view failed", e);
      rows = [];
    }
    list.replaceChildren(...(shown === "details" && rows.length ? [detailsHead()] : []), ...(rows.length || shown === "columns" ? rows : [status]));
    if (shown === "columns") list.scrollLeft = list.scrollWidth;
    sync();
  }

  function sync(): void {
    for (const row of list.querySelectorAll<HTMLElement>(".pick-row[data-path]")) row.setAttribute("aria-selected", String(row.dataset.path === picked?.path));
    open.disabled = !(picked?.usable || (directory && path.length > 0 && !fileName.value));
  }

  function select(entry: PickEntry): void {
    picked = entry;
    fileName.value = entry.usable ? entry.node.name : fileName.value;
    sync();
  }

  // A folder is entered unless the dialog is choosing folders and this one is picked already; a usable file closes the dialog.
  function activate(entry: PickEntry): void {
    if (!entry.node.file && !(directory && picked === entry && fileName.value)) {
      void go(entry.path.split("/"));
      return;
    }
    if (!entry.usable) return;
    picked = entry;
    dialog.close("ok");
  }

  // Typing a name selects the first entry that starts with it; Enter opens it, as in Explorer.
  fileName.addEventListener("input", () => {
    const typed = fileName.value.trim().toLowerCase();
    picked = typed ? (entries.find((e) => e.node.name.toLowerCase().startsWith(typed)) ?? null) : null;
    sync();
    list.querySelector<HTMLElement>('.pick-row[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  });
  fileName.addEventListener("keydown", (ev) => {
    if (ev.key !== "Enter") return;
    ev.preventDefault();
    const exact = entries.find((e) => e.node.name.toLowerCase() === fileName.value.trim().toLowerCase()) ?? picked;
    if (exact) activate(exact);
  });
  filter.addEventListener("change", () => void paint());
  list.addEventListener("keydown", (ev) => {
    const rows = [...list.querySelectorAll<HTMLElement>(".pick-row[data-path]")];
    const at = rows.indexOf(document.activeElement as HTMLElement);
    const to = ev.key === "ArrowDown" ? at + 1 : ev.key === "ArrowUp" ? at - 1 : ev.key === "Home" ? 0 : ev.key === "End" ? rows.length - 1 : null;
    if (to !== null) {
      ev.preventDefault();
      const row = rows[Math.max(0, Math.min(rows.length - 1, to))];
      row?.focus();
      row?.click();
    } else if (ev.key === "Backspace" && path.length > 0) {
      ev.preventDefault();
      void go(path.slice(0, -1));
    }
  });
  // Explorer's folder dialog picks the folder it is in when nothing inside is selected.
  open.addEventListener("click", (ev) => {
    if (picked?.usable) return;
    if (directory && path.length > 0) {
      picked = { node: worknode, path: path.join("/"), where: null, usable: true };
      return;
    }
    ev.preventDefault();
  });

  await load();
  const answer = await closed;
  // Set by the handlers while the dialog was open, which the compiler cannot follow.
  const chosen = picked as PickEntry | null;
  return answer === "ok" && chosen?.usable ? chosen.path : undefined;
}
