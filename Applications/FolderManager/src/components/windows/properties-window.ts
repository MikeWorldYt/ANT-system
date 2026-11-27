// The Properties window: Explorer's General tab for one item, read-only, with a folder's totals filling in as they are counted, and a Customise tab for the support nodes a node may hold.
import "../../styles/index.css";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { el } from "../dom";
import { checkbox } from "../atoms/checkbox";
import { textButton } from "../atoms/button";
import { textInput } from "../atoms/text-input";
import { describeError, itemProperties, itemTotals, supportNodes } from "../../services/ipc/api";
import { onItemTotals } from "../../services/ipc/events";
import { customise } from "./properties-customise";
import type { Customise } from "./properties-customise";
import { count, formatBytes, formatStamp } from "./properties-format";
import { tabs } from "./properties-tabs";
import type { ItemFacts, ItemTotals } from "../../services/ipc/types";

export interface PropertiesTarget {
  relParts: string[];
  icon: string;
}

const field = (label: string): [HTMLElement, HTMLElement] => {
  const value = el("dd", { class: "prop-value" });
  return [el("div", { class: "prop-row" }, el("dt", {}, label), value), value];
};

const flag = (label: string): [HTMLElement, HTMLInputElement] => {
  const box = checkbox({ disabled: "" });
  return [el("label", { class: "prop-flag" }, box, label), box];
};

export async function mountPropertiesWindow(target: PropertiesTarget): Promise<void> {
  const app = document.querySelector<HTMLDivElement>("#app");
  if (!app) throw new Error("Missing #app mount point");
  const name = textInput({ class: "prop-name", readonly: "", "aria-label": "Name" });
  const [kindRow, kind] = field("Type:");
  const [locationRow, location] = field("Location:");
  const [sizeRow, size] = field("Size:");
  const [diskRow, onDisk] = field("Size on disk:");
  const [containsRow, contains] = field("Contains:");
  const [createdRow, created] = field("Created:");
  const [modifiedRow, modified] = field("Modified:");
  const [accessedRow, accessed] = field("Accessed:");
  const [readOnlyLabel, readOnly] = flag("Read-only");
  const [hiddenLabel, hidden] = flag("Hidden");
  const sizes = el("dl", { class: "prop-group" }, sizeRow, diskRow, containsRow);
  const error = el("p", { class: "prop-error", role: "alert" });
  const close = (): void => {
    getCurrentWindow().close().catch((e: unknown) => {
      error.textContent = `Could not close the window: ${describeError(e)}`;
    });
  };
  let custom: Customise | undefined;
  let busy = false;
  const ok = textButton("OK", "btn-primary prop-button", () => void commit(true));
  const cancel = textButton("Cancel", "prop-button", close);
  const apply = textButton("Apply", "prop-button", () => void commit(false));
  apply.disabled = true;
  const sync = (): void => {
    apply.disabled = busy || !custom?.changed();
  };
  async function commit(leave: boolean): Promise<void> {
    if (busy) return;
    if (!custom?.changed()) {
      if (leave) close();
      return;
    }
    busy = true;
    error.textContent = "";
    ok.disabled = cancel.disabled = apply.disabled = true;
    const failure = await custom.apply();
    busy = false;
    ok.disabled = cancel.disabled = false;
    sync();
    if (failure) error.textContent = failure;
    else if (leave) close();
  }
  const icon = el("img", { class: "prop-icon", src: target.icon, alt: "", width: "32", height: "32", draggable: "false" });
  const general = el(
    "div",
    { class: "prop-body" },
    el("div", { class: "prop-head" }, icon, name),
    el("dl", { class: "prop-group" }, kindRow, locationRow),
    sizes,
    el("dl", { class: "prop-group" }, createdRow, modifiedRow, accessedRow),
    el("dl", { class: "prop-group" }, el("div", { class: "prop-row" }, el("dt", {}, "Attributes:"), el("dd", { class: "prop-value prop-flags" }, readOnlyLabel, hiddenLabel))),
  );
  const pages = tabs("Properties");
  app.replaceChildren(el("main", { class: "prop-window" }, pages.list, general, error, el("footer", { class: "prop-actions" }, ok, cancel, apply)));
  pages.add("General", general);
  document.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape") close();
  });
  ok.focus();

  // Only a storage node that can hold support nodes gets the tab.
  const attachCustomise = async (): Promise<void> => {
    try {
      const rows = await supportNodes(target.relParts);
      if (!rows) return;
      custom = customise(target.relParts, rows, sync);
      general.after(custom.panel);
      pages.add("Customise", custom.panel);
    } catch (e) {
      error.textContent = `Could not read the support nodes: ${describeError(e)}`;
    }
  };

  const showTotals = (t: ItemTotals): void => {
    size.textContent = formatBytes(t.size);
    onDisk.textContent = formatBytes(t.onDisk);
    contains.textContent = `${count(t.files)} Files, ${count(t.folders)} Folders`;
  };

  let facts: ItemFacts;
  try {
    facts = await itemProperties(target.relParts);
  } catch (e) {
    error.textContent = `Could not read the properties: ${describeError(e)}`;
    return;
  }
  name.value = facts.name;
  kind.textContent = facts.kind;
  location.textContent = facts.location;
  created.textContent = formatStamp(facts.created);
  modified.textContent = formatStamp(facts.modified);
  accessed.textContent = formatStamp(facts.accessed);
  readOnly.checked = facts.readOnly;
  hidden.checked = facts.hidden;
  document.title = `${facts.name} Properties`;
  modifiedRow.hidden = accessedRow.hidden = facts.folder;
  containsRow.hidden = !facts.folder;
  if (facts.folder) void attachCustomise();
  if (!facts.folder) {
    size.textContent = formatBytes(facts.size ?? 0);
    onDisk.textContent = formatBytes(facts.onDisk ?? 0);
    return;
  }
  size.textContent = onDisk.textContent = contains.textContent = "Calculating…";
  sizes.setAttribute("aria-busy", "true");
  let stop: (() => void) | undefined;
  let settled = false;
  try {
    stop = await onItemTotals((t) => {
      // A progress event can arrive after the answer; it must not overwrite the final numbers.
      if (!settled) showTotals(t);
    });
    const totals = await itemTotals(target.relParts);
    settled = true;
    if (totals) showTotals(totals);
  } catch (e) {
    settled = true;
    size.textContent = onDisk.textContent = contains.textContent = "Unavailable";
    error.textContent = `Could not count the contents: ${describeError(e)}`;
  } finally {
    stop?.();
    sizes.removeAttribute("aria-busy");
  }
}
