// The fit authority, read from a scenario fixture and the catalog: the one
// soldier frame every squad shares (`physics`), every unit type's own
// resolved numbers (`UnitCatalog`), the lowest canopy of the fixture's
// forests and a ruin's height (the catalog's prop types). The CLI, the bake and the workbench all read
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
  forests: { rule: { canopy_height_m: number } };
}

export function fixtureAuthority(fixture: AuthorityFixture, units: UnitCatalog): Authority {
  const p = fixture.physics;
  return {
    soldier_height_m: p.soldier_height_m,
    infantry_eye_m: p.infantry_eye_m,
    infantry_muzzle_m: p.infantry_muzzle_m,
    units,
    canopy_height_m: fixture.forests.rule.canopy_height_m,
    ruin_height_m: ruinHeight(units),
  };
}

/** A collapse's ruin (34c): the height of the remains the prop types drawn
 *  by the building appearances leave, which their one ruin state is authored
 *  to. Types that fall to differing heights are refused: no one ruin state
 *  fits them all. */
function ruinHeight(units: UnitCatalog): number {
  const heights = new Map<number, string[]>();
  for (const [id, t] of Object.entries(units.view.props)) {
    const d = t.destroyed;
    if (t.appearance.drawn_by !== "building" || typeof d !== "object") continue;
    heights.set(d.into.height_m, [...(heights.get(d.into.height_m) ?? []), id]);
  }
  if (heights.size > 1) {
    const each = [...heights].map(([h, ids]) => `${ids.join(", ")} at ${h} m`).join("; ");
    throw new Error(
      `the prop types drawn by the building appearances leave remains of differing heights (${each}): their one ruin state fits one`,
    );
  }
  return heights.keys().next().value ?? Infinity;
}
