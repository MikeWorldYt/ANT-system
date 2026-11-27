// Storage picker: lists the Storages this install has opened and creates or opens another, so the app needs no copy at each root.
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";

import { el } from "../../../dom";
import { cancelButton, textButton } from "../../../atoms/button";
import { selectControl } from "../../../atoms/select";
import { textInput } from "../../../atoms/text-input";
import { attachMenu } from "../../menus/anchored-menu";
import { changeStoragePath, createStorage, describeError, forgetStorage, knownStorages, moveListedStorage, renameListedStorage, revealStorage, storageLocations } from "../../../../services/ipc/api";
import type { KnownStorage, StorageLocation } from "../../../../services/ipc/api";
import { promptDialog } from "../action-dialog/form-dialog";
import { choiceDialog } from "../info-dialog/info-dialog";
import { openDialog } from "../dialog-shell";
import logo from "../../../../../src-tauri/icons/128x128.png";

export type PickResult = "opened" | "created" | null;

export const sameFolder = (a: string, b: string): boolean => a.replace(/[\\/]+$/, "").toLowerCase() === b.replace(/[\\/]+$/, "").toLowerCase();

const option = (title: string, text: string, control: HTMLElement): HTMLElement =>
  el("div", { class: "picker-option" }, el("div", {}, el("strong", {}, title), el("p", {}, text)), control);

// The onboarding form fills the window and cannot be dismissed, since the app has nothing to show without a Storage.
export async function storagePicker(current: string | null, onboarding = false): Promise<PickResult> {
  let result: PickResult = null;
  const feedback = el("p", { class: "picker-feedback", role: "alert" });
  const list = el("ul", { class: "picker-list", "aria-label": "Storages" });
  const panel = el("div", { class: "picker-panel" });
  const version = el("p", { class: "picker-version" });
  const locations = storageLocations().catch((e: unknown): StorageLocation[] => {
    console.error("Could not list the drives", e);
    return [];
  });

  async function run(action: () => Promise<PickResult>): Promise<void> {
    feedback.textContent = "";
    try {
      result = await action();
      if (result) shell.dialog.close();
    } catch (e) {
      feedback.textContent = describeError(e);
    }
  }

  async function change(action: () => Promise<unknown>): Promise<void> {
    feedback.textContent = "";
    try {
      await action();
      await fill();
    } catch (e) {
      feedback.textContent = describeError(e);
    }
  }

  // Windows offers only drives; elsewhere any folder, at the user's own risk.
  async function pickLocation(title: string): Promise<string | null> {
    const drives = await locations;
    if (!drives.length) {
      const picked = await open({ directory: true, multiple: false, title });
      return typeof picked === "string" ? picked : null;
    }
    const chosen = await choiceDialog(title, "Choose the drive for the Storage.", drives.map((d): [string, string] => [d.path, d.label]));
    return chosen === "" || chosen === "cancel" ? null : chosen;
  }

  async function rename(storage: KnownStorage): Promise<void> {
    const name = await promptDialog("Rename Storage", "New name for the Storage folder.", storage.name, "Rename");
    if (name && name !== storage.name) await change(() => renameListedStorage(storage.path, name));
  }

  async function move(storage: KnownStorage): Promise<void> {
    const destination = await pickLocation("Move Storage");
    if (destination) await change(() => moveListedStorage(storage.path, destination));
  }

  function storageMenu(storage: KnownStorage, editable: boolean): HTMLButtonElement {
    const dots = el("button", { type: "button", class: "picker-more", "aria-label": `Actions for ${storage.name}`, title: "More actions" }, "⋮");
    attachMenu(dots, `${storage.name} actions`, () => [
      ...(editable
        ? [
            { label: "Rename Storage…", icon: "pencil" as const, run: () => void rename(storage) },
            { label: "Move Storage…", icon: "forward" as const, run: () => void move(storage) },
          ]
        : []),
      ...(storage.exists ? [{ label: "Reveal in system explorer", icon: "pageStorage" as const, divider: editable, run: () => void change(() => revealStorage(storage.path)) }] : []),
      { label: "Remove from list", icon: "trash" as const, divider: true, danger: true, run: () => void change(() => forgetStorage(storage.path)) },
    ], true);
    return dots;
  }

  function row(storage: KnownStorage): HTMLElement {
    const isCurrent = current !== null && sameFolder(storage.path, current);
    const top = el("span", { class: "picker-top" }, el("strong", { class: "picker-name" }, storage.name));
    if (!storage.exists) top.append(el("span", { class: "picker-tag picker-tag-warn" }, "Not found"));
    else if (isCurrent) top.append(el("span", { class: "picker-tag" }, "Current"));
    const main = el("button", { type: "button", class: "picker-open", title: storage.path }, top, el("span", { class: "picker-row-path" }, el("bdi", { dir: "ltr" }, storage.path)));
    main.disabled = !storage.exists || isCurrent;
    main.addEventListener("click", () => void run(async () => (await changeStoragePath(storage.path), "opened")));
    const item = el("li", { class: "picker-item" }, main, storageMenu(storage, storage.exists && !isCurrent));
    item.classList.toggle("is-current", isCurrent);
    item.classList.toggle("is-missing", !storage.exists);
    return item;
  }

  async function fill(): Promise<void> {
    const storages = await knownStorages();
    list.replaceChildren(...(storages.length ? storages.map(row) : [el("li", { class: "picker-empty" }, "No Storages yet.")]));
  }

  const openFolder = () =>
    run(async () => {
      const picked = await open({ directory: true, multiple: false, title: "Open a Storage folder" });
      if (typeof picked !== "string") return null;
      await changeStoragePath(picked);
      return "opened";
    });

  function showHome(): void {
    feedback.textContent = "";
    const card = el(
      "div",
      { class: "picker-card" },
      option("Create new Storage", "Create a new ANT Storage under a folder.", textButton("Create", "btn-primary", showCreate)),
      option("Open folder as Storage", "Choose an existing ANT Storage folder.", textButton("Open", "picker-secondary", () => void openFolder())),
    );
    panel.replaceChildren(card);
    card.querySelector("button")?.focus();
  }

  function showCreate(): void {
    feedback.textContent = "";
    let parent: string | null = null;
    const name = textInput({ class: "dialog-input", placeholder: "Storage name", "aria-label": "Storage name" });
    const where = el("div", { class: "picker-location" });
    const browse = textButton("Browse", "picker-secondary", async () => {
      const picked = await open({ directory: true, multiple: false, title: "Where to create the Storage" });
      if (typeof picked !== "string") return;
      parent = picked;
      path.textContent = picked;
    });
    const path = el("span", { class: "picker-path" });
    where.append(path, browse);
    void locations.then((drives: StorageLocation[]) => {
      const [first] = drives;
      if (!first) return;
      parent = first.path;
      const text = where.closest(".picker-option")?.querySelector("p");
      if (text) text.textContent = "Pick the drive for your new Storage.";
      where.replaceChildren(selectControl("Drive", drives.map((d): [string, string] => [d.path, d.label]), first.path, (v) => (parent = v)));
    });
    const back = el("button", { type: "button", class: "picker-back" }, "← Back");
    back.addEventListener("click", showHome);
    const create = textButton("Create", "btn-primary", () =>
      void run(async () => {
        if (!name.value.trim()) throw new Error("Pick a name for the Storage.");
        if (!parent) throw new Error("Pick a place to put the Storage.");
        await createStorage(parent, name.value);
        return "created";
      }),
    );
    const hint = "Pick a place near your home folder where you can read and write.";
    panel.replaceChildren(
      back,
      el("h3", {}, "Create new Storage"),
      el("div", { class: "picker-card" }, option("Storage name", "Pick a name for your Storage.", name), option("Location", hint, where)),
      el("div", { class: "picker-submit" }, create),
    );
    name.focus();
  }

  const brand = el("div", { class: "picker-brand" }, el("img", { src: logo, alt: "", class: "picker-logo" }), el("p", { class: "picker-welcome" }, "Welcome"), version);
  const main = el("div", { class: "picker-main" }, brand, panel, feedback);
  const shell = openDialog({ title: "ANT System Manager", wide: true, body: [el("div", { class: "picker" }, el("aside", { class: "picker-side" }, el("p", { class: "picker-side-title" }, "Your Storages"), list), main)], buttons: onboarding ? [] : [cancelButton("Close")] });
  shell.dialog.classList.add("picker-dialog");
  if (onboarding) {
    shell.dialog.classList.add("picker-onboarding");
    shell.dialog.addEventListener("cancel", (ev) => ev.preventDefault());
  }
  const heading = shell.form.querySelector("h2");
  if (heading) version.before(heading);
  showHome();
  void getVersion().then(
    (v) => (version.textContent = `Version ${v}`),
    (e: unknown) => console.error("Could not read the app version", e),
  );
  await fill().catch((e: unknown) => (feedback.textContent = describeError(e)));
  await shell.closed;
  return result;
}
