// The batch replay runs on its own time: it pauses while its tab is away and stops at once on Esc.
export interface ReplayClock {
  wait(ms: number): Promise<void>;
  track(a: Animation): Animation;
  pause(): void;
  resume(): void;
  stop(): void;
  readonly stopped: boolean;
}

export function createClock(): ReplayClock {
  let paused = false;
  let stopped = false;
  const running = new Set<Animation>();
  return {
    get stopped() {
      return stopped;
    },
    // Only time spent in view counts, so a hidden tab picks up where it left.
    wait(ms) {
      return new Promise((done) => {
        let left = ms;
        let last = performance.now();
        const tick = (now: number) => {
          if (!paused) left -= now - last;
          last = now;
          if (stopped || left <= 0) done();
          else requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
    },
    track(a) {
      if (stopped) {
        a.cancel();
        return a;
      }
      running.add(a);
      if (paused) a.pause();
      const forget = () => running.delete(a);
      a.finished.then(forget, forget);
      return a;
    },
    pause() {
      paused = true;
      running.forEach((a) => a.pause());
    },
    resume() {
      paused = false;
      running.forEach((a) => a.play());
    },
    stop() {
      stopped = true;
      running.forEach((a) => a.cancel());
    },
  };
}

// An animation that is cancelled resolves too, so a stopped replay never hangs on it.
export const ended = (a: Animation): Promise<void> => a.finished.then(() => undefined, () => undefined);
