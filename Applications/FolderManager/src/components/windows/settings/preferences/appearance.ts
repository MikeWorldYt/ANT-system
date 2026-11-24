// Appearance page: theme, accent color, text size and side panel layout.
import { changeSetting } from "../../../../hooks/ops";
import { quickActionsShown, setQuickActionsShown } from "../../../../services/store/side-layout";
import { extensionsShown, hiddenItemsShown, setExtensionsShown, setHiddenItemsShown } from "../../../../services/store/view-prefs";
import { relistLevels } from "../../../../hooks/ops";
import { state, type AccentId, type TextSize, type Theme } from "../../../../services/store/state";
import { ACCENTS } from "../../../../hooks/appearance";
import { selectControl } from "../../../atoms/select";
import { switchControl } from "../../../atoms/switch";
import { segControl } from "../../../molecules/choice-group";
import { colorControl } from "../../../molecules/color-choice";
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
      control: () => colorControl<AccentId>("Accent color", ACCENTS, state.settings.accent, (value) => void changeSetting("accent", value)),
    },
    {
      title: "Icon color",
      description: "Used for folder and project icons",
      control: () => colorControl<AccentId>("Icon color", ACCENTS, state.settings.iconColor, (value) => void changeSetting("iconColor", value)),
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
      title: "Hidden items",
      description: "Show hidden, system and dot files, faded",
      control: () =>
        switchControl("Show hidden items", hiddenItemsShown(), async (on) => {
          setHiddenItemsShown(on);
          await relistLevels(null);
          return true;
        }),
    },
    {
      title: "File name extensions",
      description: "Show the extension at the end of file names",
      control: () =>
        switchControl("Show file name extensions", extensionsShown(), (on) => {
          setExtensionsShown(on);
          return Promise.resolve(true);
        }),
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
