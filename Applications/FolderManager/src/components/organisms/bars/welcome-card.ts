// First-run card in the side panel: says what an ANT Storage is and leads to the tour, which nothing else starts by itself.
import { el } from "../../dom";
import { textButton } from "../../atoms/button";
import { guiIcon, type GuiIcon } from "../../atoms/gui-icons";
import { notify } from "../../../services/store/state";
import { skipTour, startTour } from "../tour/onboarding-tour";

const POINTS: [GuiIcon, string, string][] = [
  ["layout", "Categories", "Your main areas, each on its own"],
  ["viewTree", "Nodes", "Your folders are nodes, now connected"],
  ["statusReview", "Review", "Unsorted files wait here, under control"],
  ["aiReady", "AI-ready", "Connect AI agents via MCP"],
];

function sparkles(): SVGElement {
  const icon = guiIcon("sparkles");
  icon.classList.add("welcome-sparkles");
  icon.setAttribute("aria-hidden", "true");
  return icon;
}

export function welcomeCard(): HTMLElement {
  const tour = textButton("Quick learn", "btn-primary welcome-tour", () => void startTour());
  const skip = textButton("Skip", "welcome-skip", () => {
    skipTour();
    notify();
  });
  return el(
    "div",
    { class: "welcome-card", role: "region", "aria-labelledby": "welcome-title" },
    el("h3", { id: "welcome-title", class: "welcome-title" }, "Welcome to your New ", el("span", { class: "welcome-last" }, "Storage! ", sparkles())),
    el("p", { class: "welcome-lead" }, "ANT keeps your files organized."),
    el("ul", { class: "welcome-points" }, ...POINTS.map(([icon, name, text]) => el("li", {}, el("span", { class: "welcome-icon", "aria-hidden": "true" }, guiIcon(icon)), el("span", {}, el("strong", {}, name), el("span", { class: "welcome-text" }, text))))),
    tour,
    skip,
  );
}
