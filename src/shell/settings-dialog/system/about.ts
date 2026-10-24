// About page: app and storage info, the update check, the changelog and the ways to report a bug.
import { changeSetting } from "../../../actions/ops";
import { checkForUpdates, type UpdateOutcome } from "../../../actions/updates";
import { describeError, openLink } from "../../../ipc/api";
import { state } from "../../../store/state";
import { setStatus } from "../../../ui/status";
import { actions, buttonControl, pathControl, switchControl, valueControl, type RowHandle, type SettingsPage } from "../page";

const MAIL = "info@mikeworld.dev";

const UPDATE_TEXT: Record<UpdateOutcome, string> = {
  latest: "You are on the latest version.",
  offered: "A newer version is available.",
  failed: "The update check failed.",
};

async function openOrReport(link: "issues" | "mail" | "changelog"): Promise<void> {
  try {
    await openLink(link);
  } catch (e) {
    setStatus("warning", describeError(e));
  }
}

export const aboutPage: SettingsPage = {
  id: "about",
  group: "System",
  title: "About",
  icon: "pageAbout",
  rows: (ctx) => [
    { title: "App", control: () => valueControl("ANT System Manager") },
    { title: "Version", control: () => valueControl(ctx.storage?.appVersion ?? "Unknown") },
    { title: "Storage", control: () => pathControl(ctx.storage?.root ?? "Unknown") },
    {
      title: "Updates",
      description: "Look for a newer version.",
      control: (row: RowHandle) =>
        buttonControl("Check for updates", () => void checkForUpdates(true).then((outcome) => row.describe(UPDATE_TEXT[outcome])), true),
    },
    {
      title: "Check for updates automatically",
      control: () => switchControl("Check for updates automatically", state.settings.autoUpdate, (on) => changeSetting("autoUpdate", on)),
    },
    { title: "What’s new in this version", control: () => buttonControl("View changelog", () => void openOrReport("changelog")) },
    {
      section: "Report a bug",
      title: `On GitHub, or by mail to ${MAIL}`,
      description: "The app version is added for you.",
      control: () => actions(buttonControl("Report on GitHub", () => void openOrReport("issues")), buttonControl("Email us", () => void openOrReport("mail"))),
    },
  ],
};
