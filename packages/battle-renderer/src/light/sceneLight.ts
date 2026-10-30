// The battle's light: `presentation.light` in the fixture, as authored, and
// what the presentation crate resolves it to (`presentation::light`): the
// uniform records of the sky, the haze, the sun and post, which the renderer
// uploads as they are. The battle has one light, and the fixture owns its
// numbers.

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
