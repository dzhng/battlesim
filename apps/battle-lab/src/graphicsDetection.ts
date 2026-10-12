import { graphicsSettings, type GraphicsTier, type TierBenchmarkInput } from "./graphicsSettings";

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
  for (const tier of ["high", "medium", "low"] as const) {
    const sample = samples[tier];
    if (sample && sample.averageFps >= GRAPHICS_TARGET.averageFps && sample.onePercentLowFps >= GRAPHICS_TARGET.onePercentLowFps)
      return tier;
  }
  throw new Error("Graphics auto-detect could not find a usable preset at the 45 FPS target.");
}

export function runAutoDetect(): Promise<GraphicsTier> {
  return new Promise((resolve, reject) => {
    if (!navigator.gpu) return reject(new Error("WebGPU is unavailable. Reload in a WebGPU-capable browser."));
    let last = performance.now();
    const samples: number[] = [];
    const started = last;
    const frame = (now: number) => {
      samples.push(now - last);
      last = now;
      if (now - started < 1500) requestAnimationFrame(frame);
      else {
        try {
          const result = judgeIntervals(samples, "high");
          const tier = chooseTier({ high: result, medium: result, low: result });
          graphicsSettings.set({ mode: "auto", detectedMode: tier, lastResult: { ...result, selectedMode: tier } });
          resolve(tier);
        } catch (error) { reject(error); }
      }
    };
    requestAnimationFrame(frame);
  });
}
