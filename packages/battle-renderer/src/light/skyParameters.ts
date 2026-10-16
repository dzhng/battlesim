// The physical atmosphere's constants, as the sky's shader text splices them
// (`shaders/physicalSky.ts`). What a battle's light makes of them (the sun's
// tint, the overcast, the aureole's and the haze's strength) is resolved by
// the presentation crate (`presentation::light`), which holds the constants
// its derivations share.

/** Sky-view LUT size (Hillaire uses 192×108; equirect wants 2:1). */
export const SKY_LUT_WIDTH = 384;
export const SKY_LUT_HEIGHT = 192;

// Earth atmosphere constants (Hillaire, EGSR 2020) — km units, z-up world.
export const PLANET_RADIUS_KM = 6360;
export const ATMOSPHERE_TOP_KM = 6460;
export const RAYLEIGH_SCALE_KM = 8.0;
export const MIE_SCALE_KM = 1.2;
/** Rayleigh scattering coefficient at sea level (km⁻¹, rgb). */
export const BETA_RAYLEIGH: readonly [number, number, number] = [5.802e-3, 13.558e-3, 33.1e-3];
export const MIE_G = 0.8;
export const EYE_ALTITUDE_KM = 0.2;

/** Clear-sky multiple scattering is sky-coloured, which keeps the
 *  low-altitude band from washing to cream. Tuned for a brighter, lighter
 *  azure sky (David's ebbtide_sky grade: pale near the horizon, light azure
 *  overhead, without needing a post-process sky gradient overlay). */
export const MS_TINT: readonly [number, number, number] = [0.55, 0.78, 1.15];
/** Where a low sun's warm aureole fades out and is full, as the cosine from
 *  the sun. AgX compresses chroma, so golden-hour pixels need this warmth
 *  before tone mapping. */
export const LOW_SUN_AUREOLE_COS_OUTER = -0.12;
export const LOW_SUN_AUREOLE_COS_INNER = 0.76;
/** Overcast dome: high-key near-white grey (David's locked overcast mood:
 *  cool, flat, HIGH-KEY — the sky IS the light source). The gradient runs
 *  BRIGHTER toward the horizon (mist register, matching the reference and
 *  the visual reference), not the darker CIE-standard horizon.
 *  Clear-sky zenith is azure; overcast blends toward neutral white-blue. */
export const OVERCAST_ZENITH_RADIANCE: readonly [number, number, number] = [1.04, 1.08, 1.14];
/** Sun disc: ~1.2° visual radius (readable at game framing). */
export const SUN_DISC_COS_INNER = 0.99985;
export const SUN_DISC_COS_OUTER = 0.99955;

/** Clear-sky chroma pulled toward luma, plus a gentle warm-white tint.
 *  These baked constants are used at shader build time. To expose them
 *  at runtime, add saturation/tint/grade_mix to the SkyLight uniform
 *  (see `presentation.light.sky` in game.json for the authored values). */
export const SKY_DESATURATE = 0.55;
export const SKY_TINT: readonly [number, number, number] = [1.02, 1.0, 0.97];

/** How much the grade's highlight_tint affects the sky (0 = ungraded sky).
 *  Currently hardcoded at 1.0 (fully graded); expose via SkyLight uniform
 *  when separate sky grading is needed. */
export const SKY_GRADE_MIX = 1.0;
