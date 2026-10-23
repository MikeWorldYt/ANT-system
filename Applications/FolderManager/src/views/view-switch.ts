// Owns which ViewMode is mounted in the tree host and remembers the choice between launches.
import { readPref, savePref } from "../store/prefs";
import type { ViewMode } from "./view-mode";

const STORAGE_KEY = "fm.view";

export interface ViewSwitcher {
  modes: ViewMode[];
  current(): ViewMode;
  switchTo(id: string): void;
}

export function createViewSwitcher(host: HTMLElement, modes: ViewMode[]): ViewSwitcher {
  const first = modes[0];
  if (!first) throw new Error("No view modes registered");
  const saved = readPref(STORAGE_KEY);
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
      savePref(STORAGE_KEY, id);
    },
  };
}
