// Appearance page: theme, accent color and text size.
import { changeSetting } from "../../../actions/ops";
import { state, type AccentId, type TextSize, type Theme } from "../../../store/state";
import { ACCENTS } from "../../../ui/appearance";
import { dotsControl, segControl, selectControl, type SettingsPage } from "../page";

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
  ],
};
