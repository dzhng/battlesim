// Adapted from ~/dev/game game-renderer/src/environment/environment.ts (reuse
// manifest): the physical light presets and their types. Local changes: the
// battle preset wrappers, stats and skinned-lighting bridge are dropped. Slice
// 13 moves the battle's values into `presentation.light`.

export type CivsimEnvironmentId = "golden" | "dusk" | "noon" | "overcast-highland";

/** Physical lighting parameterization for the photoreal renderer. The flat
 *  display-referred fields on CivsimEnvironment stay the bespoke WGSL knobs
 *  (campaign/water passes) until 16/17; the photoreal renderer lights the
 *  world from THIS block plus the shared sun/sky colours — mood lives in the
 *  environment, never baked into albedo. */
interface CivsimPhysicalLight {
  /** Sun DirectionalLight intensity (linear radiance units). Overcast is a
   *  weak diffuse key; the bright flat sky (IBL) carries the high-key look. */
  sunIntensity: number;
  /** toneMappingExposure for the preset (AgX tone map — see photoreal world.ts). */
  exposure: number;
  /** Atmospheric turbidity drives the physical sky model. */
  turbidity: number;
  /** IBL (sky) contribution multiplier. Omitted ⇒ the shared default; a preset
   *  sets it only to re-balance direct sun against indirect sky fill. */
  environmentIntensity?: number;
  /** Preset-owned aerial curve knobs for the single scene.fogNode owner. */
  aerial?: CivsimAerialAtmosphere;
}

interface CivsimAerialAtmosphere {
  /** Multiplier for Rayleigh/Mie distance extinction in miniature battle metres. */
  distanceScale?: number;
  /** Clear fighting radius before distance haze accrues. */
  clearRadiusM?: number;
  /** Pen-style dissolve range: starts after this camera-focus distance. */
  rangeFogNearM?: number;
  /** Pen-style dissolve range: reaches its authored runway at this distance. */
  rangeFogFarM?: number;
  rangeFogPower?: number;
  rangeFogStrength?: number;
  /** Directional Mie tint applied sunward, inside the single aerial owner. */
  sunMieTint?: [number, number, number];
  sunMieStrength?: number;
  sunMiePower?: number;
  /** Low, far valley mist pooling. Kept preset-owned, not material-local. */
  valleyMistColor?: [number, number, number];
  valleyMistHeightBottomM?: number;
  valleyMistHeightTopM?: number;
  valleyMistDistanceStartM?: number;
  valleyMistDistanceFullM?: number;
  valleyMistColorStrength?: number;
  valleyMistOpacityBoost?: number;
}

export interface CivsimEnvironment {
  id: CivsimEnvironmentId;
  /** Sun direction (radians) for this mood. */
  sunAzimuth: number;
  sunElevation: number;
  /** Warm/cool key (sun) light, also used for water glint. */
  keyColor: [number, number, number];
  /** Sky fill lifting shadows. */
  fillColor: [number, number, number];
  /** Aerial-perspective haze colour. */
  hazeColor: [number, number, number];
  /** Overall exposure; dusk is dim, not dark-albedo'd. (Bespoke display knob —
   *  the photoreal exposure is physical.exposure.) */
  exposure: number;
  physical: CivsimPhysicalLight;
}

// Toward the DEFAULT battle view: camera3d yaw 0 looks along -X (azimuth pi).
// Authored as pi/2 for the 2.5D camera and stale after the 04a flip — the
// references (battle-coastal-vista) compose with the sun IN view; restored at
// the ladder's 10c with the physical sky (record in slices/10-sky-atmosphere.md).
const SUN_TOWARD_VIEW = Math.PI;

export const CIVSIM_ENVIRONMENTS: Record<CivsimEnvironmentId, CivsimEnvironment> = {
  golden: {
    id: "golden",
    sunAzimuth: SUN_TOWARD_VIEW,
    // A true golden-hour sun (20 deg): the physical sky warms the light and
    // the sun-ward sky by transmittance — "golden's warm sky" (locked mood).
    sunElevation: 0.35,
    keyColor: [1.0, 0.88, 0.66],
    fillColor: [0.42, 0.53, 0.72],
    hazeColor: [0.86, 0.82, 0.68],
    exposure: 1.16,
    physical: {
      // Balance direct sun against sky fill so directional shadows remain
      // distinct while shaded troops keep their indirect illumination.
      sunIntensity: 6.0,
      exposure: 1.06,
      turbidity: 2.9,
      environmentIntensity: 0.35,
      aerial: {
        distanceScale: 4.6,
        clearRadiusM: 70,
        rangeFogNearM: 70,
        rangeFogFarM: 1700,
        rangeFogPower: 1.28,
        rangeFogStrength: 0.34,
        sunMieTint: [1.0, 0.8, 0.52],
        sunMieStrength: 0.34,
        sunMiePower: 3.4,
        valleyMistColor: [0.84, 0.82, 0.72],
        valleyMistHeightBottomM: 8,
        valleyMistHeightTopM: 46,
        // Starts beyond the living-meadow far-grass band (480 m), so mist
        // dissolves the valley vista without concealing grass work.
        valleyMistDistanceStartM: 520,
        valleyMistDistanceFullM: 1600,
        valleyMistColorStrength: 0.18,
        valleyMistOpacityBoost: 0.08,
      },
    },
  },
  dusk: {
    id: "dusk",
    sunAzimuth: SUN_TOWARD_VIEW,
    sunElevation: 0.24,
    keyColor: [1.0, 0.6, 0.34],
    fillColor: [0.34, 0.4, 0.56],
    hazeColor: [0.72, 0.58, 0.5],
    exposure: 0.86,
    physical: { sunIntensity: 2.1, exposure: 0.92, turbidity: 3.6 },
  },
  noon: {
    id: "noon",
    sunAzimuth: SUN_TOWARD_VIEW,
    sunElevation: 1.22,
    keyColor: [1.0, 0.98, 0.94],
    fillColor: [0.56, 0.66, 0.82],
    hazeColor: [0.8, 0.83, 0.86],
    exposure: 1.08,
    physical: { sunIntensity: 3.0, exposure: 1.05, turbidity: 2.0 },
  },
  "overcast-highland": {
    id: "overcast-highland",
    sunAzimuth: SUN_TOWARD_VIEW,
    sunElevation: 0.6,
    keyColor: [0.76, 0.8, 0.84],
    fillColor: [0.78, 0.82, 0.87],
    hazeColor: [0.88, 0.9, 0.92],
    exposure: 1.06,
    // Battle-overcast-highland is flat HIGH-KEY: the near-white sky is the
    // light source, the direct sun is only a low-contrast grey key, and the
    // turbidity supplies the fog runway before the far ring's outer edge.
    physical: {
      sunIntensity: 0.32,
      exposure: 1.24, // 9.8 swallowed the ranges whole - the reference's identity is ranges
      // READING THROUGH haze (compose round-2 verdict). 7.2 keeps the runway
      // (mood scene re-checks saturation) while the walls stay present.
      turbidity: 7.2,
    },
  },
};
