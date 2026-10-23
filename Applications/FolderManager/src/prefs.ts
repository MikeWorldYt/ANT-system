// Per-viewer UI preferences; storage can be blocked, so every access is guarded.
export function readPref(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch (e) {
    console.warn("Preference unavailable", e);
    return null;
  }
}

export function savePref(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch (e) {
    console.warn("Preference not saved", e);
  }
}
