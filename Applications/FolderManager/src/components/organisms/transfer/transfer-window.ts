// Mounts the transfer panel as the whole of its own see-through window, in the user's accent color.
import "../../../styles/index.css";
import { getSettings } from "../../../services/ipc/api";
import { ACCENTS } from "../../../hooks/appearance";
import { installTransferPanel } from "./transfer-panel";

export async function mountTransferWindow(): Promise<void> {
  const app = document.querySelector<HTMLDivElement>("#app");
  if (!app) throw new Error("Missing #app mount point");
  document.documentElement.classList.add("transfer-window");
  installTransferPanel(app);
  const accent = (await getSettings())["accent"];
  const color = ACCENTS.find((a) => a.value === accent)?.color;
  if (color) document.documentElement.style.setProperty("--accent", color);
}
