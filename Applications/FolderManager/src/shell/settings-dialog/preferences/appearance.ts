// Appearance page: theme, accent color, text size and side panel layout.
import { changeSetting } from "../../../actions/ops";
import { quickActionsShown, setQuickActionsShown } from "../../../store/side-layout";
import { state, type AccentId, type TextSize, type Theme } from "../../../store/state";
import { ACCENTS } from "../../../ui/appearance";
import { dotsControl, segControl, selectControl, switchControl, type SettingsPage } from "../page";
import { railEditor } from "./rail-editor";

export const appearancePage: SettingsPage = {
  id: "appearance",
  group: "Preferences",
  title: "Appearance",
  icon: "pageAppearance",
  rows: () => [
    {
      title: "Theme",
      control: () =>
        segControl<Theme>(
          "Theme",
          [
            ["system", "System"],
            ["light", "Light"],
            ["dark", "Dark"],
          ],
          state.settings.theme,
          (value) => void changeSetting("theme", value),
        ),
    },
    {
      title: "Accent color",
      description: "Used for buttons, titles and active states.",
      control: () => dotsControl<AccentId>("Accent color", ACCENTS, state.settings.accent, (value) => void changeSetting("accent", value)),
    },
    {
      title: "Text size",
      control: () =>
        selectControl<TextSize>(
          "Text size",
          [
            ["small", "Small"],
            ["medium", "Medium"],
            ["large", "Large"],
          ],
          state.settings.textSize,
          (value) => void changeSetting("textSize", value),
        ),
    },
    {
      title: "Quick actions",
      description: "Show Open and favorite in the expanded side panel.",
      control: () =>
        switchControl("Show quick actions in the side panel", quickActionsShown(), (on) => {
          setQuickActionsShown(on);
          return Promise.resolve(true);
        }),
    },
    {
      title: "Collapsed side panel",
      description: "Choose which buttons the collapsed rail shows and their order.",
      control: railEditor,
    },
  ],
};
