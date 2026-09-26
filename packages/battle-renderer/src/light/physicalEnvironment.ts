// Adapted from ~/dev/game game-renderer/src/environment/physicalEnvironment.ts
// (reuse manifest): renderer-independent mapping from the light to the sun and
// sky-fill terms the environment uniform carries. Local change: reads the
// fixture's `presentation.light`, not a named preset.
import type { LightPresentation } from "./sceneLight";
import { skyModelParams } from "./skyParameters";

type Rgb = [number, number, number];

export interface PhotorealEnvironmentSpec {
  /** Unit vector toward the sun (z-up): `sunDirection(light)`. */
  sunDirection: Rgb;
  /** Sun colour — LINEAR rgb, derived from the sky model's atmospheric
   *  transmittance, never authored. */
  sunColor: Rgb;
  sunIntensity: number;
  /** Linear rgb multiplier on the sky's environment light: the shadow fill. */
  fill: Rgb;
}

export function photorealEnvironment(light: LightPresentation): PhotorealEnvironmentSpec {
  const sky = skyModelParams(light);
  return {
    sunDirection: [...sky.sunDirection],
    sunColor: [...sky.sunLightColor],
    sunIntensity: light.sun_intensity,
    fill: [...light.sky.fill],
  };
}
