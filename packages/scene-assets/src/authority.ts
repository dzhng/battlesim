// The fit authority, read from a scenario fixture: the simulation's bodies
// (`physics`), the vehicles' mount muzzles (`mounts`), the lowest canopy of its forests and a ruin's height. The CLI, the bake and the
// workbench all read it here, so they judge art against the same numbers.

import type { Vec3 } from "math";
import { mountMuzzles, type MountRow } from "./mountMuzzle.ts";
import type { Authority } from "./schema.ts";

/** The parts of a fixture the authority reads. */
export interface AuthorityFixture {
  physics: {
    soldier_height_m: number;
    infantry_eye_m: number;
    infantry_muzzle_m: number;
    tank_half_extents_m: readonly number[];
    supply_half_extents_m: readonly number[];
    jeep_half_extents_m: readonly number[];
  };
  mounts: { tank: readonly MountRow[]; jeep: readonly MountRow[] };
  map: { forests: readonly { canopy_height_m: number }[] };
  props: { building: { destroyed: { into: { height_m: number } } } };
}

export function fixtureAuthority(fixture: AuthorityFixture): Authority {
  const p = fixture.physics;
  const canopies = fixture.map.forests.map((f) => f.canopy_height_m);
  return {
    soldier_height_m: p.soldier_height_m,
    infantry_eye_m: p.infantry_eye_m,
    infantry_muzzle_m: p.infantry_muzzle_m,
    tank_half_extents_m: [...p.tank_half_extents_m] as Vec3,
    supply_half_extents_m: [...p.supply_half_extents_m] as Vec3,
    jeep_half_extents_m: [...p.jeep_half_extents_m] as Vec3,
    mounts: { tank: mountMuzzles(fixture.mounts.tank), jeep: mountMuzzles(fixture.mounts.jeep) },
    // A map without forests has no canopy to stand in.
    canopy_height_m: canopies.length ? Math.min(...canopies) : Infinity,
    // A collapse's ruin: the building row's destroyed state (34c).
    ruin_height_m: fixture.props.building.destroyed.into.height_m,
  };
}
