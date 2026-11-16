/** One epoch-based monotonic clock for batch identities and playout. */
export interface RemoteClock {
  now(): number;
  schedule(run: () => void, delayMs: number): () => void;
  dispose(): void;
}

/** Worker timers keep releases independent of hidden-tab animation frames and
 *  window timer throttling. Suspension is still possible; queuedMs handles it. */
export function remoteClock(): RemoteClock {
  const worker = new Worker(new URL("./clock.worker.ts", import.meta.url), { type: "module" });
  const waiting = new Map<number, () => void>();
  let nextId = 0;
  worker.onmessage = ({ data }: MessageEvent<number>) => {
    const run = waiting.get(data);
    waiting.delete(data);
    run?.();
  };
  return {
    now: () => performance.timeOrigin + performance.now(),
    schedule(run, delayMs) {
      const id = ++nextId;
      waiting.set(id, run);
      worker.postMessage({ id, delayMs });
      return () => {
        waiting.delete(id);
        worker.postMessage({ id });
      };
    },
    dispose() {
      waiting.clear();
      worker.terminate();
    },
  };
}
