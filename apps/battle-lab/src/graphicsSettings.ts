/** The player-facing graphics choice. `auto` is resolved to a concrete tier after a benchmark. */
export type GraphicsMode = "auto" | "low" | "medium" | "high";
export type GraphicsTier = Exclude<GraphicsMode, "auto">;

export interface GraphicsBenchmarkResult {
  averageFps: number;
  onePercentLowFps: number;
  selectedMode: GraphicsTier;
}

export interface GraphicsSettings {
  mode: GraphicsMode;
  detectedMode: GraphicsTier | null;
  lastResult: GraphicsBenchmarkResult | null;
}

export interface GraphicsQualityProfile {
  unitLodScale: number;
  buildingLodScale: number;
  treeLodScale: number;
  grassDensity: number;
  forestDressing: boolean;
  understorey: boolean;
  shadowMapSize: 1024 | 1536 | 2048;
  shadowReach: number;
  bloom: true;
  castLights: true;
  authoredSunSky: true;
  effectMood: "full";
}

export type TierBenchmarkInput = Omit<GraphicsBenchmarkResult, "selectedMode">;
export type AutoDetectResults = Record<GraphicsTier, TierBenchmarkInput | null>;
export type AutoDetectResolution =
  | { mode: GraphicsTier; result: TierBenchmarkInput }
  | { mode: null; result: null; unusable: true };

const STORAGE_KEY = "battle.graphics";
const DEFAULTS: GraphicsSettings = { mode: "auto", detectedMode: null, lastResult: null };
const TIERS: readonly GraphicsTier[] = ["low", "medium", "high"];

/** Values from the graphics spike; mood-critical effects intentionally never vary. */
export const graphicsProfiles: Readonly<Record<GraphicsTier, GraphicsQualityProfile>> = {
  low: {
    unitLodScale: 1.6,
    buildingLodScale: 1.6,
    treeLodScale: 1.6,
    grassDensity: 0.55,
    forestDressing: false,
    understorey: false,
    shadowMapSize: 1024,
    shadowReach: 900,
    bloom: true,
    castLights: true,
    authoredSunSky: true,
    effectMood: "full",
  },
  medium: {
    unitLodScale: 1.25,
    buildingLodScale: 1.25,
    treeLodScale: 1.25,
    grassDensity: 0.75,
    forestDressing: true,
    understorey: true,
    shadowMapSize: 1536,
    shadowReach: 1600,
    bloom: true,
    castLights: true,
    authoredSunSky: true,
    effectMood: "full",
  },
  high: {
    unitLodScale: 1,
    buildingLodScale: 1,
    treeLodScale: 1,
    grassDensity: 1,
    forestDressing: true,
    understorey: true,
    shadowMapSize: 2048,
    shadowReach: 2600,
    bloom: true,
    castLights: true,
    authoredSunSky: true,
    effectMood: "full",
  },
};

function isTier(value: unknown): value is GraphicsTier {
  return value === "low" || value === "medium" || value === "high";
}

function isFiniteFps(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isBenchmarkResult(value: unknown): value is GraphicsBenchmarkResult {
  if (!value || typeof value !== "object") return false;
  const result = value as Record<string, unknown>;
  if (Object.keys(result).some((key) => !["averageFps", "onePercentLowFps", "selectedMode"].includes(key))) return false;
  return (
    isFiniteFps(result.averageFps) &&
    isFiniteFps(result.onePercentLowFps) &&
    isTier(result.selectedMode)
  );
}

function isSettings(value: unknown): value is GraphicsSettings {
  if (!value || typeof value !== "object") return false;
  const settings = value as Record<string, unknown>;
  if (Object.keys(settings).some((key) => !["mode", "detectedMode", "lastResult"].includes(key))) return false;
  return (
    (settings.mode === "auto" || isTier(settings.mode)) &&
    (settings.detectedMode === null || isTier(settings.detectedMode)) &&
    (settings.lastResult === null || isBenchmarkResult(settings.lastResult))
  );
}

function load(): GraphicsSettings {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const parsed: unknown = JSON.parse(raw);
    return isSettings(parsed) ? parsed : DEFAULTS;
  } catch {
    return DEFAULTS;
  }
}

let current = load();
const listeners = new Set<() => void>();

export function resolveEffectiveTier(mode: GraphicsMode, detectedMode: GraphicsTier | null): GraphicsTier {
  return mode === "auto" ? detectedMode ?? "high" : mode;
}

/** Pick the best tier that meets the RTS target (45 average, 30 one-percent low). */
export function resolveAutoDetect(results: AutoDetectResults): AutoDetectResolution {
  for (const mode of [...TIERS].reverse()) {
    const result = results[mode];
    if (!result || !isFiniteFps(result.averageFps) || !isFiniteFps(result.onePercentLowFps)) {
      return { mode: null, result: null, unusable: true };
    }
    if (result.averageFps >= 45 && result.onePercentLowFps >= 30) return { mode, result };
  }
  return { mode: null, result: null, unusable: true };
}

export const graphicsSettings = {
  get: (): GraphicsSettings => current,
  set(next: Partial<GraphicsSettings>) {
    const updated = { ...current, ...next };
    if (!isSettings(updated)) throw new Error("Invalid graphics settings");
    current = updated;
    try {
      globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(current));
    } catch {
      // Storage blocked: keep the setting for this page only.
    }
    for (const listener of listeners) listener();
  },
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => void listeners.delete(listener);
  },
};

export const graphicsSettingsStorageKey = STORAGE_KEY;
