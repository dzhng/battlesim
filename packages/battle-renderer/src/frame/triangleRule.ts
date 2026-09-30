// The simulation's ground height inside one grid cell on the GPU: its
// triangle rule, the south-west → north-east diagonal of every cell (the
// `TerrainGrid`). Fog marches and grass seats on it, reading the same
// immutable paged heights and applying this rule within each cell.
import { tgpu, d } from "typegpu";

/** Height at `(u, v)` in [0, 1]² of a cell with corner heights south-west,
 *  south-east, north-west and north-east. */
export const triangleRuleHeight = tgpu.fn(
  [d.f32, d.f32, d.f32, d.f32, d.f32, d.f32],
  d.f32,
)(/* wgsl */ `(h00: f32, h10: f32, h01: f32, h11: f32, u: f32, v: f32) -> f32 {
  if (u >= v) { return h00 + u * (h10 - h00) + v * (h11 - h10); }
  return h00 + v * (h01 - h00) + u * (h11 - h01);
}`);
