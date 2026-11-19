// About page: app and storage info, the update check, the changelog and the ways to report a bug.
import { changeSetting } from "../../../../hooks/ops";
import { checkForUpdates, type UpdateOutcome } from "../../../../hooks/updates";
import { openLink } from "../../../../services/ipc/api";
import { state } from "../../../../services/store/state";
import { reportError } from "../../../organisms/bars/status-line";
import { startTour } from "../../../organisms/tour/onboarding-tour";
import { buttonControl } from "../../../atoms/button";
import { pathControl, valueControl } from "../../../atoms/label";
import { switchControl } from "../../../atoms/switch";
import { actions } from "../../../molecules/button-row";
import type { RowHandle } from "../../../molecules/setting-row";
import type { SettingsPage } from "../page";

const MAIL = "info@mikeworld.dev";

const UPDATE_TEXT: Record<UpdateOutcome, string> = {
  latest: "You are on the latest version.",
  offered: "A newer version is available.",
  failed: "The update check failed.",
};

async function openOrReport(link: "issues" | "mail" | "changelog" | "docs"): Promise<void> {
  try {
    await openLink(link);
  } catch (e) {
    reportError("warning", e);
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
      title: "Tutorial",
      description: "Walk through the basic controls again.",
      control: () => {
        const button = buttonControl("Show tutorial", () => {
          button.closest("dialog")?.close();
          void startTour();
        }, "outline");
        return button;
      },
    },
    {
      title: "Updates",
      description: "Look for a newer version.",
      control: (row: RowHandle) =>
        buttonControl("Check for updates", () => void checkForUpdates(true).then((outcome) => row.describe(UPDATE_TEXT[outcome])), "outline"),
    },
    {
      title: "Check for updates automatically",
      control: () => switchControl("Check for updates automatically", state.settings.autoUpdate, (on) => changeSetting("autoUpdate", on)),
    },
    {
      title: "Install updates automatically",
      description: "At launch, a newer version is installed without asking.",
      control: () => switchControl("Install updates automatically", state.settings.installUpdates, (on) => changeSetting("installUpdates", on)),
    },
    {
      title: "Receive beta updates",
      description: "Get beta versions as soon as they are out. Off, only stable versions arrive.",
      control: () => switchControl("Receive beta updates", state.settings.betaUpdates, (on) => changeSetting("betaUpdates", on)),
    },
    { title: "Documentation", description: "Guides and reference for ANT.", control: () => buttonControl("Open documentation", () => void openOrReport("docs")) },
    { title: "What’s new in this version", control: () => buttonControl("View changelog", () => void openOrReport("changelog")) },
    {
      section: "Report a bug",
      title: `On GitHub, or by mail to ${MAIL}`,
      description: "The app version is added for you.",
      control: () => actions(buttonControl("Report on GitHub", () => void openOrReport("issues")), buttonControl("Email us", () => void openOrReport("mail"))),
    },
  ],
};
