// The range ruler's ground paint (Space held with a selection): a line from
// the border of the circle drawn round the measured unit to the border of a
// ring at the cursor's ground point, lit where one of the unit's
// weapons still reaches and dimmed past the last reach, a tick across it
// where each reach ends short of the cursor. It is
// paint on the ground like the orders, sized on screen (its strokes by the
// one stroke rule, `strokeWidth.ts`), and rebuilt as
// the pointer moves (`BattleFrame.setPointerMarks`). The labels are the HUD's
// (`web/src/battle/present/rangeRuler.ts` measures; the lab places them).
import { groundRing, groundStrip, isRgba, MeshBuilder, type Mesh } from "./mesh";
import type { Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { StrokeWidth } from "./strokeWidth";

/** `presentation.overlay.ruler`. Sizes are on screen at the camera's target;
 *  the strokes (`line_px`, `tick_line_px`) thin with the zoom, as every
 *  mark's do. */
export interface RulerStyle {
  line_px: number;
  /** A tick's length across the line, and its width. */
  tick_px: number;
  tick_line_px: number;
  /** The ring at the cursor: its radius. */
  end_px: number;
  /** The line where a weapon of the unit reaches. */
  reach: Rgba;
  /** The line past its last reach. */
  beyond: Rgba;
  /** Where a weapon stops reaching. */
  tick: Rgba;
}

export function validateRulerStyle(s: RulerStyle): RulerStyle {
  const ok =
    s.line_px > 0 &&
    s.tick_px > 0 &&
    s.tick_line_px > 0 &&
    s.end_px > 0 &&
    isRgba(s.reach) &&
    isRgba(s.beyond) &&
    isRgba(s.tick);
  if (!ok)
    throw new Error(
      `presentation.overlay.ruler: line_px, tick_px, tick_line_px, end_px > 0, rgba reach, beyond and tick; got ${JSON.stringify(s)}`,
    );
  return s;
}

/** What the ruler paints, across the ground. */
export interface RulerLine {
  from: readonly [number, number];
  to: readonly [number, number];
  /** Metres from `from` along the line where the paint starts: where the
   *  line leaves the circle drawn round the unit, never piercing it. */
  start_m: number;
  /** Metres from `from` along the line up to which a weapon reaches (the
   *  line's length when one reaches the cursor, or the unit has none). */
  reach_m: number;
  /** Metres from `from` along the line where each reach ends short of `to`. */
  ticks: readonly number[];
}

export function buildRangeRuler(
  line: RulerLine,
  z: SurfaceHeight,
  style: RulerStyle,
  metresPerPx: number,
  stroke: StrokeWidth,
): Mesh {
  const mesh = new MeshBuilder();
  const [ax, ay] = line.from;
  const dx = line.to[0] - ax,
    dy = line.to[1] - ay;
  const length = Math.hypot(dx, dy);
  if (length < 1e-6) return mesh.build();
  const [ux, uy] = [dx / length, dy / length];
  const at = (along: number, across = 0) =>
    [ax + ux * along - uy * across, ay + uy * along + ux * across] as const;
  /** The line's paint from `s0` to `s1` along it. */
  const strip = (s0: number, s1: number, color: Rgba) =>
    groundStrip(mesh, at(s0), at(s1), 2 * half, color, { z });
  const half = stroke(style.line_px) / 2;
  const r = style.end_px * metresPerPx;
  // The line runs from the unit's circle to the ring at the cursor, meeting
  // each at its border: the ring's inner edge, so it joins the ring's stroke.
  const [s0, s1] = [Math.max(line.start_m, 0), length - Math.max(0, r - half)];
  const reach = Math.min(Math.max(line.reach_m, 0), length);
  if (Math.min(reach, s1) > s0) strip(s0, Math.min(reach, s1), style.reach);
  if (s1 > Math.max(reach, s0)) strip(Math.max(reach, s0), s1, style.beyond);
  // A tick is a short strip across the line.
  const tickHalf = (style.tick_px * metresPerPx) / 2;
  for (const s of line.ticks)
    if (s > s0 && s < s1)
      groundStrip(mesh, at(s, -tickHalf), at(s, tickHalf), stroke(style.tick_line_px), style.tick, {
        z,
      });
  groundRing(mesh, line.to, r, 2 * half, reach >= length ? style.reach : style.beyond, {
    z,
    segments: 32,
  });
  return mesh.build();
}
