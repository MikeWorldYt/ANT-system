// Shape every settings page shares; its rows and controls come from components/.
import type { StorageInfo } from "../../../services/ipc/types";
import type { GuiIcon } from "../../atoms/gui-icons";
import type { SettingRow } from "../../molecules/setting-row";

export interface PageContext {
  storage: StorageInfo | null;
  refresh(): void;
}

export interface SettingsPage {
  id: string;
  group: string;
  title: string;
  icon: GuiIcon;
  rows(ctx: PageContext): SettingRow[];
}
