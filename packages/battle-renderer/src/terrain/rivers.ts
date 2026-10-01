// Rivers as the simulation exports them: each river's rounded centreline in
// stretches, with the water's half width, the bank's grade and the bank's
// height at both ends of each.
// One number decides everything drawn about water: how far a point lies
// inside the water's edge, which is the half width at the closest point of a
// stretch less the distance to it, the deepest over stretches. The simulation
// classifies water and carves the bed by the same number
// (`contract::river::section`), so the drawn edge is the rule's.
import { vec2, type Vec2 } from "math";
import { MeshBuilder, type Mesh, type Rgba } from "../mesh";

/** Floats per stretch: `ax, ay, bx, by, halfA, halfB, gradeA, gradeB, bankA,
 *  bankB, surfaceZ`. The native layout must publish the same fields in this
 *  order. The grade is the fall of bed and bank across the stream; `bank` is
 *  how high the land stands above the water at its edge, so the bank runs
 *  `bank / grade` metres out from the edge. */
export const RIVER_FLOATS = 11;
export const RIVER_FIELDS = [
  "ax",
  "ay",
  "bx",
  "by",
  "halfA",
  "halfB",
  "gradeA",
  "gradeB",
  "bankA",
  "bankB",
  "surfaceZ",
] as const;
/** Where a stretch's fields sit. */
export const RIVER_GRADE = 6;
export const RIVER_BANK = 8;
const RIVER_SURFACE_Z = 10;

/** The water surface is laid as square cells this wide, and reaches this far
 *  past the water's edge: the fragment cuts the edge by distance, feathered
 *  over a pixel, so the cells only have to cover it. */
const WATER_CELL_M = 8;
const WATER_MARGIN_M = 4;

const _a: Vec2 = [0, 0],
  _run: Vec2 = [0, 0],
  _to: Vec2 = [0, 0],
  _point: Vec2 = [0, 0],
  _closest: Vec2 = [0, 0];

/** How far `(x, y)` lies inside the water's edge of the stretch at `offset`
 *  (negative outside): the half width at the stretch's closest point, which
 *  runs linearly between its ends, less the distance to that point. */
export function stretchInside(
  rivers: ArrayLike<number>,
  offset: number,
  x: number,
  y: number,
): number {
  vec2.set(_a, rivers[offset], rivers[offset + 1]);
  vec2.set(_run, rivers[offset + 2] - _a[0], rivers[offset + 3] - _a[1]);
  vec2.set(_point, x, y);
  const length2 = vec2.squaredLength(_run);
  const t =
    length2 > 0
      ? Math.min(1, Math.max(0, vec2.dot(vec2.subtract(_to, _point, _a), _run) / length2))
      : 0;
  vec2.scaleAndAdd(_closest, _a, _run, t);
  const half = rivers[offset + 4] + (rivers[offset + 5] - rivers[offset + 4]) * t;
  return half - vec2.distance(_point, _closest);
}

/** The farthest any bank runs out from its water's edge, in metres. */
export function longestBank(rivers: Float32Array): number {
  let run = 0;
  for (let o = 0; o < rivers.length; o += RIVER_FLOATS)
    for (let end = 0; end < 2; end++)
      run = Math.max(run, rivers[o + RIVER_BANK + end] / rivers[o + RIVER_GRADE + end]);
  return run;
}

/** The water surface: flat cells at each river's surface height along its
 *  centreline, one per grid square the water (and a margin) touches. Cells
 *  never overlap, so the blended surface is drawn once however the river
 *  bends; where rivers meet, the cell is the first river's. */
export function waterSurfaceMesh(rivers: Float32Array): Mesh {
  const mesh = new MeshBuilder();
  const laid = new Set<number>();
  const cell = WATER_CELL_M;
  // A cell is kept when its centre is within the water's reach plus its own
  // half diagonal: every cell the water touches, and a few it does not.
  const slack = WATER_MARGIN_M + cell * Math.SQRT1_2;
  for (let o = 0; o < rivers.length; o += RIVER_FLOATS) {
    const reach = Math.max(rivers[o + 4], rivers[o + 5]) + slack;
    const z = rivers[o + RIVER_SURFACE_Z];
    const i0 = Math.floor((Math.min(rivers[o], rivers[o + 2]) - reach) / cell),
      i1 = Math.floor((Math.max(rivers[o], rivers[o + 2]) + reach) / cell),
      j0 = Math.floor((Math.min(rivers[o + 1], rivers[o + 3]) - reach) / cell),
      j1 = Math.floor((Math.max(rivers[o + 1], rivers[o + 3]) + reach) / cell);
    for (let j = j0; j <= j1; j++)
      for (let i = i0; i <= i1; i++) {
        if (stretchInside(rivers, o, (i + 0.5) * cell, (j + 0.5) * cell) < -slack) continue;
        // Cell indices stay far inside 2^15 on any admitted map (20 km).
        const key = (j + 0x8000) * 0x10000 + (i + 0x8000);
        if (laid.has(key)) continue;
        laid.add(key);
        const x = i * cell,
          y = j * cell;
        mesh.quad([x, y, z], [x + cell, y, z], [x + cell, y + cell, z], [x, y + cell, z], WHITE);
      }
  }
  return mesh.build();
}

const WHITE: Rgba = [1, 1, 1, 1];
