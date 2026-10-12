import {
  graphicsSettings,
  resolveAutoDetect,
  type GraphicsTier,
  type TierBenchmarkInput,
} from "./graphicsSettings";

export const GRAPHICS_TARGET = { averageFps: 45, onePercentLowFps: 30 } as const;

export function judgeIntervals(intervalsMs: readonly number[], tier: GraphicsTier): TierBenchmarkInput {
  if (intervalsMs.length < 30 || intervalsMs.some((v) => !Number.isFinite(v) || v <= 0))
    throw new Error("Graphics auto-detect produced insufficient or invalid frame samples.");
  const sorted = [...intervalsMs].sort((a, b) => b - a);
  const lowCount = Math.max(1, Math.ceil(sorted.length * 0.01));
  const averageFps = 1000 / (intervalsMs.reduce((a, b) => a + b, 0) / intervalsMs.length);
  const onePercentLowFps = 1000 / (sorted.slice(0, lowCount).reduce((a, b) => a + b, 0) / lowCount);
  return { averageFps, onePercentLowFps };
}

export function chooseTier(samples: Partial<Record<GraphicsTier, TierBenchmarkInput>>): GraphicsTier {
  const result = resolveAutoDetect({
    high: samples.high ?? null,
    medium: samples.medium ?? null,
    low: samples.low ?? null,
  });
  if ("unusable" in result || !result.mode) {
    throw new Error(
      `Graphics auto-detect could not meet the ${GRAPHICS_TARGET.averageFps}/${GRAPHICS_TARGET.onePercentLowFps} FPS target on any preset. Reload after closing other GPU-heavy applications.`,
    );
  }
  return result.mode;
}

export function runAutoDetect(): Promise<GraphicsTier> {
  if (!navigator.gpu) return Promise.reject(new Error("WebGPU is unavailable. Reload in a WebGPU-capable browser."));
  const sample = (tier: GraphicsTier) =>
    new Promise<TierBenchmarkInput>((resolve, reject) => {
      graphicsSettings.set({ mode: tier });
      setTimeout(() => {
      let last = performance.now();
      const samples: number[] = [];
      const started = last;
      const frame = (now: number) => {
        samples.push(now - last);
        last = now;
        if (now - started < 1500) requestAnimationFrame(frame);
        else {
          try { resolve(judgeIntervals(samples, tier)); } catch (error) { reject(error); }
        }
      };
      requestAnimationFrame(frame);
      }, 250);
    });
  return (async () => {
    const results: Partial<Record<GraphicsTier, TierBenchmarkInput>> = {};
    for (const tier of ["high", "medium", "low"] as const) results[tier] = await sample(tier);
    const selectedMode = chooseTier(results);
    const result = results[selectedMode]!;
    graphicsSettings.set({ mode: "auto", detectedMode: selectedMode, lastResult: { ...result, selectedMode } });
    return selectedMode;
  })();
}
