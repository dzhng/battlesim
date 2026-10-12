import { describe, expect, test, vi } from "vitest";
import { chooseTier, judgeIntervals, cancelAutoDetect, registerGraphicsDetectionRunner, runAutoDetect, graphicsDetectionState } from "@apps/battle-lab/src/graphicsDetection";
import { graphicsSettings } from "@apps/battle-lab/src/graphicsSettings";

describe("graphics auto-detect", () => {
  test("computes average and one-percent-low from frame intervals", () => {
    const result = judgeIntervals(Array(100).fill(16.666), "high");
    expect(result.averageFps).toBeCloseTo(60);
    expect(result.onePercentLowFps).toBeCloseTo(60);
  });
  test("chooses the highest passing tier", () => {
    expect(chooseTier({ high: { averageFps: 44, onePercentLowFps: 40 }, medium: { averageFps: 50, onePercentLowFps: 35 }, low: { averageFps: 60, onePercentLowFps: 50 } })).toBe("medium");
  });
  test("rejects an adapter whose average misses the target on every preset", () => {
    expect(() =>
      chooseTier({
        high: { averageFps: 20, onePercentLowFps: 10 },
        medium: { averageFps: 30, onePercentLowFps: 20 },
        low: { averageFps: 40, onePercentLowFps: 25 },
      }),
    ).toThrow("could not meet the 45 FPS target");
  });
  test("ignores a jagged one-percent low when average FPS meets the target", () => {
    expect(
      chooseTier({
        high: { averageFps: 46, onePercentLowFps: 10 },
        medium: { averageFps: 55, onePercentLowFps: 10 },
        low: { averageFps: 60, onePercentLowFps: 10 },
      }),
    ).toBe("high");
  });
});

describe("graphics detection runner boundary", () => {
  test("runs each canonical tier once, shares concurrent callers, and persists only the selected result", async () => {
    Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
    const seen: string[] = [];
    const dispose = registerGraphicsDetectionRunner(async (tier) => {
      seen.push(tier);
      return { averageFps: tier === "high" ? 46 : 60, onePercentLowFps: 4 };
    });
    try {
      const first = runAutoDetect();
      const second = runAutoDetect();
      expect(await first).toBe("high");
      expect(await second).toBe("high");
      expect(seen).toEqual(["high", "medium", "low"]);
      expect(graphicsDetectionState()).toMatchObject({ status: "complete", tier: "high" });
    } finally {
      dispose();
    }
  });

  test("does not persist a failed or cancelled canonical run", async () => {
    Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
    let release!: () => void;
    const dispose = registerGraphicsDetectionRunner(
      (tier, signal) =>
        new Promise((resolve, reject) => {
          signal.addEventListener("abort", () => reject(new DOMException("Detection cancelled", "AbortError")));
          release = () => resolve({ averageFps: tier === "high" ? 46 : 60, onePercentLowFps: 4 });
        }),
    );
    const before = graphicsSettings.get();
    const promise = runAutoDetect();
    cancelAutoDetect();
    await expect(promise).rejects.toThrow("cancelled");
    expect(graphicsSettings.get()).toEqual(before);
    dispose();
    void release;
  });
});

describe("canonical frame runner", () => {
  test("samples renderer frame notifications after warmup", async () => {
    const listeners = new Set<(timestamp: number) => void>();
    const { createCanonicalFrameRunner } = await import("@apps/battle-lab/src/graphicsDetection");
    const run = createCanonicalFrameRunner({
      setTier: async () => {},
      subscribeFrames: (listener) => {
        listeners.add(listener);
        return () => listeners.delete(listener);
      },
      warmupMs: 10,
      sampleMs: 400,
      timeoutMs: 500,
    });
    const promise = run("high", new AbortController().signal);
    await Promise.resolve();
    for (let t = 0; t <= 450; t += 10) listeners.forEach((listener) => listener(t));
    const result = await promise;
    expect(result.averageFps).toBeCloseTo(100);
  });
});

 test("bounds renderer preparation and aborts its work on timeout", async () => {
    vi.useFakeTimers();
    const { createCanonicalFrameRunner } = await import("@apps/battle-lab/src/graphicsDetection");
    let preparationSignal!: AbortSignal;
    const run = createCanonicalFrameRunner({
      setTier: (_, signal) => { preparationSignal = signal; return new Promise(() => {}); },
      subscribeFrames: () => () => {},
      timeoutMs: 100,
    });
    const pending = run("high", new AbortController().signal);
    const outcome = pending.then(() => "resolved", (error: Error) => error.message);
    try {
      await vi.advanceTimersByTimeAsync(100);
      expect(await Promise.race([outcome, Promise.resolve("still pending")])).toContain("timed out");
      expect(preparationSignal.aborted).toBe(true);
    } finally { vi.useRealTimers(); }
  });

test("cancelled final tier cannot persist even if the adapter resolves late", async () => {
  Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
  let finish!: () => void;
  let reachedLow!: () => void;
  const low = new Promise<void>((resolve) => { reachedLow = resolve; });
  const dispose = registerGraphicsDetectionRunner(async (tier) => {
    if (tier === "low") await new Promise<void>((resolve) => { finish = resolve; reachedLow(); });
    return { averageFps: tier === "high" ? 46 : 60, onePercentLowFps: 2 };
  });
  const before = graphicsSettings.get();
  const pending = runAutoDetect();
  await low;
  cancelAutoDetect();
  finish();
  try {
    await expect(pending).rejects.toThrow("cancelled");
    expect(graphicsSettings.get()).toBe(before);
  } finally { dispose(); }
});
