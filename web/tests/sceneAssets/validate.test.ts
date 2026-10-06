// @vitest-environment node
// One golden failure per finding code: each case breaks exactly one rule of a
// valid synthetic asset and must produce that code. The valid assets produce
// none, so every code below is caused by its mutation alone.
import { expect, test } from "vitest";
import { buildClips } from "@packages/scene-assets/src/build.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import {
  FINDING_CODES,
  KIT_BUNDLE_MAX_BYTES,
  type AppearanceEntry,
  type Bundle,
  type SkeletonClips,
  type Finding,
  type FindingCode,
  type SkeletonEntry,
} from "@packages/scene-assets/src/schema.ts";
import {
  paintFindings,
  regionalFindings,
  typeAppearanceFindings,
  validateAppearance,
  validateSkeleton,
} from "@packages/scene-assets/src/validate.ts";
import { textureFindings } from "@packages/scene-assets/src/texture.ts";
import { bakeCatalog, kitBytesFindings } from "@packages/scene-assets/src/bake.ts";
import {
  HOUSE,
  MODULES,
  YARD,
  cityCatalog,
  cityContext,
  citySources,
  descriptor,
  kitGlb,
  testSet,
} from "./city";
import { grassClumpGlb } from "@packages/scene-assets/src/grass.ts";
import { SCENERY_KINDS } from "@packages/scene-assets/src/scenery.ts";
import {
  AUTHORITY,
  GRASS_SPEC,
  SKELETON_ENTRY,
  SOLDIER_CLIPS,
  TOLERANCES,
  blockGlb,
  panelGlb,
  soldierGlb,
  syntheticUnits,
  tankGlb,
  tankMounts,
  testCatalog,
  treeGlb,
  truckGlb,
  withJson,
  type SoldierOptions,
  type TankOptions,
  TANK_DRAWS,
} from "./synthetic";

const context = { authority: AUTHORITY, tolerances: TOLERANCES };
const rig = (() => {
  const { scene } = importScene(soldierGlb(), "rig.glb", 90);
  return {
    clips: buildClips(scene!, "rig.glb", "test-rig", 30, SKELETON_ENTRY.clips).built!,
    aim_reference: SKELETON_ENTRY.aim_reference,
  };
})();

async function soldier(
  options: SoldierOptions = {},
  entry: Partial<AppearanceEntry> = {},
  bytes?: Uint8Array,
) {
  const source = "soldier.glb";
  return (
    await validateAppearance(
      {
        name: "soldier",
        entry: { unit: "soldier", source, basis_yaw_deg: 90, skeleton: "test-rig", ...entry },
        files: { [source]: bytes ?? soldierGlb({ animated: false, ...options }) },
        skeleton: rig,
      },
      context,
    )
  ).findings;
}

async function tank(
  options: TankOptions = {},
  entry: Partial<AppearanceEntry> = {},
  bytes?: Uint8Array,
) {
  const source = "tank.glb";
  return (
    await validateAppearance(
      {
        name: "tank",
        entry: { unit: "vehicle", source, basis_yaw_deg: 0, mounts: TANK_DRAWS, ...entry },
        files: { [source]: bytes ?? tankGlb(options) },
      },
      context,
    )
  ).findings;
}

async function truck(options: Parameters<typeof truckGlb>[0] = {}) {
  return (
    await validateAppearance(
      {
        name: "truck",
        entry: { unit: "vehicle", source: "truck.glb", basis_yaw_deg: 0 },
        files: { "truck.glb": truckGlb(options) },
      },
      context,
    )
  ).findings;
}

/** A scenery appearance: `kind` names a `SCENERY_KINDS` row. */
async function scenery(
  kind: string | undefined,
  states: Record<string, Uint8Array>,
  footprint?: AppearanceEntry["footprint_half_m"],
) {
  const files = Object.fromEntries(
    Object.entries(states).map(([state, bytes]) => [`${state}.glb`, bytes]),
  );
  const entry: AppearanceEntry = {
    unit: "scenery",
    scenery: kind,
    states: Object.fromEntries(Object.keys(states).map((s) => [s, `${s}.glb`])),
    basis_yaw_deg: 0,
    ...(footprint ? { footprint_half_m: footprint } : {}),
  };
  return validateAppearance({ name: kind ?? "scenery", entry, files }, context);
}

/** A city set baked over its kit and the physical catalogue: every finding. */
async function city(set: unknown = testSet(), catalogue?: unknown[], kit?: Uint8Array) {
  const sources = citySources(set, kit);
  const result = await bakeCatalog(
    cityCatalog(),
    async (path) => sources[path],
    cityContext(catalogue),
  );
  return result.reports.flatMap((r) => r.findings);
}

/** A hedgerow whose one material `edit` changes: its findings. */
async function panel(edit: Parameters<typeof panelGlb>[0]) {
  return (await scenery("hedgerow", { summer: panelGlb(edit) })).findings;
}

async function skeleton(entry: Partial<SkeletonEntry>, bytes = soldierGlb()) {
  return (await validateSkeleton("test-rig", { ...SKELETON_ENTRY, ...entry }, bytes)).findings;
}

const LFS_POINTER = new TextEncoder().encode(
  "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 12345\n",
);

const GOLDEN: Record<FindingCode, () => Promise<Finding[]>> = {
  "structure.unreadable": () => tank({}, {}, new TextEncoder().encode("not a glb at all")),
  "structure.lfs_pointer": () => tank({}, {}, LFS_POINTER),
  "structure.external_buffer": () =>
    tank(
      {},
      {},
      withJson(tankGlb(), (j) => ((j.buffers as { uri?: string }[])[0].uri = "tank.bin")),
    ),
  "structure.unsupported": () =>
    tank(
      {},
      {},
      withJson(tankGlb(), (j) => (j.extensionsRequired = ["KHR_draco_mesh_compression"])),
    ),
  "structure.root": () => tank({ twoRoots: true }),
  "structure.skin_count": () => tank({ skinned: true }),
  "structure.unskinned_mesh": () => soldier({ looseMesh: true }),
  "structure.weights": () => soldier({ fiveWeights: true }),
  "structure.attributes": () => soldier({ noNormals: true }),
  "structure.scale": () => soldier({ stretch: true }),
  "structure.tier_count": () => tank({ lods: "none" }),
  "structure.tier_order": () => tank({ lods: "inverted" }),
  "structure.skeleton": () => soldier({ extraJoint: true }),
  "structure.loop_flags": () =>
    skeleton({ clips: { ...SKELETON_ENTRY.clips, walk: {} as { loop: boolean } } }),
  "structure.clips": () =>
    skeleton({}, soldierGlb({ clips: SOLDIER_CLIPS.filter((c) => c !== "prone_pinned") })).then(
      (f) => f,
    ),
  "structure.states": async () =>
    (await scenery("wall", { summer: blockGlb(1.2) }, [5, 4, 0.6])).findings,
  "structure.scenery_kind": async () =>
    (await scenery("gazebo", { default: blockGlb(3) })).findings,
  "structure.grass": async () => (await scenery("grass", { default: blockGlb(3) })).findings,
  "structure.regional_family": async () =>
    regionalFindings("crate_mars", { unit: "scenery", regional_family: "mars" }, ["paris"]),
  // A paint brighter than white, and paints on art with nothing to paint.
  "structure.paints": async () => [
    ...paintFindings("car", { unit: "scenery", paints: [[1.2, 0.5, 0.5]] }, [{ tint: 1 }]),
    ...paintFindings("car", { unit: "scenery", paints: [[0.5, 0.5, 0.5]] }, [{ tint: 0 }]),
  ],
  "structure.texture": () =>
    tank(
      {},
      {},
      withJson(
        tankGlb({ textures: { size: 4 } }),
        (j) => ((j.images as { mimeType: string }[])[0].mimeType = "image/jpeg"),
      ),
    ),
  "texture.size": () => tank({ textures: { size: 6 } }),
  "texture.mips": async () => {
    const built = await validateAppearance(
      {
        name: "tank",
        entry: { unit: "vehicle", source: "tank.glb", basis_yaw_deg: 0, mounts: TANK_DRAWS },
        files: { "tank.glb": tankGlb({ textures: { size: 8 } }) },
      },
      context,
    );
    const bundle = built.bundle as Exclude<Bundle, SkeletonClips>;
    bundle.textures[0].levels.pop();
    return textureFindings("tank", bundle);
  },
  "texture.tangents": () => tank({ textures: { size: 4, tangents: false } }),
  // Each material rule's variants are in material.test.ts.
  "material.coverage": () => panel((m) => (m.alphaMode = "DITHER")),
  "material.coverage_source": () => panel((m) => (m.alphaMode = "MASK")),
  "material.wear": () =>
    panel((m) => {
      m.alphaMode = "BLEND";
      m.pbrMetallicRoughness.baseColorFactor = [1, 1, 1, 0.5];
      m.extras = { wear: [0.2, 0.2, 0.2, 1] };
    }),
  "material.interior": () =>
    tank(
      {},
      {},
      withJson(tankGlb(), (j) => (j.materials[0].extras = { interior: "rooms" })),
    ),
  "basis.ground": () => soldier({ lift: 0.1 }),
  "basis.forward": () => tank({}, { basis_yaw_deg: 180 }),
  "basis.up": () => tank({ flip: true }),
  "fit.soldier_height": () => soldier({ top: 1.9 }),
  "fit.eye": () => soldier({ eyeY: 1.3 }),
  "fit.muzzle": () => soldier({ muzzleY: 1.0 }),
  "fit.hull_extents": () => tank({ hullHalfY: 2.2 }),
  "fit.vehicle_muzzle": () => tank({ muzzleX: 5.9 }),
  "fit.footprint": async () =>
    (await scenery("crate", { default: blockGlb(2) }, [1, 1, 1])).findings,
  "fit.piece": async () =>
    (
      await scenery(
        "wreck",
        { default: blockGlb(6), hull: blockGlb(2), turret: blockGlb(2, 5) },
        [5, 4, 3],
      )
    ).findings,
  "fit.muzzle_arc": () => tank({ turretX: -1 }),
  "fit.mount_draw": () => tank({}, { mounts: { cannon: "gun" } }),
  "fit.part_nodes": async () =>
    (
      await validateAppearance(
        {
          name: "tank",
          entry: { unit: "vehicle", source: "t.glb", basis_yaw_deg: 0, mounts: TANK_DRAWS },
          files: { "t.glb": tankGlb() },
        },
        { ...context, authority: { ...AUTHORITY, units: syntheticUnits({ parts: ["era"] }) } },
      )
    ).findings,
  "fit.type_appearance": async () => {
    const { tank: _, ...appearances } = testCatalog().appearances;
    return typeAppearanceFindings(appearances, AUTHORITY.units);
  },
  "fit.canopy": async () => (await scenery("tree", { summer: treeGlb(12.5) })).findings,
  "fit.tree_size": async () =>
    (await scenery("tree", { summer: treeGlb(11, 0, { bole: 0.2 }) })).findings,
  "fit.dressing": async () => (await scenery("dressing", { summer: blockGlb(1.2) })).findings,
  "budget.tier_triangles": async () =>
    (await scenery("tree", { summer: treeGlb(11, 0, { crowns: overBudget(3) }) })).findings,
  "nodes.missing": () => tank({ omit: "hmg_muzzle" }),
  "nodes.hierarchy": () => tank({ muzzleUnderTurret: true }),
  "nodes.duplicate": () => tank({ duplicateWheel: true }),
  "nodes.track_properties": () => tank({ noTrackProperties: true }),
  "nodes.deploy_motion": () => truck({ noDeployMotion: true }),
  "kit.module": () => city(testSet(), undefined, kitGlb(MODULES, true)),
  "kit.bytes": async () => kitBytesFindings("kit", KIT_BUNDLE_MAX_BYTES + 1),
  "templates.source": () => city("not a set at all"),
  "templates.kit": () => city(testSet((set) => (set.kit = "another_kit"))),
  "templates.row": () => city(testSet((set) => set.templates[0].states.intact![0].pop())),
  "templates.module": () => city(testSet((set) => set.modules.push("balcony"))),
  "templates.state": () => city(testSet((set) => delete set.templates[0].states.intact)),
  "templates.physical": () => city(testSet((set) => (set.templates[0].descriptor.entrances = []))),
  "templates.fit": () => city(testSet((set) => (set.fit.side_m = 0))),
  "templates.catalogue": () => city(testSet(), [HOUSE]),
  "templates.coverage": () => city(testSet(), [HOUSE, YARD, descriptor("test-shed")]),
};

test("the valid synthetic assets produce no findings at all", async () => {
  const all = [
    ...(await soldier()),
    ...(await tank()),
    ...(await truck()),
    ...(await scenery("wall", { default: blockGlb(1.2) }, [5, 4, 0.6])).findings,
    ...(await scenery("tree", { summer: treeGlb() })).findings,
    ...(await scenery("hedgerow", { summer: treeGlb(3) })).findings,
    ...(await scenery("grass", { summer: grassClumpGlb("tuft", GRASS_SPEC) })).findings,
    ...(await skeleton({})),
    ...(await city()),
  ];
  expect(all).toEqual([]);
});

test("every finding code has a golden failure", () => {
  expect(Object.keys(GOLDEN).sort()).toEqual([...FINDING_CODES].sort());
});

for (const code of FINDING_CODES)
  test(`golden failure: ${code}`, async () => {
    const findings = await GOLDEN[code]();
    const hit = findings.find((f) => f.code === code);
    expect(hit, JSON.stringify(findings, null, 1)).toBeDefined();
    expect(hit!.fix.length).toBeGreaterThan(0);
    expect(hit!.severity).toBe("error");
  });

test("a tree must stand inside the simulation's canopy; a hedgerow need not", async () => {
  // The crown top is measured against the lowest canopy the fixture's forests have.
  const codes = async (kind: string, height: number) =>
    (await scenery(kind, { summer: treeGlb(height) })).findings.map((f) => f.code);
  expect(await codes("tree", 12.5)).toContain("fit.canopy");
  expect(await codes("tree", 11.9)).not.toContain("fit.canopy");
  expect(await codes("hedgerow", 14)).toEqual([]);
});

test("every tree is one height and one girth of bole, within the kind's band; a hedgerow is any size", async () => {
  const { top_m, bole_radius_m, within } = SCENERY_KINDS.tree.size!;
  const codes = async (kind: string, height: number, bole: number) =>
    (await scenery(kind, { summer: treeGlb(height, 0, { bole }) })).findings.map((f) => f.code);
  const [inside, outside] = [1 + within * 0.8, 1 + within * 1.2];
  expect(await codes("tree", top_m * inside, bole_radius_m / inside)).toEqual([]);
  expect(await codes("tree", top_m / inside, bole_radius_m * inside)).toEqual([]);
  expect(await codes("tree", top_m * outside, bole_radius_m)).toEqual(["fit.tree_size"]);
  expect(await codes("tree", top_m / outside, bole_radius_m)).toEqual(["fit.tree_size"]);
  expect(await codes("tree", top_m, bole_radius_m * outside)).toEqual(["fit.tree_size"]);
  expect(await codes("tree", top_m, bole_radius_m / outside)).toEqual(["fit.tree_size"]);
  expect(await codes("hedgerow", top_m / 3, bole_radius_m * 3)).toEqual([]);
});

test("forest-floor dressing has no body, so none stands taller than its kind may", async () => {
  // A soldier behind it must still be seen: the forest hides him, not a bush.
  const top = SCENERY_KINDS.dressing.top_m!;
  const codes = async (height: number) =>
    (await scenery("dressing", { summer: blockGlb(height) })).findings.map((f) => f.code);
  expect(await codes(top - 0.05)).toEqual([]);
  expect(await codes(top + 0.05)).toEqual(["fit.dressing"]);
});

test("a tree's bole is measured where it stands, not about the origin", async () => {
  // A leaning or off-centre bole has the girth it has.
  const { top_m, bole_radius_m } = SCENERY_KINDS.tree.size!;
  const off = await scenery("tree", {
    summer: treeGlb(top_m, 0, { bole: bole_radius_m, boleAt: [0.6, -0.4] }),
  });
  expect(off.findings).toEqual([]);
});

test("a tree's crown reaches no farther than the simulation's canopy radius, on every tier", async () => {
  // The synthetic crown is a square, so its corner reaches radius × √2.
  const reach = (corner: number) => corner / Math.SQRT2;
  const codes = async (kind: string, corner: number) =>
    (await scenery(kind, { summer: treeGlb(11, 0, { radius: reach(corner) }) })).findings.map(
      (f) => f.code,
    );
  expect(await codes("tree", AUTHORITY.canopy_radius_m + 0.1)).toEqual(["fit.canopy"]);
  expect(await codes("tree", AUTHORITY.canopy_radius_m - 0.1)).toEqual([]);
  expect(await codes("hedgerow", AUTHORITY.canopy_radius_m + 3)).toEqual([]);
});

/** Crown boxes per tier that put tier `tier` one box over the tree budget
 *  (`by` 0: the most boxes inside it). Finer tiers draw as many, inside
 *  their own larger budgets, so the tiers stay ordered; coarser ones draw one. */
function overBudget(tier: number, by = 1): number[] {
  const budget = SCENERY_KINDS.tree.tier_triangles!;
  const trunk = 12;
  const boxes = Math.floor((budget[tier] - trunk) / 12) + by;
  return budget.map((_, t) => (t <= tier ? boxes : 1));
}

test("a tree's tiers each stay inside the triangle budget; a hedgerow has none", async () => {
  for (let tier = 0; tier < 4; tier++) {
    const over = await scenery("tree", { summer: treeGlb(11, 0, { crowns: overBudget(tier) }) });
    expect(over.findings.map((f) => f.code)).toEqual(["budget.tier_triangles"]);
    expect(over.findings[0].message).toContain(`tier ${tier}`);
    const within = await scenery("tree", {
      summer: treeGlb(11, 0, { crowns: overBudget(tier, 0) }),
    });
    expect(within.findings).toEqual([]);
  }
  const hedge = await scenery("hedgerow", { summer: treeGlb(3, 0, { crowns: overBudget(3) }) });
  expect(hedge.findings).toEqual([]);
});

test("an LFS pointer's finding prints the exact pull command", async () => {
  const [finding] = await tank({}, {}, LFS_POINTER);
  expect(finding.fix).toBe('run: git lfs pull --include="tank.glb"');
});

test("the tank muzzle is measured against its type's mount row, not a built-in reach", async () => {
  const realistic = tankGlb({ muzzleX: 5.9 });
  expect((await tank({}, {}, realistic)).map((f) => f.code)).toContain("fit.vehicle_muzzle");
  const retuned = await validateAppearance(
    {
      name: "tank",
      entry: { unit: "vehicle", source: "t.glb", basis_yaw_deg: 0, mounts: TANK_DRAWS },
      files: { "t.glb": realistic },
    },
    {
      tolerances: TOLERANCES,
      authority: { ...AUTHORITY, units: syntheticUnits({ mounts: tankMounts(5.9) }) },
    },
  );
  expect(retuned.findings).toEqual([]);
});

test("a mount is drawn by the rig its model declares, whatever the mount is called", async () => {
  const [cannon, hmg] = tankMounts();
  const renamed = [
    { ...cannon, name: "main gun" },
    { ...hmg, name: "pintle MG", on: "main gun" },
  ];
  const fit = (mounts: Record<string, "gun" | "hmg">) =>
    validateAppearance(
      {
        name: "tank",
        entry: { unit: "vehicle", source: "t.glb", basis_yaw_deg: 0, mounts },
        files: { "t.glb": tankGlb() },
      },
      {
        tolerances: TOLERANCES,
        authority: { ...AUTHORITY, units: syntheticUnits({ mounts: renamed }) },
      },
    );
  // No "HMG" in its name, and still drawn on the HMG's ring.
  expect((await fit({ "main gun": "gun", "pintle MG": "hmg" })).findings).toEqual([]);
  // Swapped rigs: each mount's muzzle is measured on the other's nodes.
  const swapped = (await fit({ "main gun": "hmg", "pintle MG": "gun" })).findings;
  expect(swapped.map((f) => f.code)).toContain("fit.vehicle_muzzle");
  // Undeclared, unknown and shared rigs are named.
  const bad = (await fit({ "main gun": "gun", coax: "hmg" })).findings;
  expect(bad.filter((f) => f.code === "fit.mount_draw").map((f) => f.message)).toEqual([
    expect.stringMatching(/declares mount "coax", which tank does not have/),
    expect.stringMatching(/mount "pintle MG" is not drawn by any rig/),
  ]);
  const shared = (await fit({ "main gun": "gun", "pintle MG": "gun" })).findings;
  expect(shared.map((f) => f.message)).toContainEqual(
    expect.stringMatching(/"main gun" and "pintle MG" are both drawn by the gun rig/),
  );
});

test("the roof HMG is fitted on its own pivot, turning with the turret and on its own ring", async () => {
  // Right at rest, wrong as it turns: the row puts the HMG's pivot at its
  // muzzle's foot, not on the ring the model turns it on.
  const [cannon, hmg] = tankMounts();
  const moved = {
    ...hmg,
    pivot_m: [hmg.pivot_m[0] + 0.9, hmg.pivot_m[1], hmg.pivot_m[2]],
    muzzle_m: [0, 0, hmg.muzzle_m![2]],
  } as typeof hmg;
  const result = await validateAppearance(
    {
      name: "tank",
      entry: { unit: "vehicle", source: "t.glb", basis_yaw_deg: 0, mounts: TANK_DRAWS },
      files: { "t.glb": tankGlb() },
    },
    {
      tolerances: TOLERANCES,
      authority: { ...AUTHORITY, units: syntheticUnits({ mounts: [cannon, moved] }) },
    },
  );
  const codes = result.findings.map((f) => f.code);
  expect(codes).toContain("fit.muzzle_arc");
  expect(codes).not.toContain("fit.vehicle_muzzle");
  expect(result.findings.find((f) => f.code === "fit.muzzle_arc")?.message).toContain("hmg_muzzle");
});

test("a supply truck facing backwards and missing a mast stage is caught", async () => {
  expect((await truck({ reversed: true })).map((f) => f.code)).toContain("basis.forward");
  const missing = await truck({ omit: "deploy_mast_3" });
  expect(missing.map((f) => f.message).join("\n")).toContain('no "deploy_mast_3" node');
  const noPad = await truck({ omit: "deploy_leg_RL_pad" });
  expect(noPad.map((f) => f.message).join("\n")).toContain('no "deploy_leg_RL_pad" node');
});

test("a deployed pad that stops short of the ground is caught, by name", async () => {
  const short = await truck({ jackDrop: 0.05 });
  const found = short.filter((f) => f.code === "nodes.deploy_motion");
  expect(found.map((f) => f.message.match(/"(deploy_leg_\w+)"/)?.[1]).sort()).toEqual([
    "deploy_leg_FL_pad",
    "deploy_leg_FR_pad",
    "deploy_leg_RL_pad",
    "deploy_leg_RR_pad",
  ]);
});

test("a scenery kind needs the states of its row, and builds a static bundle", async () => {
  const wall = await scenery("wall", { default: blockGlb(1.2) }, [5, 4, 0.6]);
  expect(wall.bundle?.kind).toBe("static");
  const missing = await scenery("wall", { summer: blockGlb(1.2) });
  expect(missing.findings.map((f) => f.message).join("\n")).toContain('no "default" state');
});

test("catalog tolerances, not the rules, admit art that sits outside the default", async () => {
  expect(
    (await tank({ hullHalfY: 2.2 }, { tolerances: { hull_extent_m: 0.5 } })).map((f) => f.code),
  ).toEqual([]);
});

test("a prop scenery kind is fitted to its declared box; a tree has no box to fit", async () => {
  const crate = await scenery("crate", { default: blockGlb(2) }, [1, 1, 1]);
  expect(crate.findings.map((f) => f.code)).toContain("fit.footprint");
  // Each face that is off is named, the top among them.
  const tall = await scenery("crate", { default: blockGlb(9) }, [4, 4, 3]);
  const faces = tall.findings.map((f) => f.message).join("\n");
  expect(faces).toMatch(/-x face at -5\.000 m vs -4\.000/);
  expect(faces).toMatch(/\+z face at 9\.000 m vs 6\.000/);
  expect((await scenery("crate", { default: blockGlb(2) }, [5, 4, 1])).findings).toEqual([]);
  expect((await scenery("crate", { default: blockGlb(2) })).findings.map((f) => f.code)).toEqual([
    "fit.footprint",
  ]);
  expect((await scenery("tree", { summer: treeGlb() })).findings).toEqual([]);
});

test("a wreck's pieces lie within the whole wreck, off the ground and short of its box", async () => {
  const pieces = (turret: Uint8Array) =>
    scenery("wreck", { default: blockGlb(6), hull: blockGlb(2), turret }, [5, 4, 3]);
  // The thrown turret lies 3 m up on the hull: no ground or box finding.
  expect((await pieces(blockGlb(2, 3))).findings).toEqual([]);
  // One standing out of the whole is not a piece of it.
  const proud = (await pieces(blockGlb(2, 5))).findings;
  expect(proud.map((f) => f.code)).toEqual(["fit.piece"]);
  expect(proud[0].message).toMatch(/turret/);
});

test("the hull top has its own tolerance, so an antenna does not loosen the sides", async () => {
  const tall = await tank({ antenna: 3.4 });
  expect(tall.map((f) => f.message).join("\n")).toMatch(/\+z face at 3\.400 m vs 2\.400/);
  expect(await tank({ antenna: 3.4 }, { tolerances: { hull_top_m: 1.1 } })).toEqual([]);
  const wide = await tank({ antenna: 3.4, hullHalfY: 2.2 }, { tolerances: { hull_top_m: 1.1 } });
  expect(wide.map((f) => f.code)).toEqual(["fit.hull_extents"]);
});
