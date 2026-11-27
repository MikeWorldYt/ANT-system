// Session cache of folder totals the backend walks in the background; a folder has no size in its listing.
const sizes = new Map<string, number>();
// Keys whose cached total was read since the last change; an older total still shows until the new one arrives.
const fresh = new Set<string>();
const asked = new Set<string>();

export const folderSizeOf = (key: string): number | null => sizes.get(key) ?? null;

// Folders worth asking for: not yet read and not already being walked.
export function needSizes(keys: string[]): string[] {
  const need = keys.filter((k) => !fresh.has(k) && !asked.has(k));
  need.forEach((k) => asked.add(k));
  return need;
}

export function storeSize(key: string, bytes: number): void {
  sizes.set(key, bytes);
  // A total from a walk started before the last change shows but does not count as read.
  if (asked.delete(key)) fresh.add(key);
}

export function forgetAsked(keys: string[]): void {
  keys.forEach((k) => asked.delete(k));
}

export function invalidateSizes(): void {
  fresh.clear();
  asked.clear();
}
