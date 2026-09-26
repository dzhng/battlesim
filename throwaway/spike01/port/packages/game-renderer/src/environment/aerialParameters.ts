// Renderer-independent atmospheric extinction and horizon policy.
import type { CivsimEnvironment } from "./environment";
import { BETA_MIE_EXTINCTION, BETA_RAYLEIGH, mieScale } from "./skyParameters";

type Rgb = readonly [number, number, number];
/** Miniature-world amplification: battle maps are ~1–3 km across but read as
 *  many-kilometre vistas, so aerial optical depth runs this many times faster
 *  per world metre than the sky dome's true-scale physics. One constant,
 *  calibrated on the vista shots: the eye parks ~1 km out at the vista rig
 *  stop, so the whole world lives in the 0.5–4 km band — golden must keep the
 *  near field clear (subtle far haze), overcast must swallow the ranges. */
export const AERIAL_DISTANCE_SCALE = 4.5;
/** Neutral ground-fog extinction (km⁻¹) per (turbidity − onset)². Calibrated
 *  so overcast-highland (T 9.8) swallows the ranges before the far-ring
 *  edge while keeping the 1.5–2 km playable field readable. */
const FOG_COEFF_KM = 0.026;
const FOG_TURBIDITY_ONSET = 4.0;
/** Legibility floor (aesthetics rule 2 — battles stay legible): optical depth
 *  starts past the immediate fighting zone around the observer, so heavy
 *  weather never washes the units the player is commanding. */
const CLEAR_RADIUS_KM = 0.14;
export const HORIZON_SKY_Z = 0.004;
const HORIZON_FADE_START_Z = -0.18;
const HORIZON_FADE_END_Z = 0.06;
const HORIZON_CLEAR_VISIBILITY_KM = 8;
const HORIZON_WIDE_VISIBILITY_KM = 32;
const HORIZON_CLEAR_EXTRA_START_Z = 0.2;
const HORIZON_CLEAR_EXTRA_END_Z = 0.08;
const DEFAULT_RANGE_FOG_NEAR_M = CLEAR_RADIUS_KM * 1000;
const DEFAULT_RANGE_FOG_FAR_M = 1700;
const DEFAULT_RANGE_FOG_POWER = 1.28;
const DEFAULT_RANGE_FOG_STRENGTH = 0;
const DEFAULT_SUN_MIE_TINT: Rgb = [1.0, 0.86, 0.62];
const DEFAULT_SUN_MIE_STRENGTH = 0;
const DEFAULT_SUN_MIE_POWER = 3.4;
const DEFAULT_VALLEY_MIST_COLOR: Rgb = [0.84, 0.82, 0.72];
const DEFAULT_VALLEY_MIST_HEIGHT_BOTTOM_M = 8;
const DEFAULT_VALLEY_MIST_HEIGHT_TOP_M = 46;
const DEFAULT_VALLEY_MIST_DISTANCE_START_M = 520;
const DEFAULT_VALLEY_MIST_DISTANCE_FULL_M = 1600;
const DEFAULT_VALLEY_MIST_COLOR_STRENGTH = 0;
const DEFAULT_VALLEY_MIST_OPACITY_BOOST = 0;

export interface AerialParams {
  /** Per-channel extinction σ (km⁻¹, world kilometres). */
  extinction: Rgb;
  /** Koschmieder meteorological visibility (km) — evidence/test telemetry. */
  visibilityKm: number;
  distanceScale: number;
  clearRadiusKm: number;
  rangeFogNearM: number;
  rangeFogFarM: number;
  rangeFogPower: number;
  rangeFogStrength: number;
  sunMieTint: Rgb;
  sunMieStrength: number;
  sunMiePower: number;
  valleyMistColor: Rgb;
  valleyMistHeightBottomM: number;
  valleyMistHeightTopM: number;
  valleyMistDistanceStartM: number;
  valleyMistDistanceFullM: number;
  valleyMistColorStrength: number;
  valleyMistOpacityBoost: number;
}

/** The pure preset → aerial mapping (no GPU). Turbidity drives the physical
 *  extinction; optional physical.aerial fields tune the single scene.fogNode
 *  curve per preset — pinned by web/tests/photorealEnvironment.test.ts. */
export function aerialParams(env: CivsimEnvironment): AerialParams {
  const turbidity = env.physical.turbidity;
  const aerial = env.physical.aerial;
  const distanceScale = aerial?.distanceScale ?? AERIAL_DISTANCE_SCALE;
  const mie = mieScale(turbidity) * BETA_MIE_EXTINCTION;
  const fog = FOG_COEFF_KM * Math.max(0, turbidity - FOG_TURBIDITY_ONSET) ** 2;
  const extinction = BETA_RAYLEIGH.map((betaR) => (betaR + mie) * distanceScale + fog) as [
    number,
    number,
    number,
  ];
  const mean = (extinction[0] + extinction[1] + extinction[2]) / 3;
  return {
    extinction,
    visibilityKm: 3.912 / mean,
    distanceScale,
    clearRadiusKm: (aerial?.clearRadiusM ?? CLEAR_RADIUS_KM * 1000) / 1000,
    rangeFogNearM: aerial?.rangeFogNearM ?? DEFAULT_RANGE_FOG_NEAR_M,
    rangeFogFarM: aerial?.rangeFogFarM ?? DEFAULT_RANGE_FOG_FAR_M,
    rangeFogPower: aerial?.rangeFogPower ?? DEFAULT_RANGE_FOG_POWER,
    rangeFogStrength: aerial?.rangeFogStrength ?? DEFAULT_RANGE_FOG_STRENGTH,
    sunMieTint: aerial?.sunMieTint ?? DEFAULT_SUN_MIE_TINT,
    sunMieStrength: aerial?.sunMieStrength ?? DEFAULT_SUN_MIE_STRENGTH,
    sunMiePower: aerial?.sunMiePower ?? DEFAULT_SUN_MIE_POWER,
    valleyMistColor: aerial?.valleyMistColor ?? DEFAULT_VALLEY_MIST_COLOR,
    valleyMistHeightBottomM: aerial?.valleyMistHeightBottomM ?? DEFAULT_VALLEY_MIST_HEIGHT_BOTTOM_M,
    valleyMistHeightTopM: aerial?.valleyMistHeightTopM ?? DEFAULT_VALLEY_MIST_HEIGHT_TOP_M,
    valleyMistDistanceStartM:
      aerial?.valleyMistDistanceStartM ?? DEFAULT_VALLEY_MIST_DISTANCE_START_M,
    valleyMistDistanceFullM: aerial?.valleyMistDistanceFullM ?? DEFAULT_VALLEY_MIST_DISTANCE_FULL_M,
    valleyMistColorStrength: aerial?.valleyMistColorStrength ?? DEFAULT_VALLEY_MIST_COLOR_STRENGTH,
    valleyMistOpacityBoost: aerial?.valleyMistOpacityBoost ?? DEFAULT_VALLEY_MIST_OPACITY_BOOST,
  };
}

export function aerialHorizonFade(visibilityKm: number) {
  const thinHaze = Math.max(
    0,
    Math.min(
      1,
      (visibilityKm - HORIZON_CLEAR_VISIBILITY_KM) /
        (HORIZON_WIDE_VISIBILITY_KM - HORIZON_CLEAR_VISIBILITY_KM),
    ),
  );
  const horizonFadeStart = HORIZON_FADE_START_Z - HORIZON_CLEAR_EXTRA_START_Z * thinHaze;
  const horizonFadeEnd = HORIZON_FADE_END_Z + HORIZON_CLEAR_EXTRA_END_Z * thinHaze;
  return { horizonFadeStart, horizonFadeEnd };
}
