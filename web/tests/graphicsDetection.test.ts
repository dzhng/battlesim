import { describe, expect, test } from "vitest";
import { chooseTier, judgeIntervals } from "@apps/battle-lab/src/graphicsDetection";

describe("graphics auto-detect", () => {
  test("computes average and one-percent-low from frame intervals", () => {
    const result = judgeIntervals(Array(100).fill(16.666), "high");
    expect(result.averageFps).toBeCloseTo(60);
    expect(result.onePercentLowFps).toBeCloseTo(60);
  });
  test("chooses the highest passing tier", () => {
    expect(chooseTier({ high: { averageFps: 44, onePercentLowFps: 40 }, medium: { averageFps: 50, onePercentLowFps: 35 }, low: { averageFps: 60, onePercentLowFps: 50 } })).toBe("medium");
  });
  test("keeps a valid slow adapter playable at Low", () => {
    expect(chooseTier({ high: { averageFps: 20, onePercentLowFps: 10 }, medium: { averageFps: 30, onePercentLowFps: 20 }, low: { averageFps: 40, onePercentLowFps: 25 } })).toBe("low");
  });
});
