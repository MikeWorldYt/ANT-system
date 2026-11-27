// Contract every tree presentation (tree, columns) implements so the main window can swap them.
import type { TreeNode } from "../../../services/ipc/types";

export interface ViewMode {
  readonly id: string;
  readonly label: string;
  mount(host: HTMLElement): void;
  render(): void;
  // Views that show one folder's contents return it; null is the Storage root.
  openFolder?(): TreeNode | null;
  focus(): void;
  destroy(): void;
}
