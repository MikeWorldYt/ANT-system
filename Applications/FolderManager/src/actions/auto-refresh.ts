// Limits the refreshes the app starts on its own (window focus, storage watcher) so the views are not rebuilt while in use.
const INTERVAL_MS = 30_000;

let lastAt = 0;
let timer: number | undefined;

export const markRefreshed = (): void => {
  lastAt = Date.now();
};

// Runs now when the last refresh is old enough; otherwise one trailing run covers every trigger until then.
export async function autoRefresh(run: () => Promise<unknown>): Promise<boolean> {
  if (timer !== undefined) return false;
  const wait = lastAt + INTERVAL_MS - Date.now();
  if (wait > 0) {
    timer = window.setTimeout(() => {
      timer = undefined;
      void autoRefresh(run);
    }, wait);
    return false;
  }
  await run();
  return true;
}
