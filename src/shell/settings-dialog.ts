// Settings window: list preferences kept in ant.db, app info, update check and ways to report a bug.
import { describeError, getStorage, openLink } from "../ipc/api";
import { el } from "../ui/dom";
import { changeSetting } from "../actions/ops";
import { state } from "../store/state";
import { setStatus } from "../ui/status";
import { checkForUpdates } from "../actions/updates";

const MAIL = "info@mikeworld.dev";

function section(title: string, ...body: (Node | string)[]): HTMLElement {
  return el("section", { class: "settings-section" }, el("h3", {}, title), ...body);
}

function infoRow(label: string, value: string): HTMLElement {
  return el("div", { class: "settings-row" }, el("span", { class: "settings-label" }, label), el("span", {}, value));
}

async function openOrReport(link: "issues" | "mail"): Promise<void> {
  try {
    await openLink(link);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

export async function openSettings(): Promise<void> {
  const pin = el("input", { type: "checkbox", id: "setting-pin" });
  pin.checked = state.settings.pinFavorites;
  pin.addEventListener("change", async () => {
    if (!(await changeSetting("pinFavorites", pin.checked))) pin.checked = state.settings.pinFavorites;
  });
  const hideDone = el("input", { type: "checkbox", id: "setting-hide-done" });
  hideDone.checked = state.settings.hideDone;
  hideDone.addEventListener("change", async () => {
    if (!(await changeSetting("hideDone", hideDone.checked))) hideDone.checked = state.settings.hideDone;
  });
  const about = el("div", { class: "settings-rows" });

  const close = el("button", { type: "submit", class: "btn btn-primary", value: "close" }, "Close");
  const update = el("button", { type: "button", class: "btn" }, "Check for updates");
  const issues = el("button", { type: "button", class: "btn" }, "Report on GitHub");
  const mail = el("button", { type: "button", class: "btn", title: MAIL }, "Email us");
  update.addEventListener("click", () => void checkForUpdates(true));
  issues.addEventListener("click", () => void openOrReport("issues"));
  mail.addEventListener("click", () => void openOrReport("mail"));

  const form = el(
    "form",
    { method: "dialog" },
    el("h2", { id: "settings-title" }, "Settings"),
    section(
      "Preference",
      el("label", { class: "settings-check", for: "setting-pin" }, pin, "Pin favorites to the top of each list"),
      el("label", { class: "settings-check", for: "setting-hide-done" }, hideDone, "Hide Workboards with status done"),
    ),
    section("About", about, el("div", { class: "settings-actions" }, update)),
    section("Report a bug", el("p", { class: "settings-hint" }, `On GitHub, or by mail to ${MAIL}. The app version is added for you.`), el("div", { class: "settings-actions" }, issues, mail)),
    el("div", { class: "dialog-buttons" }, close),
  );
  const dialog = el("dialog", { class: "confirm settings", "aria-labelledby": "settings-title" }, form);
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  dialog.showModal();
  pin.focus();

  try {
    const info = await getStorage();
    about.replaceChildren(infoRow("App", "ANT System Manager"), infoRow("Version", info.appVersion), infoRow("Storage", info.root));
  } catch (e) {
    about.replaceChildren(infoRow("Info", describeError(e)));
  }
}
