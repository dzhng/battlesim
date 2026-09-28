// The range ruler's ground paint (Space held with a selection): a line from
// the border of the circle drawn round the measured unit to the border of a
// ring at the cursor's ground point, lit where one of the unit's
// weapons still reaches and dimmed past the last reach, a tick across it
// where each reach ends short of the cursor. It is
// paint on the ground like the orders, fixed widths on screen, and rebuilt as
// the pointer moves (`BattleFrame.setPointerMarks`). The labels are the HUD's
// (`web/src/battle/present/rangeRuler.ts` measures; the lab places them).
import { groundAnnulus, isRgba, MeshBuilder, type Mesh, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";

/** `presentation.overlay.ruler`. Widths are on screen at the camera's target. */
export interface RulerStyle {
  line_px: number;
  /** A tick's length across the line, and its width. */
  tick_px: number;
  tick_line_px: number;
  /** The ring at the cursor: its radius. */
  end_px: number;
  /** Never narrower than this on the ground. */
  min_line_m: number;
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
    s.min_line_m > 0 &&
    isRgba(s.reach) &&
    isRgba(s.beyond) &&
    isRgba(s.tick);
  if (!ok)
    throw new Error(
      `presentation.overlay.ruler: line_px, tick_px, tick_line_px, end_px and min_line_m > 0, rgba reach, beyond and tick; got ${JSON.stringify(s)}`,
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

/** Drape step: finer than the terrain grid. */
const DRAPE_STEP_M = 3;

type P2 = readonly [number, number];

export function buildRangeRuler(
  line: RulerLine,
  z: SurfaceHeight,
  style: RulerStyle,
  metresPerPx: number,
): Mesh {
  const mesh = new MeshBuilder();
  const width = (px: number) => Math.max(style.min_line_m, px * metresPerPx);
  const [ax, ay] = line.from;
  const dx = line.to[0] - ax,
    dy = line.to[1] - ay;
  const length = Math.hypot(dx, dy);
  if (length < 1e-6) return mesh.build();
  const [ux, uy] = [dx / length, dy / length];
  const at = (along: number, across: number) => {
    const x = ax + ux * along - uy * across,
      y = ay + uy * along + ux * across;
    return [x, y, z(x, y)] as const;
  };
  /** A strip `half` wide either side of the line from `s0` to `s1` along it. */
  const strip = (s0: number, s1: number, half: number, color: Rgba) => {
    const steps = Math.max(1, Math.ceil((s1 - s0) / DRAPE_STEP_M));
    for (let k = 0; k < steps; k++) {
      const [a, b] = [s0 + ((s1 - s0) * k) / steps, s0 + ((s1 - s0) * (k + 1)) / steps];
      mesh.quad(at(a, -half), at(b, -half), at(b, half), at(a, half), color);
    }
  };
  const half = width(style.line_px) / 2;
  const r = width(style.end_px);
  // The line runs from the unit's circle to the ring at the cursor, meeting
  // each at its border: the ring's inner edge, so it joins the ring's stroke.
  const [s0, s1] = [Math.max(line.start_m, 0), length - Math.max(0, r - half)];
  const reach = Math.min(Math.max(line.reach_m, 0), length);
  if (Math.min(reach, s1) > s0) strip(s0, Math.min(reach, s1), half, style.reach);
  if (s1 > Math.max(reach, s0)) strip(Math.max(reach, s0), s1, half, style.beyond);
  // A tick is a short strip across the line.
  const tickHalf = width(style.tick_px) / 2,
    tickW = width(style.tick_line_px) / 2;
  for (const s of line.ticks) {
    if (!(s > s0 && s < s1)) continue;
    const p = (along: number, across: number) => at(s + along, across);
    mesh.quad(
      p(-tickW, -tickHalf),
      p(tickW, -tickHalf),
      p(tickW, tickHalf),
      p(-tickW, tickHalf),
      style.tick,
    );
  }
  const end: P2 = [line.to[0], line.to[1]];
  groundAnnulus(mesh, end, Math.max(0, r - half), r + half, {
    z,
    segments: 32,
    colorIn: reach >= length ? style.reach : style.beyond,
  });
  return mesh.build();
}
