// The fit authority, read from a scenario fixture and the unit catalog: the
// one soldier frame every squad shares (`physics`), every unit type's own
// resolved numbers (`UnitCatalog`), the lowest canopy of the fixture's
// forests and a ruin's height. The CLI, the bake and the workbench all read
// it here, so they judge art against the same numbers.

import type { Authority } from "./schema.ts";
import type { UnitCatalog } from "./units.ts";

/** The parts of a fixture the authority reads. */
export interface AuthorityFixture {
  physics: {
    soldier_height_m: number;
    infantry_eye_m: number;
    infantry_muzzle_m: number;
  };
  map: { forests: readonly { canopy_height_m: number }[] };
  props: { building: { destroyed: { into: { height_m: number } } } };
}

export function fixtureAuthority(fixture: AuthorityFixture, units: UnitCatalog): Authority {
  const p = fixture.physics;
  const canopies = fixture.map.forests.map((f) => f.canopy_height_m);
  return {
    soldier_height_m: p.soldier_height_m,
    infantry_eye_m: p.infantry_eye_m,
    infantry_muzzle_m: p.infantry_muzzle_m,
    units,
    // A map without forests has no canopy to stand in.
    canopy_height_m: canopies.length ? Math.min(...canopies) : Infinity,
    // A collapse's ruin: the building row's destroyed state (34c).
    ruin_height_m: fixture.props.building.destroyed.into.height_m,
  };
}
