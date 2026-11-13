// General preferences: how lists behave, what the app opens on and when it asks before deleting.
import { changeSetting } from "../../../../hooks/ops";
import { state, type OnLaunch, type StartView, type TabView } from "../../../../services/store/state";
import { selectControl } from "../../../atoms/select";
import { VIEW_ICONS, viewChoices } from "../../../molecules/view-choice";
import { switchControl } from "../../../atoms/switch";
import type { SettingsPage } from "../page";

export const generalPage: SettingsPage = {
  id: "general",
  group: "Preferences",
  title: "General",
  icon: "pageGeneral",
  rows: () => [
    {
      section: "Setup",
      title: "On launch",
      description: "What to show when app starts",
      control: () =>
        selectControl<OnLaunch>(
          "On launch",
          [
            ["continue", "Continue where I left off"],
            ["root", "Open Root"],
          ],
          state.settings.onLaunch,
          (value) => void changeSetting("onLaunch", value),
        ),
    },
    {
      section: "Setup",
      title: "Default View",
      description: "View when app opens new windows",
      control: () =>
        viewChoices<StartView>(
          "Default View",
          [
            ["continue", "Last used", "layout"],
            ["tree", "Tree", VIEW_ICONS.tree ?? "layout"],
            ["list", "List", VIEW_ICONS.list ?? "layout"],
            ["details", "Details", VIEW_ICONS.details ?? "layout"],
            ["cards", "Cards", VIEW_ICONS.cards ?? "layout"],
            ["grid", "Grid", VIEW_ICONS.grid ?? "layout"],
            ["columns", "Columns", VIEW_ICONS.columns ?? "layout"],
          ],
          state.settings.startView,
          (value) => void changeSetting("startView", value),
        ),
    },
    {
      section: "Setup",
      title: "View when opening a tab",
      description: "The view a new tab starts with",
      control: () =>
        selectControl<TabView>(
          "View when opening a tab",
          [
            ["last", "Last used"],
            ["default", "Default View"],
          ],
          state.settings.tabView,
          (value) => void changeSetting("tabView", value),
        ),
    },
    {
      section: "Preferences",
      title: "Show directory tracking",
      description: "Visibility of id and trackers (e.g. 01, 02, 03, PJ.30...)",
      control: () => switchControl("Show directory tracking", !state.settings.hideDirectoryIdentifiers, (on) => changeSetting("hideDirectoryIdentifiers", !on)),
    },
    {
      section: "Preferences",
      title: "Show finished Workboards",
      description: "Visibility of Workboards with status done",
      control: () => switchControl("Show finished Workboards", !state.settings.hideDone, (on) => changeSetting("hideDone", !on)),
    },
    {
      section: "Preferences",
      title: "Pin favorites to the top",
      description: "Favorites items to the top of each list",
      control: () => switchControl("Pin favorites to the top", state.settings.pinFavorites, (on) => changeSetting("pinFavorites", on)),
    },
    {
      section: "Preferences",
      title: "Show Favorites folder",
      description: "Lists the favorite folders of the Storage at the top of the root",
      control: () => switchControl("Show Favorites folder", state.settings.showFavorites, (on) => changeSetting("showFavorites", on)),
    },
    {
      section: "Preferences",
      title: "Show Downloads folder",
      description: "Adds system Downloads into the app",
      control: () => switchControl("Show Downloads folder", state.settings.showDownloads, (on) => changeSetting("showDownloads", on)),
    },
    {
      section: "Preferences",
      title: "Show Review entry",
      description: "Lists the review folders of the whole Storage",
      control: () => switchControl("Show Review entry", state.settings.showReview, (on) => changeSetting("showReview", on)),
    },
    {
      section: "Preferences",
      title: "Auto-open columns",
      description: "In Columns view, selecting a folder opens its contents",
      control: () => switchControl("Auto-open columns", state.settings.autoOpenColumns, (on) => changeSetting("autoOpenColumns", on)),
    },
    {
      section: "Preferences",
      title: "Confirm before deleting",
      description: "Ask for confirmation when deleting any node or file",
      control: () => switchControl("Confirm before deleting", state.settings.confirmDelete, (on) => changeSetting("confirmDelete", on)),
    },
  ],
};
