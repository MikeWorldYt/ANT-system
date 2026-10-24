// Settings window: sidebar with search, one page at a time, built from the pages in preferences/ and system/.
import "../../styles/settings.css";
import { describeError, getStorage } from "../../ipc/api";
import type { StorageInfo } from "../../ipc/types";
import { el } from "../../ui/dom";
import { guiIcon } from "../../ui/gui-icons";
import { setStatus } from "../../ui/status";
import { renderRow, type PageContext, type SettingRow, type SettingsPage } from "./page";
import { generalPage } from "./preferences/general";
import { shortcutsPage } from "./preferences/shortcuts";
import { advancedPage } from "./system/advanced";
import { aboutPage } from "./system/about";

const PAGES: SettingsPage[] = [generalPage, shortcutsPage, advancedPage, aboutPage];

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

export async function openSettings(): Promise<void> {
  let storage: StorageInfo | null = null;
  try {
    storage = await getStorage();
  } catch (e) {
    setStatus("warning", `Storage info: ${describeError(e)}`);
  }

  let currentId = PAGES[0]!.id;
  const title = el("h2", { id: "settings-title" });
  const body = el("div", { class: "sw-body" });
  const search = el("input", { type: "search", class: "sw-search-input", placeholder: "Search settings", "aria-label": "Search settings" });
  const nav = new Map<string, HTMLButtonElement>();
  const ctx: PageContext = { storage, refresh: () => show(currentId) };

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

  const close = el("button", { type: "button", class: "btn btn-primary" }, "Close");
  const main = el("section", { class: "sw-main" }, el("div", { class: "sw-head" }, title, close), body);
  const dialog = el("dialog", { class: "settings-window", "aria-labelledby": "settings-title" }, side, main);
  close.addEventListener("click", () => dialog.close());
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  show(currentId);
  dialog.showModal();
  search.focus();
}
