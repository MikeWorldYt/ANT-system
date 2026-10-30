// Interface icons from assets/gui, inlined so they take the surrounding text color.
import add from "../assets/gui/add.svg?raw";
import back from "../assets/gui/back.svg?raw";
import chevronDown from "../assets/gui/chevron-down.svg?raw";
import chevronRight from "../assets/gui/chevron-right.svg?raw";
import chevronsLeft from "../assets/gui/chevrons-left.svg?raw";
import chevronsRight from "../assets/gui/chevrons-right.svg?raw";
import forward from "../assets/gui/forward.svg?raw";
import infoPanel from "../assets/gui/info-panel.svg?raw";
import layout from "../assets/gui/layout.svg?raw";
import pageAbout from "../assets/gui/page-about.svg?raw";
import pageAppearance from "../assets/gui/page-appearance.svg?raw";
import pageAdvanced from "../assets/gui/page-advanced.svg?raw";
import pageBackup from "../assets/gui/page-backup.svg?raw";
import pageExtensions from "../assets/gui/page-extensions.svg?raw";
import pageGeneral from "../assets/gui/page-general.svg?raw";
import pageStorage from "../assets/gui/page-storage.svg?raw";
import pageShortcuts from "../assets/gui/page-shortcuts.svg?raw";
import pencil from "../assets/gui/pencil.svg?raw";
import moveDown from "../assets/gui/move-down.svg?raw";
import moveUp from "../assets/gui/move-up.svg?raw";
import refresh from "../assets/gui/refresh.svg?raw";
import search from "../assets/gui/search.svg?raw";
import settings from "../assets/gui/settings.svg?raw";
import separator from "../assets/gui/separator.svg?raw";
import starred from "../assets/gui/starred.svg?raw";
import statusDone from "../assets/gui/status-done.svg?raw";
import statusHold from "../assets/gui/status-hold.svg?raw";
import statusProgress from "../assets/gui/status-progress.svg?raw";
import statusReview from "../assets/gui/status-review.svg?raw";
import trash from "../assets/gui/trash.svg?raw";
import up from "../assets/gui/up.svg?raw";
import warning from "../assets/gui/warning.svg?raw";

const SOURCES = { add, back, chevronDown, chevronRight, chevronsLeft, chevronsRight, forward, infoPanel, layout, moveDown, moveUp, pageAbout, pageAdvanced, pageAppearance, pageBackup, pageExtensions, pageGeneral, pageStorage, pageShortcuts, pencil, refresh, search, separator, settings, starred, statusDone, statusHold, statusProgress, statusReview, trash, up, warning };

export type GuiIcon = keyof typeof SOURCES;

export function guiIcon(name: GuiIcon): SVGElement {
  const svg = new DOMParser().parseFromString(SOURCES[name], "image/svg+xml").documentElement;
  // A fixed color attribute would stop the icon from following the button color; the metadata is dead weight.
  svg.removeAttribute("color");
  svg.querySelector("metadata")?.remove();
  svg.setAttribute("class", "gui-icon");
  svg.setAttribute("aria-hidden", "true");
  return document.importNode(svg, true) as unknown as SVGElement;
}
