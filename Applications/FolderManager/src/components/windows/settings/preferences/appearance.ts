// Appearance page: theme, accent color, text size and side panel layout.
import { changeSetting } from "../../../../hooks/ops";
import { quickActionsShown, setQuickActionsShown } from "../../../../services/store/side-layout";
import { state, type AccentId, type TextSize, type Theme } from "../../../../services/store/state";
import { ACCENTS } from "../../../../hooks/appearance";
import { selectControl } from "../../../atoms/select";
import { switchControl } from "../../../atoms/switch";
import { dotsControl, segControl } from "../../../molecules/choice-group";
import type { SettingsPage } from "../page";
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
      description: "Used for buttons, titles and active states",
      control: () => dotsControl<AccentId>("Accent color", ACCENTS, state.settings.accent, (value) => void changeSetting("accent", value)),
    },
    {
      title: "Icon color",
      description: "Used for folder and project icons",
      control: () => dotsControl<AccentId>("Icon color", ACCENTS, state.settings.iconColor, (value) => void changeSetting("iconColor", value)),
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
      description: "Show Open and favorite in the expanded side panel",
      control: () =>
        switchControl("Show quick actions in the side panel", quickActionsShown(), (on) => {
          setQuickActionsShown(on);
          return Promise.resolve(true);
        }),
    },
    {
      title: "Side panel",
      description: "Choose which buttons shows, drag for change their order",
      control: railEditor,
    },
  ],
};
