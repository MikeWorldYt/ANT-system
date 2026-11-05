// Runs one full refresh after a long idle spell, then waits for new activity so an unattended window stays quiet.
const IDLE_MS = 5 * 60_000;
const INPUTS = ["pointerdown", "pointermove", "keydown", "wheel"];

export function startIdleRefresh(run: () => Promise<boolean>): void {
  let lastInput = Date.now();
  let active = true;
  const onInput = (): void => {
    lastInput = Date.now();
    active = true;
  };
  for (const type of INPUTS) window.addEventListener(type, onInput, { capture: true, passive: true });
  const check = async (): Promise<void> => {
    if (active && Date.now() - lastInput >= IDLE_MS && (await run())) active = false;
    window.setTimeout(() => void check(), IDLE_MS / 5);
  };
  window.setTimeout(() => void check(), IDLE_MS);
}
