// The fit authority, read from a scenario fixture and the catalog: the one
// soldier frame every squad shares (`physics`), every unit type's own
// resolved numbers (`UnitCatalog`) and the canopy of the fixture's forests.
// The CLI, the bake and the workbench all read it here, so they judge art
// against the same numbers.

import type { Authority } from "./schema.ts";
import type { UnitCatalog } from "./units.ts";

/** The parts of a fixture the authority reads. */
export interface AuthorityFixture {
  physics: {
    soldier_height_m: number;
    infantry_eye_m: number;
    infantry_muzzle_m: number;
  };
  forests: { rule: { canopy_height_m: number; canopy_radius_m: number } };
}

export function fixtureAuthority(fixture: AuthorityFixture, units: UnitCatalog): Authority {
  const p = fixture.physics;
  return {
    soldier_height_m: p.soldier_height_m,
    infantry_eye_m: p.infantry_eye_m,
    infantry_muzzle_m: p.infantry_muzzle_m,
    units,
    canopy_height_m: fixture.forests.rule.canopy_height_m,
    canopy_radius_m: fixture.forests.rule.canopy_radius_m,
  };
}
