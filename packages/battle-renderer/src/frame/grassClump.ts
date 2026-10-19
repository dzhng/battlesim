// One grass clump as the build leaves it for the draw: its GPU record, the
// stride a client allocates the clump buffer by, and the row a probe reads
// back from it.
import { d } from "typegpu";

/** One clump as the build leaves it for the draw. */
export const Clump = d
  .struct({
    root: d.vec3f,
    height: d.f32,
    /** The ground's albedo under it, sqrt-encoded rgb8. */
    colour: d.u32,
    kind: d.u32,
    /** Precomputed yaw sine and cosine, shared by every vertex in the clump. */
    yawSin: d.f32,
    yawCos: d.f32,
    /** Blade width multiplier: far blades are held to a least pixel width. */
    width: d.f32,
  })
  .$name("GrassClump");
export const GRASS_CLUMP_BYTES = 40;

/** One clump as the probe reads it back. */
export interface GrassClumpRow {
  root: [number, number, number];
  height: number;
  kind: string;
  /** Its colour, linear rgb: the ground's under it, dried where it is dry. */
  colour: [number, number, number];
  tier: 0 | 1;
  /** How far tracks or trampling laid it over, 0 upright to 1 flat. */
  laid: number;
}

/** The rows of a clump buffer read back whole: each tier's `count` clumps
 *  from its first slot `start`, their kinds named by `names`. */
export function clumpRows(
  bytes: ArrayBuffer,
  tiers: readonly (readonly [tier: 0 | 1, start: number, count: number])[],
  names: readonly string[],
): GrassClumpRow[] {
  const f = new Float32Array(bytes);
  const u = new Uint32Array(bytes);
  const rows: GrassClumpRow[] = [];
  for (const [tier, start, count] of tiers)
    for (let i = 0; i < count; i++) {
      const o = ((start + i) * GRASS_CLUMP_BYTES) / 4;
      rows.push({
        root: [f[o], f[o + 1], f[o + 2]],
        height: f[o + 3],
        kind: names[u[o + 5]] ?? "?",
        colour: [0, 8, 16].map((shift) => (((u[o + 4] >>> shift) & 255) / 255) ** 2) as [
          number,
          number,
          number,
        ],
        tier,
        laid: 1 - (u[o + 4] >>> 24) / 255,
      });
    }
  return rows;
}
