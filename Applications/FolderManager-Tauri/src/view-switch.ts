// Owns which ViewMode is mounted in the tree host and remembers the choice between launches.
import type { ViewMode } from "./view-mode";

const STORAGE_KEY = "fm.view";

export interface ViewSwitcher {
  modes: ViewMode[];
  current(): ViewMode;
  switchTo(id: string): void;
}

function readSaved(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch (e) {
    console.warn("View preference unavailable", e);
    return null;
  }
}

function save(id: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch (e) {
    console.warn("View preference not saved", e);
  }
}

export function createViewSwitcher(host: HTMLElement, modes: ViewMode[]): ViewSwitcher {
  const first = modes[0];
  if (!first) throw new Error("No view modes registered");
  const saved = readSaved();
  let active: ViewMode = modes.find((m) => m.id === saved) ?? first;
  active.mount(host);

  return {
    modes,
    current: () => active,
    switchTo(id) {
      const next = modes.find((m) => m.id === id);
      if (!next || next === active) return;
      active.destroy();
      active = next;
      active.mount(host);
      active.render();
      save(id);
    },
  };
}
