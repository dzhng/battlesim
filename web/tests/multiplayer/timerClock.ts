import type { RemoteClock } from "../../src/battle/remote/clock";

/** Node tests use the test runner's timers; production uses the worker clock. */
export const timerClock = (): RemoteClock => ({
  now: () => Date.now(),
  schedule(run, delay) {
    const timer = setTimeout(run, delay);
    return () => clearTimeout(timer);
  },
  dispose() {},
});
