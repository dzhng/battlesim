// The battle's light: `presentation.light` in the fixture, as authored, and the
// few values derived from it that every light consumer must agree on. The sky,
// the environment light, the aerial haze and the cascades all read the one sun
// direction from here; the camera uniform carries the same two angles.
//
// Rewritten (reuse manifest, technique) from ~/dev/game
// game-renderer/src/environment/environment.ts, whose named presets this
// replaces: the battle has one light, and the fixture owns its numbers.
import { vec3, type Vec3 } from "math";
import { CSM_CASCADES } from "./shadowPolicy";

export type Rgb = readonly [number, number, number];

/** Aerial haze: extinction over distance from the camera's ground focus, plus
 *  a range term, a sunward Mie tint and a low valley mist. */
export interface HazeSettings {
  /** Multiplies the physical extinction per world metre. */
  distance_scale: number;
  /** No haze within this distance of the focus. */
  clear_radius_m: number;
  range_near_m: number;
  range_far_m: number;
  range_power: number;
  range_strength: number;
  /** Linear rgb. */
  sun_tint: Rgb;
  sun_strength: number;
  sun_power: number;
  /** Linear rgb. */
  mist_color: Rgb;
  mist_height_m: readonly [number, number];
  mist_distance_m: readonly [number, number];
  mist_color_strength: number;
  mist_opacity: number;
}

/** Applied to HDR before AgX. Tints multiply linear rgb. */
export interface GradeSettings {
  saturation: number;
  contrast: number;
  split_tone: number;
  shadow_lift: number;
  shadow_tint: Rgb;
  highlight_tint: Rgb;
  lift: Rgb;
}

export interface BloomSettings {
  strength: number;
  radius: number;
  /** Linear HDR luminance where bloom starts, before exposure. */
  threshold: number;
  smooth_width: number;
}

/** Post owns the display transform: exposure, grade, AgX and bloom. */
export interface PostSettings {
  exposure: number;
  grade: GradeSettings;
  bloom: BloomSettings;
}

export interface CascadeSettings {
  /** The receiver layout's slot count; changing it is a shader-layout change. */
  count: number;
  map_size: number;
  /** No sun shadow beyond this view depth. */
  max_far_m: number;
  /** Practical split: 0 uniform, 1 logarithmic. */
  split_lambda: number;
  /** Normal bias per cascade, in that cascade's texels, with a floor. */
  normal_bias_texels: number;
  normal_bias_min_m: number;
  /** Normalized depth bias, scaled by cascade index. */
  depth_bias: number;
  /** Penumbra width in world metres; the PCF radius follows each cascade's texel. */
  softness_m: number;
}

/** `presentation.light` in `fixtures/village.json`. */
export interface LightPresentation extends PostSettings {
  /** Radians counter-clockwise from world +X. */
  sun_azimuth: number;
  /** Radians above the horizon. */
  sun_elevation: number;
  sun_intensity: number;
  /** `fill` multiplies the sky's environment light, per linear channel: the
   *  shadow fill. A warm fill stands in for light bounced off sunlit ground,
   *  which the sky-only environment lacks, so shade under a blue sky stays
   *  darkened ground instead of turning teal. */
  sky: { turbidity: number; radiance: number; fill: Rgb };
  haze: HazeSettings;
  /** The land past the map edge reaches this far out. Its look is the
   *  biome's (`fixtures/biomes/summer.json`). */
  backdrop: { reach_m: number };
  cascades: CascadeSettings;
}

/** Unit vector toward the sun, z up: the one sun direction. */
export function sunDirection(
  light: Pick<LightPresentation, "sun_azimuth" | "sun_elevation">,
): Vec3 {
  // Azimuth about world +Z from +X, elevation above the ground plane. (The
  // package's `spherical` is y-up, so the z-up angles are written out.)
  const cosEl = Math.cos(light.sun_elevation);
  return vec3.fromValues(
    cosEl * Math.cos(light.sun_azimuth),
    cosEl * Math.sin(light.sun_azimuth),
    Math.sin(light.sun_elevation),
  );
}

export function postSettings(light: LightPresentation): PostSettings {
  return { exposure: light.exposure, grade: light.grade, bloom: light.bloom };
}

/** Checks every number the GPU will see; throws naming the first bad field. */
export function validateLight(light: LightPresentation): LightPresentation {
  const bad = (path: string, why: string) => {
    throw new Error(`presentation.light.${path}: ${why}`);
  };
  const walk = (value: unknown, path: string) => {
    if (typeof value === "number") {
      if (!Number.isFinite(value)) bad(path, "must be finite");
    } else if (Array.isArray(value)) value.forEach((v, i) => walk(v, `${path}[${i}]`));
    else if (value && typeof value === "object")
      for (const [k, v] of Object.entries(value)) walk(v, path ? `${path}.${k}` : k);
    else bad(path, "must be a number");
  };
  walk(light, "");
  const within = (path: string, v: number, lo: number, hi: number) => {
    if (!(v >= lo && v <= hi)) bad(path, `must be within [${lo}, ${hi}], got ${v}`);
  };
  within("sun_elevation", light.sun_elevation, 0.05, Math.PI / 2);
  within("sun_intensity", light.sun_intensity, 0, 100);
  within("sky.turbidity", light.sky.turbidity, 1, 12);
  within("sky.radiance", light.sky.radiance, 0, 1000);
  light.sky.fill.forEach((c, i) => within(`sky.fill[${i}]`, c, 0, 4));
  within("exposure", light.exposure, 0.01, 16);
  within("backdrop.reach_m", light.backdrop.reach_m, 0, 1e6);
  const c = light.cascades;
  if (c.count !== CSM_CASCADES) bad("cascades.count", `the receiver layout has ${CSM_CASCADES}`);
  if (![512, 1024, 2048, 4096].includes(c.map_size))
    bad("cascades.map_size", "must be 512, 1024, 2048 or 4096");
  within("cascades.max_far_m", c.max_far_m, 10, 20000);
  within("cascades.split_lambda", c.split_lambda, 0, 1);
  within("cascades.softness_m", c.softness_m, 0, 10);
  within("bloom.threshold", light.bloom.threshold, 0, 100);
  return light;
}
