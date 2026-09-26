// FogStyle: what unseen looks like. Seen pixels are the real world, untouched;
// an unseen pixel is dimmed, desaturated, pulled toward a night tint and
// ruled with fine screen-space lines. The lines are the cue sun shadow never
// has inside fog (spike 02 found dim and cool alone read as shadow); the tint
// is the second, since the sky's fill makes every sun shadow warm. At the
// boundary, a rim on the seen side is the cue a shadow's edge never has
// (slice 15b: a sight wedge leaving a wall read as a second cast shadow), and
// `edge_softness` fades the unseen side in over a few pixels.
//
// Every number is live-tunable (`BattleFrame.setFogStyle`) and lives in the
// fixture's `presentation.fog`: named styles, one selected. `fogLook` is the
// one WGSL function unseen pixels go through, applied by the fog mask pass
// (`fogMaskPass.ts`) from the per-pixel mask the world's materials write.
import { tgpu, d } from "typegpu";

/** One unseen look, as the fixture writes it. */
export interface FogStyle {
  /** Brightness kept, 0–1: the look's last multiplier. */
  dim: number;
  /** 0–1: how far the colour moves to the night tint at its own luminance. */
  cool: number;
  /** The night tint's hue (rgb); rescaled to unit luminance, so it tints
   *  without brightening. */
  tint: [number, number, number];
  /** Saturation kept before cooling: 0 grey, 1 the lit colour's own. */
  saturation: number;
  /** A glow of the night tint added over all unseen, at this HDR luminance:
   *  fog reads as a luminous veil, not a dark mass, and the veil lifts a sun
   *  shadow inside fog toward the fog around it. 0 off. */
  veil: number;
  /** Screen-space lines ruled across unseen pixels. */
  lines: {
    /** Brightness change on a line: 0.3 is 30% brighter, −0.3 darker; 0 off. */
    strength: number;
    /** A line's least brightening, in the night tint at this HDR luminance:
     *  keeps the lines as plain in a shadow as on lit ground, so a shadow
     *  inside fog reads as the same fog, darker, not as a second layer. */
    floor: number;
    /** Distance between lines, device pixels. */
    spacing_px: number;
    /** Line width, device pixels (anti-aliased). */
    width_px: number;
    /** The lines' angle, degrees counter-clockwise from screen horizontal. */
    angle_deg: number;
  };
  /** Device pixels over which the unseen side fades in from the boundary;
   *  0 is a hard edge. Seen pixels never change. */
  edge_softness: number;
  /** A line along the boundary, on the seen side: a sun shadow has none. */
  rim: {
    /** Device pixels; 0 draws none. */
    width_px: number;
    /** Display sRGB, 0–1: drawn after post, so exactly this colour. */
    color: [number, number, number];
    /** Opacity over the lit world, 0–1. */
    alpha: number;
  };
}

/** The widest rim or soft edge a style may ask for, device pixels. */
export const FOG_EDGE_REACH_PX = 8;
/** The mask pass stores distances to the other side up to this many pixels;
 *  its search reaches `ceil(max(rim, softness)) + 1`, always below it. */
export const FOG_DISTANCE_CAP_PX = 16;

/** `presentation.fog`: named unseen looks and the one drawn. */
export interface FogPresentation {
  style: string;
  styles: Record<string, FogStyle>;
}

/** Rec. 709 luminance weights, the same the grade uses. */
const LUMA = [0.2126, 0.7152, 0.0722] as const;

export function validateFogStyle(s: FogStyle, path = "fog style"): FogStyle {
  const within = (name: string, v: number, lo: number, hi: number) => {
    if (!(Number.isFinite(v) && v >= lo && v <= hi))
      throw new Error(`${path}.${name} must be within [${lo}, ${hi}], got ${v}`);
  };
  within("dim", s.dim, 0, 1);
  within("cool", s.cool, 0, 1);
  within("saturation", s.saturation, 0, 1);
  within("veil", s.veil, 0, 1);
  if (!Array.isArray(s.tint) || s.tint.length !== 3)
    throw new Error(`${path}.tint must be [r, g, b]`);
  s.tint.forEach((c, i) => within(`tint[${i}]`, c, 0, 4));
  if (!(LUMA[0] * s.tint[0] + LUMA[1] * s.tint[1] + LUMA[2] * s.tint[2] > 0))
    throw new Error(`${path}.tint must not be black`);
  within("lines.strength", s.lines.strength, -1, 4);
  within("lines.floor", s.lines.floor, 0, 1);
  within("lines.spacing_px", s.lines.spacing_px, 1, 256);
  within("lines.width_px", s.lines.width_px, 0, 256);
  within("lines.angle_deg", s.lines.angle_deg, -360, 360);
  within("edge_softness", s.edge_softness, 0, FOG_EDGE_REACH_PX);
  if (!s.rim) throw new Error(`${path}.rim must be {width_px, color, alpha}`);
  within("rim.width_px", s.rim.width_px, 0, FOG_EDGE_REACH_PX);
  within("rim.alpha", s.rim.alpha, 0, 1);
  if (!Array.isArray(s.rim.color) || s.rim.color.length !== 3)
    throw new Error(`${path}.rim.color must be [r, g, b]`);
  s.rim.color.forEach((c, i) => within(`rim.color[${i}]`, c, 0, 1));
  return s;
}

/** Every named style valid, and the selected one among them. */
export function validateFogPresentation(fog: FogPresentation): FogPresentation {
  const names = Object.keys(fog.styles ?? {});
  if (!names.length) throw new Error("presentation.fog.styles must name at least one style");
  for (const name of names) validateFogStyle(fog.styles[name], `presentation.fog.styles.${name}`);
  if (!names.includes(fog.style))
    throw new Error(`presentation.fog.style "${fog.style}" is not one of [${names.join(", ")}]`);
  return fog;
}

/** The style `presentation.fog` selects. */
export function selectedFogStyle(fog: FogPresentation): FogStyle {
  return fog.styles[fog.style];
}

/** The style as the GPU reads it. */
export const FogStyleUniform = d
  .struct({
    /** The night tint at unit luminance. */
    tint: d.vec3f,
    dim: d.f32,
    cool: d.f32,
    saturation: d.f32,
    veil: d.f32,
    lineStrength: d.f32,
    lineSpacingPx: d.f32,
    lineWidthPx: d.f32,
    /** Radians. */
    lineAngle: d.f32,
    lineFloor: d.f32,
    edgeSoftnessPx: d.f32,
    /** Display sRGB. */
    rimColor: d.vec3f,
    rimAlpha: d.f32,
    rimWidthPx: d.f32,
    /** How far the mask pass searches for the other side, pixels. */
    reachPx: d.f32,
  })
  .$name("FogStyleUniform");

/** The uniform's values for a style. */
export function fogStyleUniform(s: FogStyle) {
  const luma = LUMA[0] * s.tint[0] + LUMA[1] * s.tint[1] + LUMA[2] * s.tint[2];
  return {
    tint: d.vec3f(s.tint[0] / luma, s.tint[1] / luma, s.tint[2] / luma),
    dim: s.dim,
    cool: s.cool,
    saturation: s.saturation,
    veil: s.veil,
    lineStrength: s.lines.strength,
    lineSpacingPx: s.lines.spacing_px,
    lineWidthPx: s.lines.width_px,
    lineAngle: (s.lines.angle_deg * Math.PI) / 180,
    lineFloor: s.lines.floor,
    edgeSoftnessPx: s.edge_softness,
    rimColor: d.vec3f(...s.rim.color),
    rimAlpha: s.rim.alpha,
    rimWidthPx: s.rim.width_px,
    reachPx: Math.ceil(Math.max(s.rim.width_px, s.edge_softness)) + 1,
  };
}

/** An unseen pixel's colour: `lit` (HDR, linear) at framebuffer `pixel`. */
export const fogLook = tgpu.fn(
  [d.vec3f, d.vec2f, FogStyleUniform],
  d.vec3f,
)(/* wgsl */ `(lit: vec3f, pixel: vec2f, s: FogStyleUniform) -> vec3f {
  let lum = dot(lit, vec3f(0.2126, 0.7152, 0.0722));
  let kept = mix(vec3f(lum), lit, s.saturation);
  let c = mix(kept, lum * s.tint, s.cool) * s.dim;
  // Distance to the nearest line, in pixels, across the lines' direction.
  let across = vec2f(sin(s.lineAngle), cos(s.lineAngle));
  let t = dot(pixel, across) / s.lineSpacingPx;
  let dist = abs(fract(t + 0.5) - 0.5) * s.lineSpacingPx;
  let cover = clamp(s.lineWidthPx * 0.5 + 0.5 - dist, 0.0, 1.0);
  return c * (1.0 + s.lineStrength * cover) + s.tint * (s.veil + s.lineFloor * cover);
}`);
