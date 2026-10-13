// FogStyle: what unseen looks like. Seen pixels are the real world, untouched;
// an unseen pixel is dimmed, desaturated, pulled toward a night tint and
// ruled with fine screen-space lines. The lines are the cue sun shadow never
// has inside fog (spike 02 found dim and cool alone read as shadow); the tint
// is the second, since the sky's fill makes every sun shadow warm. At the
// boundary, a rim on the seen side is the cue a shadow's edge never has
// (without it, a sight wedge leaving a wall read as a second cast shadow), and
// `edge_softness` fades the unseen side in over a few pixels.
//
// Every number is live-tunable (`BattleFrame.setFogLook`) and lives in the
// fixture's `presentation.fog`: named styles, one selected. The presentation
// crate checks a style and resolves it into the uniform (`fog.rs`); `fogLook`
// is the one WGSL function unseen pixels go through, applied by the fog mask
// pass (`fogMaskPass.ts`) from the per-pixel mask the world's materials write.
import { tgpu, d } from "typegpu";
import { FogLook } from "../records.gen";

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

/** The mask pass stores distances to the other side up to this many pixels;
 *  a style's search reaches `ceil(max(rim, softness)) + 1`, always below it
 *  (`FOG_EDGE_REACH_PX`, the presentation crate's `fog.rs`). */
export const FOG_DISTANCE_CAP_PX = 16;

/** `presentation.fog`: named unseen looks and the one drawn. */
export interface FogPresentation {
  style: string;
  styles: Record<string, FogStyle>;
}

/** An unseen pixel's colour: `lit` (HDR, linear) at framebuffer `pixel`. */
export const fogLook = tgpu.fn(
  [d.vec3f, d.vec2f, FogLook],
  d.vec3f,
)(/* wgsl */ `(lit: vec3f, pixel: vec2f, s: FogLook) -> vec3f {
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
