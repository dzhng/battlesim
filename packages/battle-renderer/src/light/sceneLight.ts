// The battle's light: `presentation.light` in the fixture, as authored, and
// what the presentation crate resolves it to (`presentation::light`): the
// uniform records of the sky, the haze, the sun and post, which the renderer
// uploads as they are. The battle has one light, and the fixture owns its
// numbers.
import {
  AERIAL_WORDS,
  BLOOM_WORDS,
  GRADE_WORDS,
  SKY_LIGHT_WORDS,
  SUN_CASCADES,
  SUN_WORDS,
} from "../records.gen";

export type Rgb = readonly [number, number, number];

/** Aerial haze: extinction over distance from the camera's ground focus, plus
 *  a range term, a sunward Mie tint and a low valley mist. */
export interface HazeSettings {
  /** Multiplies the physical extinction per world metre. */
  distance_scale: number;
  /** No haze within this distance of the focus. */
  clear_radius_m: number;
  /** Past this orbit distance the haze's distances stretch with the
   *  camera's, so a whole-map overview keeps the far tactical view's haze
   *  instead of whiting the map out. */
  overview_from_m: number;
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
  /** Off leaves only exposure and AgX (default on). */
  enabled?: boolean;
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

/** `light` in `fixtures/presentation.json`. */
export interface LightPresentation extends PostSettings {
  /** Radians counter-clockwise from world +X. */
  sun_azimuth: number;
  /** Radians above the horizon. */
  sun_elevation: number;
  sun_intensity: number;
  /** The share of the sun a sun-shadowed surface still receives: what the
   *  penumbra, the caster's edges and the lit ground around it scatter in,
   *  which neither the shadow map nor the sky-only fill carries. It keeps a
   *  sun shadow a lit, coloured ground in the sun's hue, never near-black, so
   *  shade can't be read as unseen ground. 0 is a black occluder. */
  shadow_floor: number;
  /** `fill` multiplies the sky's environment light, per linear channel: the
   *  shadow fill. A warm fill stands in for light bounced off sunlit ground,
   *  which the sky-only environment lacks, so shade under a blue sky stays
   *  darkened ground instead of turning teal.
   *  `saturation` (default 1) and `tint` (default white) shape the clear sky
   *  before tone mapping; `grade_mix` is how much of the grade the sky takes
   *  (default 1), so a warm grade can sit on the land under a blue sky. */
  sky: {
    turbidity: number;
    radiance: number;
    fill: Rgb;
    saturation?: number;
    tint?: Rgb;
    grade_mix?: number;
    clouds: CloudSettings;
  };
  haze: HazeSettings;
  /** The land past the map edge reaches this far out. Its look is the
   *  biome's (`fixtures/biomes/summer.json`). */
  backdrop: { reach_m: number };
  cascades: CascadeSettings;
}

/** A fair-weather cumulus layer over the view's sky: drawn in the background
 *  only, never in the environment light, so the day stays sunny. */
export interface CloudSettings {
  /** The share of the sky the clouds cover, 0–1. */
  coverage: number;
  /** The layer's height above the eye, metres. */
  height_m: number;
  /** The size of the largest cloud masses, metres. */
  scale_m: number;
  /** How soft a cloud's edge is, in noise units. */
  softness: number;
  opacity: number;
  /** A sunlit cloud top's radiance, in the sky's linear units. */
  radiance: number;
  /** A cloud base's colour against its sunlit top, per linear channel. */
  shade: Rgb;
}

/** The light a frame is built with: the authored settings, and the records
 *  the presentation crate resolved from them (`SkyLight`, `Aerial`, `Sun`,
 *  `Bloom`, `Grade` in `records.gen.ts`), each its uniform's bytes. */
export interface ResolvedLight {
  settings: LightPresentation;
  sky: Float32Array<ArrayBuffer>;
  aerial: Float32Array<ArrayBuffer>;
  sun: Float32Array<ArrayBuffer>;
  bloom: Float32Array<ArrayBuffer>;
  grade: Float32Array<ArrayBuffer>;
}

/** `settings` with its records, split from the crate's words in their order
 *  (`light_records`). */
export function resolvedLight(settings: LightPresentation, words: Float32Array): ResolvedLight {
  const sizes = [SKY_LIGHT_WORDS, AERIAL_WORDS, SUN_WORDS, BLOOM_WORDS, GRADE_WORDS];
  const total = sizes.reduce((a, b) => a + b);
  if (words.length !== total)
    throw new Error(`light records: ${words.length} words, expected ${total}`);
  let at = 0;
  const [sky, aerial, sun, bloom, grade] = sizes.map((n) => words.slice(at, (at += n)));
  return { settings, sky, aerial, sun, bloom, grade };
}

/** Checks every number the GPU will see; throws naming the first bad field. */
export function validateLight(light: LightPresentation): LightPresentation {
  const bad = (path: string, why: string) => {
    throw new Error(`presentation.light.${path}: ${why}`);
  };
  const walk = (value: unknown, path: string) => {
    if (typeof value === "number") {
      if (!Number.isFinite(value)) bad(path, "must be finite");
    } else if (typeof value === "boolean") {
      // A switch, such as `grade.enabled`.
    } else if (Array.isArray(value)) value.forEach((v, i) => walk(v, `${path}[${i}]`));
    else if (value && typeof value === "object")
      for (const [k, v] of Object.entries(value)) walk(v, path ? `${path}.${k}` : k);
    else bad(path, "must be a number or boolean");
  };
  walk(light, "");
  const within = (path: string, v: number, lo: number, hi: number) => {
    if (!(v >= lo && v <= hi)) bad(path, `must be within [${lo}, ${hi}], got ${v}`);
  };
  within("sun_elevation", light.sun_elevation, 0.05, Math.PI / 2);
  within("sun_intensity", light.sun_intensity, 0, 100);
  within("shadow_floor", light.shadow_floor, 0, 0.5);
  within("sky.turbidity", light.sky.turbidity, 1, 12);
  within("sky.radiance", light.sky.radiance, 0, 1000);
  light.sky.fill.forEach((c, i) => within(`sky.fill[${i}]`, c, 0, 4));
  if (light.sky.saturation !== undefined) within("sky.saturation", light.sky.saturation, 0, 3);
  if (light.sky.tint !== undefined)
    light.sky.tint.forEach((c, i) => within(`sky.tint[${i}]`, c, 0, 4));
  if (light.sky.grade_mix !== undefined) within("sky.grade_mix", light.sky.grade_mix, 0, 1);
  const clouds = light.sky.clouds;
  within("sky.clouds.coverage", clouds.coverage, 0, 1);
  within("sky.clouds.height_m", clouds.height_m, 100, 20000);
  within("sky.clouds.scale_m", clouds.scale_m, 10, 1e5);
  within("sky.clouds.softness", clouds.softness, 0.001, 0.5);
  within("sky.clouds.opacity", clouds.opacity, 0, 1);
  within("sky.clouds.radiance", clouds.radiance, 0, 100);
  clouds.shade.forEach((c, i) => within(`sky.clouds.shade[${i}]`, c, 0, 4));
  within("exposure", light.exposure, 0.01, 16);
  within("backdrop.reach_m", light.backdrop.reach_m, 0, 1e6);
  within("haze.overview_from_m", light.haze.overview_from_m, 1, 1e6);
  const c = light.cascades;
  if (c.count !== SUN_CASCADES) bad("cascades.count", `the receiver layout has ${SUN_CASCADES}`);
  if (![512, 1024, 2048, 4096].includes(c.map_size))
    bad("cascades.map_size", "must be 512, 1024, 2048 or 4096");
  within("cascades.max_far_m", c.max_far_m, 10, 20000);
  within("cascades.split_lambda", c.split_lambda, 0, 1);
  within("cascades.softness_m", c.softness_m, 0, 10);
  within("bloom.threshold", light.bloom.threshold, 0, 100);
  return light;
}
