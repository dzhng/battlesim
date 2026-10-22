import {
  GRAPHICS_TARGET_FPS,
  GRAPHICS_TIERS,
  graphicsSettings,
  resolveAutoDetect,
  type AutoDetectResults,
  type GraphicsTier,
  type TierBenchmarkInput,
} from "./graphicsSettings";

// Safari can deliver a valid but slow stream while the menu scene is under
// memory pressure; reject only streams too short to estimate at all.
const MIN_FRAME_SAMPLES = 15;
export function judgeIntervals(intervalsMs: readonly number[]): TierBenchmarkInput {
  if (
    intervalsMs.length < MIN_FRAME_SAMPLES ||
    intervalsMs.some((v) => !Number.isFinite(v) || v <= 0)
  )
    throw new Error("Graphics auto-detect produced insufficient or invalid frame samples.");
  const sorted = [...intervalsMs].sort((a, b) => b - a);
  const lowCount = Math.max(1, Math.ceil(sorted.length * 0.01));
  const averageFps = 1000 / (intervalsMs.reduce((a, b) => a + b, 0) / intervalsMs.length);
  const onePercentLowFps = 1000 / (sorted.slice(0, lowCount).reduce((a, b) => a + b, 0) / lowCount);
  return { averageFps, onePercentLowFps };
}

/** A runner owns the actual canonical menu-reel workload. It must exclude
 * preparation/warm-up and reject incomplete data; this module only owns the
 * policy, serialization and persistence boundary. */
export type GraphicsDetectionRunner = (
  tier: GraphicsTier,
  signal: AbortSignal,
) => Promise<TierBenchmarkInput>;

/** Build a runner from renderer-ready frame notifications. `setTier` must
 * rebuild the canonical reel viewport and resolve only after its first frame;
 * `subscribeFrames` receives presentation timestamps from that renderer. This
 * keeps display callbacks out of the policy module and prevents measuring
 * React/menu scheduling instead of the workload. */
export function createCanonicalFrameRunner({
  setTier,
  subscribeFrames,
  warmupMs = 500,
  sampleMs = 1500,
  timeoutMs = 8_000,
}: {
  setTier: (tier: GraphicsTier, signal: AbortSignal) => Promise<void>;
  subscribeFrames: (listener: (timestampMs: number) => void) => () => void;
  warmupMs?: number;
  sampleMs?: number;
  timeoutMs?: number;
}): GraphicsDetectionRunner {
  return (tier, signal) =>
    new Promise<TierBenchmarkInput>((resolve, reject) => {
      const preparation = new AbortController();
      const intervals: number[] = [];
      let previous = 0;
      let started: number | null = null;
      let done = false;
      let timeout: ReturnType<typeof setTimeout> | undefined;
      let unsubscribe = () => {};
      const abort = () => finish(new DOMException("Detection cancelled", "AbortError"));
      const finish = (error?: unknown) => {
        if (done) return;
        done = true;
        clearTimeout(timeout);
        signal.removeEventListener("abort", abort);
        unsubscribe();
        preparation.abort();
        if (error) reject(error);
        else {
          try {
            resolve(judgeIntervals(intervals));
          } catch (reason) {
            reject(reason);
          }
        }
      };
      if (signal.aborted) return abort();
      signal.addEventListener("abort", abort, { once: true });
      timeout = setTimeout(
        () =>
          finish(
            new Error("Graphics auto-detect timed out before the scene produced enough frames."),
          ),
        timeoutMs,
      );
      const ready = () => {
        if (done) return;
        const stop = subscribeFrames((timestampMs) => {
          if (done) return;
          if (!Number.isFinite(timestampMs))
            return finish(new Error("Graphics auto-detect received an invalid frame timestamp."));
          if (started === null) {
            started = timestampMs;
            previous = timestampMs;
            return;
          }
          const interval = timestampMs - previous;
          if (interval <= 0)
            return finish(
              new Error("Graphics auto-detect received non-increasing frame timestamps."),
            );
          previous = timestampMs;
          const elapsed = timestampMs - started;
          if (elapsed < warmupMs) return;
          intervals.push(interval);
          if (elapsed >= warmupMs + sampleMs) finish();
        });
        unsubscribe = stop;
        // A source may synchronously emit while being subscribed.
        if (done) stop();
      };
      try {
        void setTier(tier, preparation.signal).then(ready).catch(finish);
      } catch (error) {
        finish(error);
      }
    });
}

let runner: GraphicsDetectionRunner | null = null;
let active: { promise: Promise<GraphicsTier>; controller: AbortController } | null = null;
const listeners = new Set<() => void>();
export interface GraphicsDetectionState {
  status: "idle" | "running" | "complete";
  tier: GraphicsTier | null;
  currentTier: GraphicsTier | null;
  completed: number;
}
const IDLE: GraphicsDetectionState = {
  status: "idle",
  tier: null,
  currentTier: null,
  completed: 0,
};
let state = IDLE;

function publish(next: typeof state): void {
  state = next;
  listeners.forEach((listener) => listener());
}

/** Install the canonical reel adapter while the menu/backdrop is mounted. */
export function registerGraphicsDetectionRunner(next: GraphicsDetectionRunner | null): () => void {
  runner = next;
  return () => {
    if (runner === next) runner = null;
  };
}

export function graphicsDetectionState(): typeof state {
  return state;
}

export function subscribeGraphicsDetection(listener: () => void): () => void {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}

/** Run the bounded canonical workload once. Concurrent callers share one run.
 * Every run ends on a tier: a measurement that breaks falls back to the best
 * tier proven so far, for this page only, so the next launch measures again.
 * Only cancellation rejects, and it leaves the settings untouched. */
export function runAutoDetect(): Promise<GraphicsTier> {
  if (active) return active.promise;
  const controller = new AbortController();
  const cancelled = () => {
    if (controller.signal.aborted) throw new DOMException("Detection cancelled", "AbortError");
  };
  publish({ status: "running", tier: null, currentTier: null, completed: 0 });
  const promise = (async () => {
    const results: AutoDetectResults = {};
    let measured = true;
    try {
      if (!navigator.gpu) throw new Error("WebGPU is unavailable.");
      if (!runner) throw new Error("No menu scene is mounted to measure.");
      for (const [index, tier] of GRAPHICS_TIERS.entries()) {
        cancelled();
        publish({ status: "running", tier: null, currentTier: tier, completed: index });
        const result = await runner(tier, controller.signal);
        cancelled();
        results[tier] = result;
        publish({ status: "running", tier: null, currentTier: tier, completed: index + 1 });
        if (result.averageFps < GRAPHICS_TARGET_FPS) break;
      }
    } catch (error) {
      cancelled();
      measured = false;
      console.warn("Graphics auto-detect fell back to the best proven tier:", error);
    }
    const selectedMode = resolveAutoDetect(results);
    const result = results[selectedMode];
    graphicsSettings.set(
      {
        mode: "auto",
        detectedMode: selectedMode,
        lastResult: result ? { ...result, selectedMode } : null,
      },
      { persist: measured },
    );
    publish({
      status: "complete",
      tier: selectedMode,
      currentTier: null,
      completed: Object.keys(results).length,
    });
    return selectedMode;
  })()
    .catch((error: unknown) => {
      publish(IDLE);
      throw error;
    })
    .finally(() => {
      active = null;
    });
  active = { promise, controller };
  return promise;
}

export function cancelAutoDetect(): void {
  active?.controller.abort();
}
