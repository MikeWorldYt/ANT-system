// Extensions page: install from a repository or the library, turn installed ones on and off, remove them.
import {
  describeError,
  inspectExtensionRepo,
  installInspectedExtension,
  installRegistryExtension,
  listExtensions,
  listRegistry,
  removeExtension,
  setExtensionEnabled,
  type ExtensionInfo,
  type RegistryEntry,
} from "../../../ipc/api";
import { reloadExtensions } from "../../../extensions/host";
import { describePermission } from "../../../extensions/protocol";
import { confirmDialog } from "../../../ui/dialogs";
import { setStatus } from "../../../ui/status";
import { actions, buttonControl, inputControl, switchControl, valueControl, type PageContext, type RowHandle, type SettingRow, type SettingsPage } from "../page";

type Loaded<T> = { state: "loading" } | { state: "ok"; value: T } | { state: "error"; message: string };

const UNVALIDATED = "Unvalidated: this code was not reviewed by the ANT developers. It runs isolated, but install it only if you trust its author.";

// Keyed by the Storage root, so switching Storage reads its own extensions again.
let cache: { root: string; installed: Loaded<ExtensionInfo[]>; library: Loaded<RegistryEntry[]> } | null = null;

function load<T>(fetch: () => Promise<T>, set: (v: Loaded<T>) => void, ctx: PageContext): Loaded<T> {
  void fetch().then(
    (value) => set({ state: "ok", value }),
    (e) => set({ state: "error", message: describeError(e) }),
  ).then(ctx.refresh);
  return { state: "loading" };
}

function data(ctx: PageContext): NonNullable<typeof cache> {
  const root = ctx.storage?.root ?? "";
  if (cache?.root === root) return cache;
  const fresh: NonNullable<typeof cache> = { root, installed: { state: "loading" }, library: { state: "loading" } };
  cache = fresh;
  fresh.installed = load(listExtensions, (v) => (fresh.installed = v), ctx);
  fresh.library = load(listRegistry, (v) => (fresh.library = v), ctx);
  return fresh;
}

function changed(ctx: PageContext): void {
  cache = null;
  ctx.refresh();
  reloadExtensions();
}

async function attempt(row: RowHandle, fn: () => Promise<void>): Promise<void> {
  try {
    await fn();
  } catch (e) {
    row.describe(describeError(e));
  }
}

const permissionsText = (perms: string[]) => perms.map(describePermission).join("; ");

function findRepo(url: string, row: RowHandle, ctx: PageContext): Promise<void> {
  return attempt(row, async () => {
    row.describe("Looking up the repository…");
    const p = await inspectExtensionRepo(url);
    if (p.alreadyInstalled) return row.describe(`${p.name} is already installed.`);
    row.describe("");
    const message = `${p.name} ${p.version} by ${p.author} (${Math.ceil(p.size / 1024)} KB). It asks to: ${permissionsText(p.permissions)}. ${UNVALIDATED}`;
    if (!(await confirmDialog(`Install ${p.name}?`, message, "Install"))) return;
    await installInspectedExtension(p.token);
    changed(ctx);
  });
}

function installedRow(ext: ExtensionInfo, ctx: PageContext): SettingRow {
  const remove = (row: RowHandle) =>
    buttonControl(
      "Remove",
      () =>
        void attempt(row, async () => {
          if (!(await confirmDialog("Remove extension", `${ext.name} and its folder are deleted.`, "Remove"))) return;
          await removeExtension(ext.id);
          changed(ctx);
        }),
      "danger",
    );
  const toggle = async (on: boolean): Promise<boolean> => {
    try {
      await setExtensionEnabled(ext.id, on);
      ext.enabled = on;
      reloadExtensions();
      return true;
    } catch (e) {
      setStatus("warning", `Extension ${ext.name}: ${describeError(e)}`);
      return false;
    }
  };
  const title = `${ext.name} ${ext.version}`;
  if (ext.status === "rejected") return { section: "Installed", title, description: ext.reason ?? "Rejected.", control: (row) => remove(row) };
  return {
    section: "Installed",
    title,
    description: `${ext.author} · ${ext.trust === "validated" ? "Validated" : "Unvalidated"} · ${permissionsText(ext.permissions)}`,
    control: (row) => actions(switchControl(`Enable ${ext.name}`, ext.enabled, toggle), remove(row)),
  };
}

function libraryRow(entry: RegistryEntry, ctx: PageContext): SettingRow {
  const install = (row: RowHandle) => {
    const button = buttonControl(entry.installed ? "Installed" : "Install", () =>
      void attempt(row, async () => {
        button.disabled = true;
        await installRegistryExtension(entry.id).finally(() => (button.disabled = false));
        changed(ctx);
      }),
    );
    button.disabled = entry.installed;
    return button;
  };
  return { section: "Library", title: `${entry.name} ${entry.version}`, description: `${entry.author} · ${entry.description}`, control: install };
}

function rowsOf<T>(section: string, loaded: Loaded<T[]>, empty: string, row: (item: T) => SettingRow, retry: () => void): SettingRow[] {
  if (loaded.state === "loading") return [{ section, title: "Loading…", control: () => valueControl("") }];
  if (loaded.state === "error") return [{ section, title: "Unavailable", description: loaded.message, control: () => buttonControl("Retry", retry) }];
  if (loaded.value.length === 0) return [{ section, title: empty, control: () => valueControl("") }];
  return loaded.value.map(row);
}

export const extensionsPage: SettingsPage = {
  id: "extensions",
  group: "System",
  title: "Extensions",
  icon: "pageExtensions",
  rows: (ctx) => {
    const d = data(ctx);
    const retry = () => {
      cache = null;
      ctx.refresh();
    };
    return [
      {
        section: "Add from a repository",
        title: "Repository address",
        description: "A public GitHub address, such as https://github.com/owner/repo.",
        control: (row) => inputControl("Repository address", "https://github.com/…", "Find", (url) => void findRepo(url, row, ctx)),
      },
      ...rowsOf("Installed", d.installed, "No extensions installed.", (ext) => installedRow(ext, ctx), retry),
      ...rowsOf("Library", d.library, "The library is empty.", (entry) => libraryRow(entry, ctx), retry),
    ];
  },
};
