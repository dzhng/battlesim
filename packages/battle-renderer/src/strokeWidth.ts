// The one rule for how wide a painted mark's stroke is: routes, marker
// outlines and arrowheads, a soldier's marker, the supply and objective
// rings, the range ruler, the map border. Each is given a width in screen
// pixels, drawn in full at the default camera and closer; as the camera
// pulls out it thins smoothly (in the log of the zoom) to `thin_scale` of
// that at `thin_m_per_px`, so a marker's border doesn't swamp a scene that
// has shrunk under it, and never below `min_px` on screen (legible) nor
// `min_m` on the ground.
import { fade, lerp, remapClamp } from "math";

/** `presentation.overlay.stroke`. Zooms are metres per pixel at the
 *  camera's target (`metresPerPxAt`). */
export interface StrokeRule {
  /** At this zoom and closer, every stroke is its full width. */
  full_m_per_px: number;
  /** At this zoom and farther, every stroke is `thin_scale` of it. */
  thin_m_per_px: number;
  thin_scale: number;
  /** No stroke thins below this on screen (nor below its full width). */
  min_px: number;
  /** Nor below this on the ground. */
  min_m: number;
}

export function validateStrokeRule(s: StrokeRule): StrokeRule {
  const ok =
    s.full_m_per_px > 0 &&
    s.thin_m_per_px > s.full_m_per_px &&
    s.thin_scale > 0 &&
    s.thin_scale <= 1 &&
    s.min_px > 0 &&
    s.min_m > 0;
  if (!ok)
    throw new Error(
      `presentation.overlay.stroke: 0 < full_m_per_px < thin_m_per_px, thin_scale in (0, 1], min_px and min_m > 0; got ${JSON.stringify(s)}`,
    );
  return s;
}

/** A stroke's width in metres on the ground from its full width in pixels. */
export type StrokeWidth = (px: number) => number;

/** The stroke widths where one pixel spans `metresPerPx` at the camera's
 *  target: the one function every mark builder sizes its strokes with. */
export function strokeWidth(rule: StrokeRule, metresPerPx: number): StrokeWidth {
  const t = fade(
    remapClamp(
      Math.log(metresPerPx),
      Math.log(rule.full_m_per_px),
      Math.log(rule.thin_m_per_px),
      0,
      1,
    ),
  );
  const scale = lerp(1, rule.thin_scale, t);
  return (px) =>
    Math.max(rule.min_m, Math.max(Math.min(px, rule.min_px), px * scale) * metresPerPx);
}
