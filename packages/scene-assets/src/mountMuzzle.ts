// Where a vehicle mount's rounds leave: the simulation's muzzle model
// (`weapons::placed_muzzle`), read from a unit type's `mounts` rows. Each
// mount fires from its own muzzle. Its pivot, in the hull's frame at rest,
// swings with its carrier round the carrier's ring: the turret it is `on`
// (round that turret's pivot, at its bearing), or the hull (round its
// middle, at its yaw). Its muzzle turns with its own bearing about the pivot.
// The validator fits the models' mount nodes to it (`fit.vehicle_muzzle`,
// `fit.muzzle_arc`), and the effects and sounds place a hull's shot with it.

import { vec3, type Vec3 } from "math";

/** A `mounts` row, as far as the muzzle model reads it. */
export interface MountRow {
  id: string;
  /** The earlier mount whose turret carries this one; null, the hull. */
  on: string | null;
  /** Where it turns, in the hull's frame at rest (forward, left, up). */
  pivot_m: readonly number[];
  /** Its muzzle from the pivot along its own bearing; null, a hand weapon. */
  muzzle_m: readonly number[] | null;
}

/** A mount's muzzle model: `on` indexes the type's mount list (null: the hull). */
export interface MountMuzzle {
  id: string;
  on: number | null;
  pivot: Vec3;
  /** The ring its carrier turns round, under the carrier's pivot; the
   *  hull's middle for a hull mount. */
  ring: Vec3;
  muzzle: Vec3;
}

/** The muzzle models of a type's mount rows, in order; null for a hand weapon. */
export function mountMuzzles(rows: readonly MountRow[]): (MountMuzzle | null)[] {
  return rows.map((row) => {
    if (!row.muzzle_m) return null;
    const on = row.on === null ? -1 : rows.findIndex((c) => c.id === row.on);
    const [px, py, pz] = row.pivot_m;
    const [mx, my, mz] = row.muzzle_m;
    const [rx, ry] = on < 0 ? [0, 0] : rows[on].pivot_m;
    return {
      id: row.id,
      on: on < 0 ? null : on,
      pivot: vec3.fromValues(px, py, pz),
      ring: vec3.fromValues(rx, ry, 0),
      muzzle: vec3.fromValues(mx, my, mz),
    };
  });
}

const _offset_origin: Vec3 = [0, 0, 0];
const _offset_muzzle = vec3.create();
const _offset_ring = vec3.create();

/** The muzzle's offset from the unit's position, into `out`, for a hull
 *  heading `yaw` with the mount's carrier pointing along `carried` and the
 *  mount along `bearing` (world radians). */
export function muzzleOffset(
  out: Vec3,
  m: MountMuzzle,
  yaw: number,
  carried: number,
  bearing: number,
): Vec3 {
  vec3.rotateZ(_offset_muzzle, m.muzzle, _offset_origin, bearing);
  vec3.rotateZ(out, m.pivot, m.ring, carried);
  vec3.sub(out, out, m.ring);
  vec3.rotateZ(_offset_ring, m.ring, _offset_origin, yaw);
  vec3.add(out, out, _offset_ring);
  return vec3.add(out, out, _offset_muzzle);
}
