import { describe, expect, test, vi } from "vitest";
import {
  judgeIntervals,
  cancelAutoDetect,
  registerGraphicsDetectionRunner,
  runAutoDetect,
  graphicsDetectionState,
} from "@apps/battle-lab/src/graphicsDetection";
import {
  graphicsSettings,
  graphicsSettingsStorageKey,
} from "@apps/battle-lab/src/graphicsSettings";

describe("graphics auto-detect", () => {
  test("computes average and one-percent-low from frame intervals", () => {
    const result = judgeIntervals(Array(100).fill(16.666));
    expect(result.averageFps).toBeCloseTo(60);
    expect(result.onePercentLowFps).toBeCloseTo(60);
  });
  test("accepts a slow but valid frame stream as evidence", () => {
    const result = judgeIntervals(Array(15).fill(66.666));
    expect(result.averageFps).toBeCloseTo(15);
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
      expect(seen).toEqual(["low", "medium", "high"]);
      expect(graphicsDetectionState()).toMatchObject({ status: "complete", tier: "high" });
    } finally {
      dispose();
    }
  });

  test("starts at low and stops after the first tier that misses the target", async () => {
    Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
    const seen: string[] = [];
    const dispose = registerGraphicsDetectionRunner(async (tier) => {
      seen.push(tier);
      if (tier === "high") throw new Error("high should not be tested after medium fails");
      return { averageFps: tier === "medium" ? 44 : 60, onePercentLowFps: 4 };
    });
    try {
      await expect(runAutoDetect()).resolves.toBe("low");
      expect(seen).toEqual(["low", "medium"]);
      expect(graphicsDetectionState()).toMatchObject({
        status: "complete",
        tier: "low",
        completed: 2,
      });
    } finally {
      dispose();
    }
  });

  test("an adapter that misses the target on every preset plays on the lowest", async () => {
    Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
    const dispose = registerGraphicsDetectionRunner(async () => ({
      averageFps: 18,
      onePercentLowFps: 9,
    }));
    try {
      await expect(runAutoDetect()).resolves.toBe("low");
      expect(graphicsDetectionState()).toMatchObject({ status: "complete", tier: "low" });
      expect(graphicsSettings.get()).toMatchObject({
        detectedMode: "low",
        lastResult: { averageFps: 18, selectedMode: "low" },
      });
    } finally {
      dispose();
    }
  });

  test("a measurement that breaks lets the player in on the best proven preset, retried next launch", async () => {
    Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
    localStorage.removeItem(graphicsSettingsStorageKey);
    graphicsSettings.set(
      { mode: "auto", detectedMode: null, lastResult: null },
      { persist: false },
    );
    const dispose = registerGraphicsDetectionRunner(async (tier) => {
      if (tier === "medium") throw new Error("timed out");
      return { averageFps: 60, onePercentLowFps: 40 };
    });
    try {
      await expect(runAutoDetect()).resolves.toBe("low");
      expect(graphicsDetectionState()).toMatchObject({ status: "complete", tier: "low" });
      expect(graphicsSettings.get().detectedMode).toBe("low");
      expect(localStorage.getItem(graphicsSettingsStorageKey)).toBeNull();
    } finally {
      dispose();
    }
  });

  test("detection without a menu scene to measure still completes on the lowest preset", async () => {
    Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
    await expect(runAutoDetect()).resolves.toBe("low");
    expect(graphicsDetectionState()).toMatchObject({ status: "complete", tier: "low" });
  });

  test("does not persist a failed or cancelled canonical run", async () => {
    Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
    let release!: () => void;
    const dispose = registerGraphicsDetectionRunner(
      (tier, signal) =>
        new Promise((resolve, reject) => {
          signal.addEventListener("abort", () =>
            reject(new DOMException("Detection cancelled", "AbortError")),
          );
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
    setTier: (_, signal) => {
      preparationSignal = signal;
      return new Promise(() => {});
    },
    subscribeFrames: () => () => {},
    timeoutMs: 100,
  });
  const pending = run("high", new AbortController().signal);
  const outcome = pending.then(
    () => "resolved",
    (error: Error) => error.message,
  );
  try {
    await vi.advanceTimersByTimeAsync(100);
    expect(await Promise.race([outcome, Promise.resolve("still pending")])).toContain("timed out");
    expect(preparationSignal.aborted).toBe(true);
  } finally {
    vi.useRealTimers();
  }
});

test("cancelled final tier cannot persist even if the adapter resolves late", async () => {
  Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
  let finish!: () => void;
  let reachedLow!: () => void;
  const low = new Promise<void>((resolve) => {
    reachedLow = resolve;
  });
  const dispose = registerGraphicsDetectionRunner(async (tier) => {
    if (tier === "low")
      await new Promise<void>((resolve) => {
        finish = resolve;
        reachedLow();
      });
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
  } finally {
    dispose();
  }
});
