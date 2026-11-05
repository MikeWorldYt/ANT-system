// Contract every tree presentation (tree, columns) implements so the main window can swap them.
export interface ViewMode {
  readonly id: string;
  readonly label: string;
  mount(host: HTMLElement): void;
  render(): void;
  focus(): void;
  destroy(): void;
}
