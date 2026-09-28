// Where a vehicle mount's rounds leave: the simulation's muzzle model
// (`weapons::muzzle`), read from a unit type's `mounts` rows. Each mount
// fires from its own muzzle. Its pivot sits in its carrier's frame and turns
// with the carrier: the turret it is `on` (that mount's bearing), or the
// hull (its yaw). Its muzzle turns with its own bearing about the pivot.
// The validator fits the models' mount nodes to it (`fit.vehicle_muzzle`,
// `fit.muzzle_arc`), and the effects and sounds place a hull's shot with it.

import { vec3, type Vec3 } from "math";

/** A `mounts` row, as far as the muzzle model reads it. */
export interface MountRow {
  name: string;
  /** The earlier mount whose turret carries this one; null, the hull. */
  on: string | null;
  /** Where it turns, in its carrier's frame (forward, left, up). */
  pivot_m: readonly number[];
  /** Its muzzle from the pivot along its own bearing; null, a hand weapon. */
  muzzle_m: readonly number[] | null;
}

/** A mount's muzzle model: `on` indexes the type's mount list (null: the hull). */
export interface MountMuzzle {
  name: string;
  on: number | null;
  pivot: Vec3;
  muzzle: Vec3;
}

/** The muzzle models of a type's mount rows, in order; null for a hand weapon. */
export function mountMuzzles(rows: readonly MountRow[]): (MountMuzzle | null)[] {
  return rows.map((row) => {
    if (!row.muzzle_m) return null;
    const on = row.on === null ? -1 : rows.findIndex((c) => c.name === row.on);
    const [px, py, pz] = row.pivot_m;
    const [mx, my, mz] = row.muzzle_m;
    return {
      name: row.name,
      on: on < 0 ? null : on,
      pivot: vec3.fromValues(px, py, pz),
      muzzle: vec3.fromValues(mx, my, mz),
    };
  });
}

const _offset_origin: Vec3 = [0, 0, 0];
const _offset_muzzle = vec3.create();

/** The muzzle's offset from the unit's position, into `out`, with its carrier
 *  pointing along `carried` and the mount along `bearing` (world radians). */
export function muzzleOffset(out: Vec3, m: MountMuzzle, carried: number, bearing: number): Vec3 {
  vec3.rotateZ(_offset_muzzle, m.muzzle, _offset_origin, bearing);
  vec3.rotateZ(out, m.pivot, _offset_origin, carried);
  return vec3.add(out, out, _offset_muzzle);
}
