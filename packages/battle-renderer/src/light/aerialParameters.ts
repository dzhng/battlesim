// Adapted from ~/dev/game game-renderer/src/environment/aerialParameters.ts
// (reuse manifest): renderer-independent atmospheric extinction and horizon
// policy. Local change: the curve's numbers come from `presentation.light.haze`,
// with no per-preset defaults.
import type { LightPresentation } from "./sceneLight";
import { BETA_MIE_EXTINCTION, BETA_RAYLEIGH, mieScale } from "./skyParameters";
import type { Vec3 } from "math";

/** Neutral ground-fog extinction (km⁻¹) per (turbidity − onset)²: heavy
 *  weather only; clear turbidities add none. */
const FOG_COEFF_KM = 0.026;
const FOG_TURBIDITY_ONSET = 4.0;
export const HORIZON_SKY_Z = 0.004;
const HORIZON_FADE_START_Z = -0.18;
const HORIZON_FADE_END_Z = 0.06;
const HORIZON_CLEAR_VISIBILITY_KM = 8;
const HORIZON_WIDE_VISIBILITY_KM = 32;
const HORIZON_CLEAR_EXTRA_START_Z = 0.2;
const HORIZON_CLEAR_EXTRA_END_Z = 0.08;

export interface AerialParams {
  /** Per-channel extinction σ (km⁻¹, world kilometres). */
  extinction: Readonly<Vec3>;
  /** Koschmieder meteorological visibility (km) — evidence/test telemetry. */
  visibilityKm: number;
  distanceScale: number;
  clearRadiusKm: number;
  rangeFogNearM: number;
  rangeFogFarM: number;
  rangeFogPower: number;
  rangeFogStrength: number;
  sunMieTint: Readonly<Vec3>;
  sunMieStrength: number;
  sunMiePower: number;
  valleyMistColor: Readonly<Vec3>;
  valleyMistHeightBottomM: number;
  valleyMistHeightTopM: number;
  valleyMistDistanceStartM: number;
  valleyMistDistanceFullM: number;
  valleyMistColorStrength: number;
  valleyMistOpacityBoost: number;
}

/** The pure light → aerial mapping (no GPU). Turbidity drives the physical
 *  extinction; `presentation.light.haze` shapes the curve. */
export function aerialParams(light: LightPresentation): AerialParams {
  const turbidity = light.sky.turbidity;
  const h = light.haze;
  const mie = mieScale(turbidity) * BETA_MIE_EXTINCTION;
  const fog = FOG_COEFF_KM * Math.max(0, turbidity - FOG_TURBIDITY_ONSET) ** 2;
  const extinction = BETA_RAYLEIGH.map((betaR) => (betaR + mie) * h.distance_scale + fog) as [
    number,
    number,
    number,
  ];
  const mean = (extinction[0] + extinction[1] + extinction[2]) / 3;
  return {
    extinction,
    visibilityKm: 3.912 / mean,
    distanceScale: h.distance_scale,
    clearRadiusKm: h.clear_radius_m / 1000,
    rangeFogNearM: h.range_near_m,
    rangeFogFarM: h.range_far_m,
    rangeFogPower: h.range_power,
    rangeFogStrength: h.range_strength,
    sunMieTint: h.sun_tint,
    sunMieStrength: h.sun_strength,
    sunMiePower: h.sun_power,
    valleyMistColor: h.mist_color,
    valleyMistHeightBottomM: h.mist_height_m[0],
    valleyMistHeightTopM: h.mist_height_m[1],
    valleyMistDistanceStartM: h.mist_distance_m[0],
    valleyMistDistanceFullM: h.mist_distance_m[1],
    valleyMistColorStrength: h.mist_color_strength,
    valleyMistOpacityBoost: h.mist_opacity,
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
