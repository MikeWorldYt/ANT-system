// General preferences: how lists behave, what the app opens on and when it asks before deleting.
import { changeSetting } from "../../../actions/ops";
import { state, type OnLaunch, type StartView } from "../../../store/state";
import { selectControl, switchControl, type SettingsPage } from "../page";

export const generalPage: SettingsPage = {
  id: "general",
  group: "Preferences",
  title: "General",
  icon: "pageGeneral",
  rows: () => [
    {
      section: "Preferences",
      title: "Pin favorites to the top of each list",
      control: () => switchControl("Pin favorites to the top of each list", state.settings.pinFavorites, (on) => changeSetting("pinFavorites", on)),
    },
    {
      section: "Preferences",
      title: "Hide Workboards with status done",
      control: () => switchControl("Hide Workboards with status done", state.settings.hideDone, (on) => changeSetting("hideDone", on)),
    },
    {
      section: "Preferences",
      title: "Hide directory identifiers",
      control: () => switchControl("Hide directory identifiers", state.settings.hideDirectoryIdentifiers, (on) => changeSetting("hideDirectoryIdentifiers", on)),
    },
    {
      section: "Preferences",
      title: "Open columns on select",
      description: "In Columns view, selecting a folder opens its contents. Double-click the folder icon or the arrow otherwise.",
      control: () => switchControl("Open columns on select", state.settings.autoOpenColumns, (on) => changeSetting("autoOpenColumns", on)),
    },
    {
      section: "Startup",
      title: "On launch",
      description: "What to show when ANT starts.",
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
      section: "Startup",
      title: "View",
      description: "The view ANT opens with.",
      control: () =>
        selectControl<StartView>(
          "View",
          [
            ["continue", "Continue where I left off"],
            ["tree", "Tree"],
            ["columns", "Columns"],
          ],
          state.settings.startView,
          (value) => void changeSetting("startView", value),
        ),
    },
    {
      section: "Startup",
      title: "Confirm before deleting",
      description: "Ask for confirmation when deleting Workboards or folders.",
      control: () => switchControl("Confirm before deleting", state.settings.confirmDelete, (on) => changeSetting("confirmDelete", on)),
    },
  ],
};
