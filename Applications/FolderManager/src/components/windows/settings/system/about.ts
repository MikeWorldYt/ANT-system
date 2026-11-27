// About page: the app identity header, the update check, the changelog and the ways to report a bug.
import { changeSetting } from "../../../../hooks/ops";
import { checkForUpdates, type UpdateOutcome } from "../../../../hooks/updates";
import { openLink } from "../../../../services/ipc/api";
import { state } from "../../../../services/store/state";
import { reportError } from "../../../organisms/bars/status-line";
import { startTour, tourSeen } from "../../../organisms/tour/onboarding-tour";
import { buttonControl } from "../../../atoms/button";
import { el } from "../../../dom";
import antLogo from "../../../../assets/ant-logo.svg";
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
  intro: (ctx) =>
    el("header", { class: "sw-about-head" }, el("img", { class: "sw-about-logo", src: antLogo, alt: "", width: "40", height: "40" }), el("div", {}, el("strong", { class: "sw-about-name" }, "ANT System Manager"), el("span", { class: "sw-about-version" }, `Version ${ctx.storage?.appVersion ?? "unknown"}`))),
  rows: () => [
    {
      section: "Updates",
      title: "Check for updates",
      description: "Look for a newer version.",
      control: (row: RowHandle) =>
        buttonControl("Check now", () => void checkForUpdates(true).then((outcome) => row.describe(UPDATE_TEXT[outcome])), "outline"),
    },
    {
      section: "Updates",
      title: "Check and install updates automatically",
      description: "At launch, a newer stable version is installed without asking. Betas always ask first.",
      // One switch drives both settings, since checking without installing only nags at every launch.
      control: () =>
        switchControl("Check and install updates automatically", state.settings.autoUpdate && state.settings.installUpdates, (on) =>
          changeSetting("autoUpdate", on).then(() => changeSetting("installUpdates", on)),
        ),
    },
    {
      section: "Updates",
      title: "Receive beta updates",
      description: "Get beta versions as soon as they are out. Off, only stable versions arrive.",
      control: () => switchControl("Receive beta updates", state.settings.betaUpdates, (on) => changeSetting("betaUpdates", on)),
    },
    {
      section: "Help",
      title: "Tutorial",
      description: "Walk through the basic controls again.",
      control: () => {
        const button = buttonControl("Show tutorial", () => {
          button.closest("dialog")?.close();
          void startTour();
        }, "outline");
        button.classList.toggle("tour-pulse", !tourSeen());
        return button;
      },
    },
    { section: "Help", title: "Documentation", description: "Guides and reference for ANT.", control: () => buttonControl("Open documentation", () => void openOrReport("docs")) },
    { section: "Help", title: "What’s new in this version", control: () => buttonControl("View changelog", () => void openOrReport("changelog")) },
    {
      section: "Report a bug",
      title: `On GitHub, or by mail to ${MAIL}`,
      description: "The app version is added for you.",
      control: () => actions(buttonControl("Report on GitHub", () => void openOrReport("issues"), "outline"), buttonControl("Email us", () => void openOrReport("mail"))),
    },
  ],
};
