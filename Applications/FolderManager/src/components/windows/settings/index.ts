// Settings window: sidebar with search, one page at a time, built from the pages in preferences/ and system/.
import { getStorage } from "../../../services/ipc/api";
import type { StorageInfo } from "../../../services/ipc/types";
import { el } from "../../dom";
import { guiIcon } from "../../atoms/gui-icons";
import { textButton } from "../../atoms/button";
import { showModal } from "../../organisms/dialogs/dialog-shell";
import { reportError } from "../../organisms/bars/status-line";
import { renderRow, type SettingRow } from "../../molecules/setting-row";
import type { PageContext, SettingsPage } from "./page";
import { generalPage } from "./preferences/general";
import { appearancePage } from "./preferences/appearance";
import { shortcutsPage } from "./preferences/shortcuts";
import { backupPage } from "./data/backup";
import { storagePage } from "./data/storage";
import { extensionsPage } from "./system/extensions";
import { advancedPage } from "./system/advanced";
import { aboutPage } from "./system/about";

const PAGES: SettingsPage[] = [generalPage, appearancePage, shortcutsPage, storagePage, backupPage, extensionsPage, advancedPage, aboutPage];

function sections(rows: SettingRow[]): HTMLElement[] {
  const out: HTMLElement[] = [];
  let current: string | undefined;
  for (const row of rows) {
    if (row.section && row.section !== current) out.push(el("h3", {}, row.section));
    current = row.section;
    out.push(renderRow(row));
  }
  return out;
}

export async function openSettings(pageId?: string): Promise<void> {
  let storage: StorageInfo | null = null;
  try {
    storage = await getStorage();
  } catch (e) {
    reportError("warning", e, "Storage info");
  }

  let currentId = PAGES.find((p) => p.id === pageId)?.id ?? PAGES[0]!.id;
  const title = el("h2", { id: "settings-title" });
  const body = el("div", { class: "sw-body" });
  const search = el("input", { type: "search", class: "sw-search-input", placeholder: "Search settings", "aria-label": "Search settings" });
  const nav = new Map<string, HTMLButtonElement>();
  const ctx: PageContext = {
    storage,
    refresh: () => {
      // A rename changes the path every page shows, so the info is read again.
      void getStorage().then(
        (info) => {
          ctx.storage = info;
          show(currentId);
        },
        (e) => reportError("warning", e, "Storage info"),
      );
    },
  };

  const side = el("aside", { class: "sw-side" }, el("div", { class: "sw-search" }, guiIcon("search"), search));
  let group = "";
  for (const page of PAGES) {
    if (page.group !== group) {
      group = page.group;
      side.append(el("div", { class: "sw-group" }, group));
    }
    const button = el("button", { type: "button", class: "sw-nav" }, guiIcon(page.icon), page.title);
    button.addEventListener("click", () => {
      search.value = "";
      show(page.id);
    });
    nav.set(page.id, button);
    side.append(button);
  }

  function show(id: string): void {
    currentId = id;
    const page = PAGES.find((p) => p.id === id)!;
    title.textContent = page.title;
    body.replaceChildren(...sections(page.rows(ctx)));
    body.scrollTop = 0;
    nav.forEach((button, pageId) => {
      button.classList.remove("dim");
      if (pageId === id) button.setAttribute("aria-current", "page");
      else button.removeAttribute("aria-current");
    });
  }

  function find(query: string): void {
    const needle = query.toLowerCase();
    title.textContent = "Results";
    body.replaceChildren();
    nav.forEach((button) => button.removeAttribute("aria-current"));
    let found = 0;
    for (const page of PAGES) {
      const rows = page.rows(ctx).filter((r) => `${r.title} ${r.description ?? ""}`.toLowerCase().includes(needle));
      nav.get(page.id)?.classList.toggle("dim", rows.length === 0);
      if (rows.length === 0) continue;
      found += rows.length;
      body.append(el("h3", {}, page.title), ...rows.map(renderRow));
    }
    if (found === 0) body.append(el("div", { class: "sw-empty" }, `No settings match “${query}”.`));
  }

  search.addEventListener("input", () => (search.value.trim() ? find(search.value.trim()) : show(currentId)));

  const close = textButton("Close", "btn-primary", () => dialog.close());
  const main = el("section", { class: "sw-main" }, el("div", { class: "sw-head" }, title, close), body);
  const dialog = el("dialog", { class: "settings-window", "aria-labelledby": "settings-title" }, side, main);
  show(currentId);
  // A press that starts inside and ends on the backdrop, like a text selection, keeps the window open.
  dialog.addEventListener("pointerdown", (ev) => {
    const r = dialog.getBoundingClientRect();
    if (ev.target === dialog && (ev.clientX < r.left || ev.clientX > r.right || ev.clientY < r.top || ev.clientY > r.bottom)) dialog.close();
  });
  void showModal(dialog);
  search.focus();
}
