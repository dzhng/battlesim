// @vitest-environment node
// The unit art rules (specs/unit-models slice 12): tiers that really reduce,
// dressing held to its own allowance outside the hull, and class budgets.
import { expect, test } from "vitest";
import type { AppearanceEntry } from "@packages/scene-assets/src/schema.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import { buildClips } from "@packages/scene-assets/src/build.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import { UNIT_ART, unitArtRule, type UnitArtRule } from "@packages/scene-assets/src/unitArt.ts";
import {
  AUTHORITY,
  SKELETON_ENTRY,
  TANK_DRAWS,
  TOLERANCES,
  soldierGlb,
  tankGlb,
  type TankOptions,
} from "./synthetic";

const context = { authority: AUTHORITY, tolerances: TOLERANCES };
const rig = (() => {
  const { scene } = importScene(soldierGlb(), "rig.glb", 90);
  return {
    clips: buildClips(scene!, "rig.glb", "test-rig", 30, SKELETON_ENTRY.clips).built!,
    aim_reference: SKELETON_ENTRY.aim_reference,
  };
})();

function tank(options: TankOptions = {}, entry: Partial<AppearanceEntry> = {}) {
  return validateAppearance(
    {
      name: "tank",
      entry: {
        unit: "vehicle",
        source: "tank.glb",
        basis_yaw_deg: 0,
        mounts: TANK_DRAWS,
        ...entry,
      },
      files: { "tank.glb": tankGlb(options) },
    },
    context,
  );
}

test("a unit's mesh that names no tier is refused, though the tiers it would be copied into still reduce", async () => {
  // The turret is in every tier already: only the missing name is wrong.
  const result = await tank({ unsuffixed: "turret_shell" });
  expect(result.findings.map((f) => [f.code, f.severity])).toEqual([
    ["structure.tier_unsuffixed", "error"],
  ]);
  expect(result.findings[0].message).toContain('"turret_shell"');
  expect(result.bundle).toBeNull();
});

test("each tier of a unit draws at most its class's ratio of the tier before", async () => {
  // The synthetic tank draws 10, 8, 5 and 4 boxes. Its road wheels kept a
  // tier longer, tier 2 draws all of tier 1's 8: no more, but no fewer.
  const wheels = { wheel_L_1_disc: 3, wheel_R_1_disc: 3, wheel_L_2_disc: 3 };
  const flat = await tank({ partTiers: wheels });
  expect(flat.findings.map((f) => [f.code, f.severity])).toEqual([
    ["structure.tier_ratio", "error"],
  ]);
  expect(flat.findings[0].message).toMatch(/tier 2 draws 96 triangles, 1\.00 of tier 1's 96/);
  expect((await tank()).findings).toEqual([]);
});

test("a thin antenna marked as dressing may rise above the hull; unmarked, it is hull and must fit", async () => {
  // The hull box's top is 2.4 m; the antenna is 1 cm across.
  const codes = async (o: TankOptions) => (await tank(o)).findings.map((f) => f.code);
  expect(await codes({ antenna: 3.4 })).toEqual(["fit.hull_extents"]);
  expect(await codes({ antenna: 3.4, dressing: { antenna: true } })).toEqual([]);
  const { thin_top_m } = unitArtRule(null).dressing;
  expect(await codes({ antenna: 2.4 + thin_top_m - 0.05, dressing: { antenna: true } })).toEqual(
    [],
  );
  expect(await codes({ antenna: 2.4 + thin_top_m + 0.05, dressing: { antenna: true } })).toEqual([
    "fit.dressing",
  ]);
});

test("bulky dressing stays within its allowance of the hull box on every face", async () => {
  // A 1 × 1 m load on the rear deck (the roof is at 1.6 m, the box's top 2.4).
  const { bulky_m } = unitArtRule(null).dressing;
  const load = (overhang: number, height: number) =>
    tank({ dressing: { stowage: { size: [1, 1, height], overhang } } }).then((r) => r.findings);
  expect(await load(bulky_m - 0.05, 0.8 + bulky_m - 0.05)).toEqual([]);
  const behind = await load(bulky_m + 0.05, 0.5);
  expect(behind.map((f) => f.code)).toEqual(["fit.dressing"]);
  expect(behind[0].message).toMatch(/dressing_stowage.*-x/);
  // Too wide to be an antenna, it may not rise as one.
  const tall = await load(0, 0.8 + bulky_m + 0.05);
  expect(tall.map((f) => f.code)).toEqual(["fit.dressing"]);
  expect(tall[0].message).toMatch(/dressing_stowage.*\+z/);
});

test("dressing never excuses the hull: the body still fits its box within the tolerance", async () => {
  const wide = await tank({ hullHalfY: 2.2, antenna: 3.4, dressing: { antenna: true } });
  expect(wide.findings.map((f) => f.code)).toEqual(["fit.hull_extents"]);
  expect(wide.findings[0].message).toMatch(/\+y face/);
  expect(wide.findings[0].fix).not.toMatch(/widen/);
});

/** Run `body` with `rows` added to the class table, then take them out. */
async function withClassRows<T>(
  rows: Record<string, Partial<UnitArtRule>>,
  body: () => Promise<T>,
) {
  Object.assign(UNIT_ART, rows);
  try {
    return await body();
  } finally {
    for (const cls of Object.keys(rows)) delete UNIT_ART[cls];
  }
}

test("a unit is held to its own class's budgets: triangles per tier, bundle bytes, texture layers", async () => {
  // The synthetic tank is tracked and heavy; its textured paint is three textures.
  const textured = { textures: { size: 4 } };
  const tight = {
    tier_triangles: [120, 95, 60, 48] as const,
    bundle_bytes: 1000,
    textures: 2,
  };
  const findings = await withClassRows({ tracked_heavy: tight }, () =>
    tank(textured).then((r) => r.findings),
  );
  expect(findings.map((f) => [f.code, f.severity, f.message.match(/over .*/)?.[0]])).toEqual([
    ["budget.tier_triangles", "error", "over its budget of 95"],
    ["budget.bundle_bytes", "error", "over its budget of 1000"],
    ["budget.unit_textures", "error", "over its budget of 2"],
  ]);
  // Another class's budget is not this unit's.
  expect(
    await withClassRows({ wheeled_light: tight, soldier: tight }, () =>
      tank(textured).then((r) => r.findings),
    ),
  ).toEqual([]);
});

test("every soldier is held to the soldier class's budgets", async () => {
  const findings = await withClassRows({ soldier: { tier_triangles: [47, 36, 24, 12] } }, () =>
    validateAppearance(
      {
        name: "soldier",
        entry: { unit: "soldier", source: "s.glb", basis_yaw_deg: 90, skeleton: "test-rig" },
        files: { "s.glb": soldierGlb({ animated: false }) },
        skeleton: rig,
      },
      context,
    ).then((r) => r.findings),
  );
  expect(findings.map((f) => f.code)).toEqual(["budget.tier_triangles"]);
  expect(findings[0].message).toContain("tier 0 draws 48 triangles");
});
