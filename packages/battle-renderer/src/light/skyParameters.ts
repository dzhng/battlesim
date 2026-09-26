// Physical atmosphere data shared by renderer backends. Units, authored tuning,
// and parameter derivation stay here; GPU resources and shader assembly do not.
import { sunDirection as sunToward, type LightPresentation } from "./sceneLight";
import { smoothstep as smoothstepScalar } from "@packages/renderer-core/src/math";

export type Rgb = readonly [number, number, number];

/** Sky-view LUT size (Hillaire uses 192×108; equirect wants 2:1). */
export const SKY_LUT_WIDTH = 384;
export const SKY_LUT_HEIGHT = 192;

// Earth atmosphere constants (Hillaire, EGSR 2020) — km units, z-up world.
export const PLANET_RADIUS_KM = 6360;
export const ATMOSPHERE_TOP_KM = 6460;
export const RAYLEIGH_SCALE_KM = 8.0;
export const MIE_SCALE_KM = 1.2;
/** Rayleigh scattering coefficient at sea level (km⁻¹, rgb). */
export const BETA_RAYLEIGH: Rgb = [5.802e-3, 13.558e-3, 33.1e-3];
/** Mie scattering/extinction at sea level for turbidity 2 (km⁻¹). */
export const BETA_MIE_SCATTER = 3.996e-3;
export const BETA_MIE_EXTINCTION = 4.44e-3;
export const MIE_G = 0.8;
export const EYE_ALTITUDE_KM = 0.2;

/** Uniform multiple-scattering floor (Hillaire's ψms role, one constant),
 *  sky-blue tinted — clear-sky multiple scattering is sky-coloured, which
 *  keeps the low-altitude band from washing to cream. */
export const MS_FLOOR = 0.32;
export const MS_TINT: Rgb = [0.5, 0.7, 1.0];
/** Low-sun dust/aerosol aureole. AgX deliberately compresses chroma in the
 *  display frame, so the sky model must carry enough warm sunward radiance
 *  before tone mapping for golden-hour pixels to remain warm. */
// The aureole preserves a warm-neutral golden band without affecting
// noon/overcast, which the strength curve gates to approximately zero.
export const LOW_SUN_AUREOLE_RADIANCE = 0.55;
export const LOW_SUN_AUREOLE_COS_OUTER = -0.12;
export const LOW_SUN_AUREOLE_COS_INNER = 0.76;
/** Below-horizon ground bounce tint (dry Aegean earth, applied to horizon
 *  radiance in the LUT's lower hemisphere — the IBL's up-welling light). */
export const GROUND_BOUNCE_TINT: Rgb = [0.34, 0.3, 0.25];
/** Overcast dome: high-key near-white grey (David's locked overcast mood:
 *  cool, flat, HIGH-KEY — the sky IS the light source). The gradient runs
 *  BRIGHTER toward the horizon (mist register, matching the reference and
 *  the visual reference), not the darker CIE-standard horizon. */
export const OVERCAST_ZENITH_RADIANCE: Rgb = [1.02, 1.05, 1.1];
/** Sun disc: ~1.2° visual radius (readable at game framing). The radiance is
 *  kept BELOW the ACES saturation knee so the transmittance tint survives —
 *  at 60 the dusk disc blew to pure white (10a critique); at 2.5 dusk reads
 *  a warm gold disc, noon a bright cream one (mood over strict photometry —
 *  a real 14° sun is blinding white, the register wants the evening tint). */
export const SUN_DISC_COS_INNER = 0.99985;
export const SUN_DISC_COS_OUTER = 0.99955;
export const SUN_DISC_RADIANCE = 2.5;

/** How much Mie the preset's turbidity adds (T = 1 → pure Rayleigh air). */
export function mieScale(turbidity: number): number {
  return Math.max(0.05, turbidity - 1);
}

/** Overcastness derived from turbidity — the single physical axis David's
 *  overcast mood hangs on (highland T 9.8 → 1.0; the three clear presets → 0). */
export function overcastFromTurbidity(turbidity: number): number {
  // Saturates by the overcast preset's turbidity so warm physical sky cannot
  // bleed through the cool grey dome.
  // Extinction (the fog runway the compose gate accepted) is driven by
  // turbidity directly and does not move with this ramp.
  return smoothstepScalar(4.0, 7.0, turbidity);
}

export function lowSunAureoleStrength(sunDirectionZ: number, overcast: number): number {
  return (
    // Squared: a mostly-overcast sky
    // must keep only a trace of aureole, or the overcast band reads warm.
    (1 - overcast) ** 2 *
    (1 - smoothstepScalar(0.25, 0.85, sunDirectionZ)) *
    LOW_SUN_AUREOLE_RADIANCE
  );
}

export interface SkyModelParams {
  /** Unit vector toward the sun (z-up). */
  sunDirection: Rgb;
  turbidity: number;
  mieScale: number;
  /** Scales the single-scattered sky (`presentation.light.sky.radiance`):
   *  the sky's brightness against the sunlit ground, for the view and the
   *  environment light alike. */
  radiance: number;
  overcast: number;
  /** Linear rgb transmittance toward the sun (max-channel-normalized) — the
   *  physically derived sun tint (warm low sun, near-white high sun). */
  sunTransmittance: Rgb;
  /** The sun DirectionalLight colour: transmittance desaturated toward grey
   *  as overcast rises (cloud diffusion kills the direct tint). */
  sunLightColor: Rgb;
}

/** The pure light → sky-model mapping (no GPU). Sun elevation + turbidity
 *  drive everything. */
export function skyModelParams(light: LightPresentation): SkyModelParams {
  const sunDirection = sunToward(light);
  const turbidity = light.sky.turbidity;
  const overcast = overcastFromTurbidity(turbidity);
  const sunTransmittance = transmittanceToSun(sunDirection, turbidity);
  const lum =
    0.2126 * sunTransmittance[0] + 0.7152 * sunTransmittance[1] + 0.0722 * sunTransmittance[2];
  const sunLightColor: Rgb = [
    sunTransmittance[0] + (lum - sunTransmittance[0]) * overcast,
    sunTransmittance[1] + (lum - sunTransmittance[1]) * overcast,
    sunTransmittance[2] + (lum - sunTransmittance[2]) * overcast,
  ];
  return {
    sunDirection,
    turbidity,
    mieScale: mieScale(turbidity),
    radiance: light.sky.radiance,
    overcast,
    sunTransmittance,
    sunLightColor,
  };
}

/** Beer–Lambert transmittance from the eye toward `direction` through the
 *  exponential atmosphere (numeric integral, spherical geometry), normalized
 *  to its max channel. Same coefficients as the LUT bake. */
export function transmittanceToSun(direction: Rgb, turbidity: number): Rgb {
  const mieExt = mieScale(turbidity) * BETA_MIE_EXTINCTION;
  const r0 = PLANET_RADIUS_KM + EYE_ALTITUDE_KM;
  const mu = Math.max(direction[2], 0.0);
  const tTop =
    -r0 * mu + Math.sqrt(r0 * r0 * (mu * mu - 1) + ATMOSPHERE_TOP_KM * ATMOSPHERE_TOP_KM);
  const steps = 64;
  const dt = tTop / steps;
  const od = [0, 0, 0];
  for (let i = 0; i < steps; i++) {
    const t = (i + 0.5) * dt;
    const h = Math.sqrt(r0 * r0 + t * t + 2 * r0 * t * mu) - PLANET_RADIUS_KM;
    const rho_r = Math.exp(-h / RAYLEIGH_SCALE_KM);
    const rho_m = Math.exp(-h / MIE_SCALE_KM);
    for (let c = 0; c < 3; c++) od[c] += (BETA_RAYLEIGH[c] * rho_r + mieExt * rho_m) * dt;
  }
  const t3 = od.map((x) => Math.exp(-x));
  const peak = Math.max(t3[0], t3[1], t3[2], 1e-6);
  return [t3[0] / peak, t3[1] / peak, t3[2] / peak];
}
