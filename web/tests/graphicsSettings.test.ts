import { afterEach, describe, expect, test, vi } from "vitest";

const KEY = "battle.graphics";

async function loadStore(raw?: string) {
  vi.resetModules();
  localStorage.clear();
  if (raw !== undefined) localStorage.setItem(KEY, raw);
  return import("@apps/battle-lab/src/graphicsSettings");
}

afterEach(() => {
  localStorage.clear();
  vi.resetModules();
});

describe("graphics settings persistence", () => {
  test.each([
    [undefined, "auto"],
    ["{}", "auto"],
    ['{"mode":"potato"}', "auto"],
    ['{"mode":"high","detectedMode":null,"lastResult":null,"future":true}', "auto"],
    ['{"mode":"high","detected":"potato"}', "auto"],
    ['{"mode":"high","lastResult":{"averageFps":"fast"}}', "auto"],
  ])("uses first-launch auto for missing or malformed records (%s)", async (raw, mode) => {
    const { graphicsSettings } = await loadStore(raw);
    expect(graphicsSettings.get().mode).toBe(mode);
    expect(graphicsSettings.get().detectedMode).toBeNull();
    expect(graphicsSettings.get().lastResult).toBeNull();
  });

  test("round-trips a valid selection and auto-detect metadata", async () => {
    const { graphicsSettings } = await loadStore();
    const result = {
      averageFps: 51,
      onePercentLowFps: 36,
      selectedMode: "medium" as const,
    };
    graphicsSettings.set({ mode: "medium", detectedMode: "medium", lastResult: result });
    expect(JSON.parse(localStorage.getItem(KEY)!)).toEqual({
      mode: "medium",
      detectedMode: "medium",
      lastResult: result,
    });
    expect(graphicsSettings.get()).toEqual({
      mode: "medium",
      detectedMode: "medium",
      lastResult: result,
    });
  });

  test("notifies subscribers and unsubscribes", async () => {
    const { graphicsSettings } = await loadStore();
    const listener = vi.fn();
    const unsubscribe = graphicsSettings.subscribe(listener);
    graphicsSettings.set({ mode: "low" });
    unsubscribe();
    graphicsSettings.set({ mode: "high" });
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe("auto-detect resolver", () => {
  test("selects the highest tier meeting both thresholds, including edges", async () => {
    const { resolveAutoDetect } = await loadStore();
    expect(
      resolveAutoDetect({
        low: { averageFps: 80, onePercentLowFps: 50 },
        medium: { averageFps: 45, onePercentLowFps: 30 },
        high: { averageFps: 45, onePercentLowFps: 29.99 },
      }),
    ).toEqual({ mode: "medium", result: { averageFps: 45, onePercentLowFps: 30 } });
  });

  test("returns unusable when results are incomplete or non-finite", async () => {
    const { resolveAutoDetect } = await loadStore();
    expect(resolveAutoDetect({ low: { averageFps: 44, onePercentLowFps: 29 }, medium: null, high: null })).toEqual({
      mode: null,
      result: null,
      unusable: true,
    });
  });
});

describe("quality profiles", () => {
  test("profiles preserve mood invariants and lower cost monotonically", async () => {
    const { graphicsProfiles } = await loadStore();
    expect(graphicsProfiles.high.bloom).toBe(true);
    expect(graphicsProfiles.medium.bloom).toBe(true);
    expect(graphicsProfiles.low.bloom).toBe(true);
    expect(graphicsProfiles.high.castLights).toBe(true);
    expect(graphicsProfiles.medium.castLights).toBe(true);
    expect(graphicsProfiles.low.castLights).toBe(true);
    expect(graphicsProfiles.low.grassDensity).toBeGreaterThan(0.5);
    expect(graphicsProfiles.medium.grassDensity).toBeGreaterThan(graphicsProfiles.low.grassDensity);
    expect(graphicsProfiles.high.grassDensity).toBeGreaterThan(graphicsProfiles.medium.grassDensity);
    expect(graphicsProfiles.low.shadowMapSize).toBeLessThan(graphicsProfiles.medium.shadowMapSize);
    expect(graphicsProfiles.medium.shadowMapSize).toBeLessThan(graphicsProfiles.high.shadowMapSize);
    expect(graphicsProfiles.low.shadowReach).toBeLessThan(graphicsProfiles.medium.shadowReach);
    expect(graphicsProfiles.medium.shadowReach).toBeLessThan(graphicsProfiles.high.shadowReach);
  });
});
