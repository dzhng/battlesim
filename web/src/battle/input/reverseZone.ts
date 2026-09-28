/** The implicit reverse zone (battle-look slice 39, Q31): with exactly one
 *  vehicle selected, a right-click in the strip behind its hull is a reverse
 *  move. The strip runs from the rear face up to `reverse_zone_length_m`
 *  back, and is the hull's width plus `reverse_zone_margin_m` each side. */
import { vec2, type Vec2 } from "math";
import village from "@fixtures/village.json";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import type { OwnUnitView } from "../sim/observation";

const { reverse_zone_length_m: LENGTH_M, reverse_zone_margin_m: MARGIN_M } = village.controls;

const _zone_local: Vec2 = [0, 0];
const _zone_origin: Vec2 = [0, 0];

/** Whether a right-click at `point` on the ground reverses `selected`:
 *  exactly one unit, a vehicle, and the point inside the strip behind it.
 *  The rear face itself counts; the far end and the sides are inclusive. */
export function inReverseZone(selected: readonly OwnUnitView[], point: [number, number]): boolean {
  if (selected.length !== 1) return false;
  const unit = selected[0];
  const hull = UNITS.hull(unit.kind)?.half_extents_m;
  if (!hull) return false;
  _zone_local[0] = point[0] - unit.position[0];
  _zone_local[1] = point[1] - unit.position[1];
  // Into the hull's frame: +x forward along its yaw, +y to its left.
  vec2.rotate(_zone_local, _zone_local, _zone_origin, -unit.yaw);
  const behind = -_zone_local[0] - hull[0];
  return behind >= 0 && behind <= LENGTH_M && Math.abs(_zone_local[1]) <= hull[1] + MARGIN_M;
}
