// The fit authority, read from a scenario fixture and the catalog: the one
// soldier frame every squad shares (`physics`), every unit type's own
// resolved numbers (`UnitCatalog`), the canopy of the fixture's
// forests, a ruin's height and how a template building ends (the catalog's
// prop types). The CLI, the bake and the workbench all read
// it here, so they judge art against the same numbers.

import type { Authority, BuildingCollapse } from "./schema.ts";
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
    ruin_height_m: ruinHeight(units),
    collapse: buildingCollapse(units),
  };
}

/** How a template building ends (C42, C43): the rule on the prop types that
 *  collapse as buildings, which every template's damage state is authored to.
 *  Types that carry differing rules are refused: a template is not a prop
 *  type, so its art can fit one rule only. */
function buildingCollapse(units: UnitCatalog): BuildingCollapse | null {
  const rules = new Map<string, { rule: BuildingCollapse; ids: string[] }>();
  for (const [id, t] of Object.entries(units.view.props)) {
    const into = typeof t.destroyed === "object" ? t.destroyed.into : null;
    if (!into?.building) continue;
    const rule = {
      min_height_m: into.height_m,
      height_fraction: into.building.height_fraction,
      max_height_m: into.building.max_height_m,
      max_floors: into.building.collapse_max_floors,
    };
    const key = JSON.stringify(rule);
    rules.set(key, { rule, ids: [...(rules.get(key)?.ids ?? []), id] });
  }
  if (rules.size > 1)
    throw new Error(
      `prop types collapse as buildings by differing rules (${[...rules.values()].map((r) => r.ids.join(", ")).join("; ")}): a template's damage state fits one`,
    );
  return rules.values().next().value?.rule ?? null;
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
