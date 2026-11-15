// @vitest-environment node
// A soldier's fit to the simulation's soldier in the poses that are measured
// beyond standing aim: a supported active kit's hold, and each way he lies dead.
import { expect, test } from "vitest";
import { INFANTRY_CORPSES, type Tolerances } from "@packages/scene-assets/src/schema.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import { UnitCatalog } from "@packages/scene-assets/src/units.ts";
import {
  AUTHORITY,
  TOLERANCES,
  soldierGlb,
  soldierRig,
  syntheticUnits,
  tankMounts,
} from "./synthetic";

const context = { authority: AUTHORITY, tolerances: TOLERANCES };
const rig = soldierRig();

test.each([0, 0.4])(
  "a grounded active kit is checked against its declared full bore (error %s m)",
  async (boreError) => {
    const units = syntheticUnits();
    const view = units.view;
    const grounded = new UnitCatalog({
      ...view,
      units: view.units.map((u) =>
        u.id === "rifle"
          ? {
              ...u,
              mounts: [
                {
                  ...tankMounts()[0],
                  turret: false,
                  on: null,
                  pivot_m: [0, 0, 0.75],
                  muzzle_m: [0.7 + boreError, -0.2, 0],
                  operator_appearance: {
                    active: ["tripod"],
                    carried: ["packed"],
                    active_pose: { clip: "kneel_fire", phase: 0 },
                  },
                },
              ],
            }
          : u,
      ),
    });
    const result = await validateAppearance(
      {
        name: "tripod",
        entry: { unit: "soldier", source: "tripod.glb", basis_yaw_deg: 90, skeleton: "test-rig" },
        files: { "tripod.glb": soldierGlb({ animated: false, muzzleY: 0.75 }) },
        skeleton: rig,
      },
      { ...context, authority: { ...AUTHORITY, units: grounded } },
    );
    if (boreError === 0) expect(result.findings.map((f) => f.message)).toEqual([]);
    else
      expect(
        result.findings.some((f) => f.code === "fit.muzzle" && f.message.includes("declares")),
      ).toBe(true);
  },
);

/** The `fit.lies` findings of the synthetic soldier on a rig whose corpses
 *  come to rest with the pelvis at `corpseZ`. */
async function lies(corpseZ: number, tolerances: Partial<Tolerances> = {}) {
  const result = await validateAppearance(
    {
      name: "soldier",
      entry: { unit: "soldier", source: "s.glb", basis_yaw_deg: 90, skeleton: "test-rig" },
      files: { "s.glb": soldierGlb({ animated: false }) },
      skeleton: soldierRig({ corpseZ }),
    },
    { ...context, tolerances: { ...TOLERANCES, ...tolerances } },
  );
  return result.findings.filter((f) => f.code === "fit.lies");
}

test("a corpse lies on the ground: not raised so it reads as a man, not sunk into the earth", async () => {
  expect(await lies(0.15)).toEqual([]);
  // Hung at hip height, or buried to the chest: every corpse pose is named.
  for (const corpseZ of [0.9, -0.3]) {
    const found = await lies(corpseZ);
    expect(found.map((f) => INFANTRY_CORPSES.find((c) => f.message.includes(`"${c}"`)))).toEqual([
      ...INFANTRY_CORPSES,
    ]);
  }
});

test("how high a corpse may lie is the catalog's tolerance", async () => {
  expect(await lies(0.9, { soldier_lying_height_m: 1.5 })).toEqual([]);
});
