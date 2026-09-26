// Renderer-independent mapping from the canonical preset to physical lighting.
import type { CivsimEnvironment, CivsimEnvironmentId } from "./environment";
import { skyModelParams } from "./skyParameters";

type Rgb = [number, number, number];

export interface PhotorealEnvironmentSpec {
  id: CivsimEnvironmentId;
  /** Unit vector toward the sun, from the preset azimuth/elevation (z-up). */
  sunDirection: Rgb;
  /** Sun DirectionalLight colour — LINEAR rgb, derived from the sky model's
   *  atmospheric transmittance, never the authored keyColor. */
  sunColor: Rgb;
  sunIntensity: number;
  exposure: number;
  /** Atmospheric turbidity — drives the physical sky + aerial haze. */
  turbidity: number;
  environmentIntensity: number;
}

// Presets without a sky-fill override retain the shared lighting balance.
const ENVIRONMENT_INTENSITY = 0.7;

/** The pure preset → physical-parameters mapping (no GPU, no scene mutation).
 *  Sun intensity / exposure / turbidity — and the sky fill, when a preset
 *  balances its own — come from the preset's physical block
 *  (CivsimPhysicalLight): per-preset knobs on the ONE owner. */
export function photorealEnvironment(env: CivsimEnvironment): PhotorealEnvironmentSpec {
  const cosEl = Math.cos(env.sunElevation);
  const sky = skyModelParams(env);
  return {
    id: env.id,
    sunDirection: [
      cosEl * Math.cos(env.sunAzimuth),
      cosEl * Math.sin(env.sunAzimuth),
      Math.sin(env.sunElevation),
    ],
    sunColor: [...sky.sunLightColor],
    sunIntensity: env.physical.sunIntensity,
    exposure: env.physical.exposure,
    turbidity: env.physical.turbidity,
    environmentIntensity: env.physical.environmentIntensity ?? ENVIRONMENT_INTENSITY,
  };
}
