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
  /** Internal render resolution multiplier for bounded fill cost. */
  renderScale: number;
  forestDressing: boolean;
  understorey: boolean;
  shadowMapSize: 512 | 1024 | 2048;
  shadowReach: number;
  /** Low keeps the water's depth colour alone (no refraction). */
  waterDetail: "full" | "depth";
  bloom: true;
  castLights: true;
  authoredSunSky: true;
  effectMood: "full";
}

export type TierBenchmarkInput = Omit<GraphicsBenchmarkResult, "selectedMode">;
export type AutoDetectResults = Partial<Record<GraphicsTier, TierBenchmarkInput>>;

/** The RTS frame-rate target auto-detect picks a tier against. */
export const GRAPHICS_TARGET_FPS = 45;

const STORAGE_KEY = "battle.graphics";
const DEFAULTS: GraphicsSettings = { mode: "auto", detectedMode: null, lastResult: null };
export const GRAPHICS_TIERS: readonly GraphicsTier[] = ["low", "medium", "high"];

/** Values from the graphics spike; mood-critical effects intentionally never vary. */
export const graphicsProfiles: Readonly<Record<GraphicsTier, GraphicsQualityProfile>> = {
  low: {
    unitLodScale: 1.6,
    buildingLodScale: 1.6,
    treeLodScale: 1.6,
    grassDensity: 0.55,
    renderScale: 0.75,
    forestDressing: false,
    understorey: false,
    shadowMapSize: 1024,
    shadowReach: 900,
    waterDetail: "depth",
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
    renderScale: 1,
    forestDressing: true,
    understorey: true,
    shadowMapSize: 1024,
    shadowReach: 1600,
    waterDetail: "full",
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
    renderScale: 1,
    forestDressing: true,
    understorey: true,
    shadowMapSize: 2048,
    shadowReach: 2600,
    waterDetail: "full",
    bloom: true,
    castLights: true,
    authoredSunSky: true,
    effectMood: "full",
  },
};

/** Test-only scene profile: cheaper than the player's low tier and never exposed as a player choice. */
export const sceneMinimumProfile: GraphicsQualityProfile = {
  unitLodScale: 2.2,
  buildingLodScale: 2.2,
  treeLodScale: 2.2,
  grassDensity: 0.2,
  renderScale: 0.75,
  forestDressing: false,
  understorey: false,
  shadowMapSize: 512,
  shadowReach: 450,
  waterDetail: "depth",
  bloom: true,
  castLights: true,
  authoredSunSky: true,
  effectMood: "full",
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
  if (
    Object.keys(result).some(
      (key) => !["averageFps", "onePercentLowFps", "selectedMode"].includes(key),
    )
  )
    return false;
  return (
    isFiniteFps(result.averageFps) &&
    isFiniteFps(result.onePercentLowFps) &&
    isTier(result.selectedMode)
  );
}

function isSettings(value: unknown): value is GraphicsSettings {
  if (!value || typeof value !== "object") return false;
  const settings = value as Record<string, unknown>;
  if (Object.keys(settings).some((key) => !["mode", "detectedMode", "lastResult"].includes(key)))
    return false;
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

export function resolveEffectiveTier(
  mode: GraphicsMode,
  detectedMode: GraphicsTier | null,
): GraphicsTier {
  return mode === "auto" ? (detectedMode ?? "high") : mode;
}

/** The best tier that meets the target average. A machine that meets it on
 *  none, or left no evidence, plays on the lowest: detection never turns a
 *  player away. */
export function resolveAutoDetect(results: AutoDetectResults): GraphicsTier {
  let selected = GRAPHICS_TIERS[0];
  for (const mode of GRAPHICS_TIERS) {
    const result = results[mode];
    if (!result || result.averageFps < GRAPHICS_TARGET_FPS) break;
    selected = mode;
  }
  return selected;
}

export const graphicsSettings = {
  get: (): GraphicsSettings => current,
  set(next: Partial<GraphicsSettings>, options: { persist?: boolean } = {}) {
    const updated = { ...current, ...next };
    if (!isSettings(updated)) throw new Error("Invalid graphics settings");
    current = updated;
    if (options.persist !== false) {
      try {
        globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(current));
      } catch {
        // Storage blocked: keep the setting for this page only.
      }
    }
    for (const listener of listeners) listener();
  },
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => void listeners.delete(listener);
  },
};

export const graphicsSettingsStorageKey = STORAGE_KEY;
