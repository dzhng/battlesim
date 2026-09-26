// The fit authority, read from a scenario fixture: the simulation's bodies
// (`physics`), the lowest canopy of its forests and a ruin's height. The CLI, the bake and the
// workbench all read it here, so they judge art against the same numbers.

import type { Vec3 } from "math";
import type { Authority } from "./schema.ts";

/** The parts of a fixture the authority reads. */
export interface AuthorityFixture {
  physics: {
    soldier_height_m: number;
    infantry_eye_m: number;
    infantry_muzzle_m: number;
    tank_half_extents_m: readonly number[];
    tank_muzzle_local_m: readonly number[];
    supply_half_extents_m: readonly number[];
  };
  map: { forests: readonly { canopy_height_m: number }[] };
  buildings: { ruin_height_m: number };
}

export function fixtureAuthority(fixture: AuthorityFixture): Authority {
  const p = fixture.physics;
  const canopies = fixture.map.forests.map((f) => f.canopy_height_m);
  return {
    soldier_height_m: p.soldier_height_m,
    infantry_eye_m: p.infantry_eye_m,
    infantry_muzzle_m: p.infantry_muzzle_m,
    tank_half_extents_m: [...p.tank_half_extents_m] as Vec3,
    tank_muzzle_local_m: [...p.tank_muzzle_local_m] as Vec3,
    supply_half_extents_m: [...p.supply_half_extents_m] as Vec3,
    // A map without forests has no canopy to stand in.
    canopy_height_m: canopies.length ? Math.min(...canopies) : Infinity,
    ruin_height_m: fixture.buildings.ruin_height_m,
  };
}
