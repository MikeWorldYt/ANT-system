// Contract every tree presentation (list, columns) implements so the shell can swap them.
export interface ViewMode {
  readonly id: string;
  readonly label: string;
  mount(host: HTMLElement): void;
  render(): void;
  focus(): void;
  destroy(): void;
}
