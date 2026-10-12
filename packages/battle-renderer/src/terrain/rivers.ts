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
 *  centreline, one per grid square the water (and a margin) touches
 *  (`Ground.water_cells`: four floats each, the south-west corner, the
 *  surface height and the side, in metres). Cells never overlap, so the
 *  blended surface is drawn once however the river bends. */
export function waterSurfaceMesh(cells: Float32Array): Mesh {
  const mesh = new MeshBuilder();
  for (let o = 0; o < cells.length; o += 4) {
    const [x, y, z, cell] = [cells[o], cells[o + 1], cells[o + 2], cells[o + 3]];
    mesh.quad([x, y, z], [x + cell, y, z], [x + cell, y + cell, z], [x, y + cell, z], WHITE);
  }
  return mesh.build();
}

const WHITE: Rgba = [1, 1, 1, 1];
