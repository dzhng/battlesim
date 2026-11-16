// @vitest-environment node
// The bake end to end: a valid catalog bakes to content-addressed bundles,
// deterministically, and the bundles decode to what the art says, a static
// prop's far mesh among it.
import { expect, test } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { encodeBundle } from "@packages/scene-assets/src/codec.ts";
import { sha256Hex } from "@packages/scene-assets/src/glb.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { positionsBounds } from "@packages/scene-assets/src/pose.ts";
import { SCENERY_KINDS, type SceneryRule } from "@packages/scene-assets/src/scenery.ts";
import type { Catalog, FindingCode } from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, bakedBundle, testCatalog, testSources, treeGlb } from "./synthetic";
import { KIT, MODULES, cityCatalog, cityContext, citySources, kitGlb, testSet } from "./city";
import { FAR_SLAB, farPropGlb, withFarCopy } from "./farProp";

async function bake() {
  const sources = testSources();
  return bakeCatalog(testCatalog(), async (path) => sources[path], { authority: AUTHORITY });
}

test("a valid catalog bakes every entry with no error findings", async () => {
  const result = await bake();
  const errors = result.reports.flatMap((r) => r.findings.filter((f) => f.severity === "error"));
  expect(errors).toEqual([]);
  expect(result.ok).toBe(true);
  expect(Object.keys(result.runtime.appearances).sort()).toEqual([
    "crate",
    "rifleman",
    "tank",
    "tank_wreck",
    "truck",
    "truck_wreck",
  ]);
  expect(Object.keys(result.runtime.skeletons)).toEqual(["test-rig"]);
});

test("each runtime file is named by the sha256 of its bytes, and loading the catalog reads every one", async () => {
  const result = await bake();
  for (const [path, bytes] of result.files) expect(path.split("/")[0]).toBe(await sha256Hex(bytes));
  const files = new Map(result.files);
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  const fetch = memoryFetch(files, "/a/");
  const read = new Set<string>();
  await new AppearanceLibrary((url) => {
    read.add(url.slice("/a/".length));
    return fetch(url);
  }).load("/a/");
  read.delete("catalog.json");
  expect([...read].sort()).toEqual([...result.files.keys()].sort());
});

test("baking twice gives the same hashes", async () => {
  const [a, b] = [await bake(), await bake()];
  expect(b.runtime).toEqual(a.runtime);
});

test("bundles decode to the kind, tiers and parts the catalog declared", async () => {
  const result = await bake();
  const decoded = (hash: string) => bakedBundle(result, hash);
  const rifleman = await decoded(result.runtime.appearances.rifleman.bundle);
  if (rifleman.kind !== "skinned") throw new Error(rifleman.kind);
  expect(rifleman.skeleton).toBe("test-rig");
  // The unweighted leaf joint is dropped; the armature folds into the root.
  expect(rifleman.joints.map((j) => j.name)).toEqual([
    "root",
    "pelvis",
    "spine_01",
    "Head",
    "hand_r",
  ]);
  expect(rifleman.sockets.map((s) => s.name).sort()).toEqual(["eye", "muzzle"]);
  expect(rifleman.tiers).toHaveLength(4);
  expect(rifleman.tiers[0].indices.length).toBeGreaterThan(rifleman.tiers[1].indices.length);
  const clips = await decoded(result.runtime.skeletons["test-rig"]);
  if (clips.kind !== "clips") throw new Error(clips.kind);
  expect(clips.joints.map((j) => j.name)).toEqual(rifleman.joints.map((j) => j.name));
  expect(clips.clips.find((c) => c.name === "walk")?.loop).toBe(true);
  expect(clips.clips.find((c) => c.name === "death")?.loop).toBe(false);

  const tank = await decoded(result.runtime.appearances.tank.bundle);
  if (tank.kind !== "articulated") throw new Error(tank.kind);
  const muzzle = tank.nodes.find((n) => n.name === "muzzle")!;
  expect(muzzle.pivot[0]).toBeCloseTo(3, 5);
  expect(muzzle.pivot[2]).toBeCloseTo(2, 5);
  const track = tank.nodes.find((n) => n.name === "track_L")!.extras;
  expect(track.link_pitch_m).toBeCloseTo(0.16, 6);
  expect(track.track_length_m).toBeCloseTo(14.2, 5);
  // Frames are engine-aligned: the turret yaws about its own +Z.
  expect(tank.nodes.find((n) => n.name === "turret")!.bind.r.map((x) => Math.abs(x))).toEqual([
    0, 0, 0, 1,
  ]);

  const crate = await decoded(result.runtime.appearances.crate.bundle);
  if (crate.kind !== "static") throw new Error(crate.kind);
  expect(crate.states.map((s) => [s.name, s.bounds.max[2]])).toEqual([["default", 6]]);
});

/** The test catalog with the crate drawn by `source`: its bundle, read back
 *  from the bake's files through the one transport. */
async function bakedCrate(source: Uint8Array) {
  const sources: Record<string, Uint8Array> = {
    ...testSources(),
    "assets/source/test-crate.glb": source,
  };
  const result = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  const errors = result.reports.flatMap((r) => r.findings.filter((f) => f.severity === "error"));
  expect(errors).toEqual([]);
  const crate = await bakedBundle(result, result.runtime.appearances.crate.bundle);
  if (crate.kind !== "static") throw new Error(crate.kind);
  return crate;
}

test("a static prop naming _LOD4 bakes a fifth, far mesh of only those meshes, through the transport", async () => {
  const crate = await bakedCrate(farPropGlb(1));
  const [state] = crate.states;
  expect(state.tiers).toHaveLength(5);
  const far = state.tiers[4];
  // One slab: the lid, unsuffixed, is in tiers 0 to 3 and never the far mesh.
  expect(far.indices.length / 3).toBe(12);
  const bounds = positionsBounds(far.positions);
  [bounds.min, bounds.max].forEach((corner, i) =>
    corner.forEach((v, axis) => expect(v).toBeCloseTo(FAR_SLAB[i][axis], 5)),
  );
  // Tier 3 is its one wall box and the lid; the slab joined no tier.
  expect(state.tiers[3].indices.length / 3).toBe(24);
});

test("the same prop naming no _LOD4 bakes its four tiers to the bytes the far mesh leaves alone", async () => {
  const plain = await bakedCrate(farPropGlb());
  expect(plain.states.map((s) => s.tiers.length)).toEqual([4]);
  // Dropping the far mesh from the far-carrying bundle gives the plain
  // bundle's exact encoding: the far mesh touches nothing else.
  const far = await bakedCrate(farPropGlb(1));
  const without = { ...far, states: far.states.map((s) => ({ ...s, tiers: s.tiers.slice(0, 4) })) };
  expect(encodeBundle(without)).toEqual(encodeBundle(plain));
});

/** Each appearance's error codes, baking `catalog` (the test catalog, by
 *  default) over the test sources with `sources` in their place. */
async function refusals(sources: Record<string, Uint8Array>, catalog: Catalog = testCatalog()) {
  const all = { ...testSources(), ...sources };
  const result = await bakeCatalog(catalog, async (path) => all[path], { authority: AUTHORITY });
  const codes: Record<string, FindingCode[]> = {};
  for (const r of result.reports)
    codes[r.name] = r.findings.filter((f) => f.severity === "error").map((f) => f.code);
  return codes;
}

test("only a static prop carries a far mesh: a body, a vehicle, a kit's module and a tree naming _LOD4 are refused", async () => {
  const farred = (path: string) => ({ [path]: withFarCopy(testSources()[path]) });
  const tier = ["structure.tier_count"];
  expect((await refusals(farred("assets/source/test-rifleman.glb"))).rifleman).toEqual(tier);
  expect((await refusals(farred("assets/source/test-tank.glb"))).tank).toEqual(tier);
  const kitSources = citySources(testSet(), withFarCopy(kitGlb(MODULES)));
  const city = await bakeCatalog(cityCatalog(), async (path) => kitSources[path], cityContext());
  const kit = city.reports.find((r) => r.name === KIT)!;
  expect(kit.findings.map((f) => f.code)).toEqual(tier);
  const catalog = testCatalog();
  catalog.appearances.oak = {
    unit: "scenery",
    scenery: "tree",
    states: { summer: "oak.glb" },
    basis_yaw_deg: 0,
  };
  expect((await refusals({ "oak.glb": withFarCopy(treeGlb()) }, catalog)).oak).toEqual(tier);
});

test("a far mesh draws something, and fewer triangles than tier 3", async () => {
  // The synthetic prop's tier 3 is two boxes: one slab is fewer, two as many.
  const crate = async (far: number) =>
    (await refusals({ "assets/source/test-crate.glb": farPropGlb(far) })).crate;
  expect(await crate(1)).toEqual([]);
  expect(await crate(0)).toEqual(["structure.tier_count"]);
  expect(await crate(2)).toEqual(["structure.tier_order"]);
});

/** `run`, with a scenery kind the test defines named `kind`. */
async function withKind<T>(kind: string, rule: SceneryRule, run: () => Promise<T>): Promise<T> {
  SCENERY_KINDS[kind] = rule;
  try {
    return await run();
  } finally {
    delete SCENERY_KINDS[kind];
  }
}

/** The test catalog with `kiosk`, of scenery `kind`, drawn by `states`
 *  (state: source), on the synthetic prop's box: its error codes. */
async function kiosk(kind: string, states: Record<string, Uint8Array>) {
  const catalog = testCatalog();
  catalog.appearances.kiosk = {
    unit: "scenery",
    scenery: kind,
    states: Object.fromEntries(Object.keys(states).map((s) => [s, `${s}.glb`])),
    basis_yaw_deg: 0,
    footprint_half_m: [5, 4, 3],
  };
  const sources = Object.fromEntries(Object.entries(states).map(([s, b]) => [`${s}.glb`, b]));
  return (await refusals(sources, catalog)).kiosk;
}

test("every state of an appearance carries a far mesh, or none does", async () => {
  const rule: SceneryRule = { states: ["default", "burnt"], footprint: { kind: "prop" } };
  await withKind("test_burning", rule, async () => {
    const codes = (a: Uint8Array, b: Uint8Array) => kiosk("test_burning", { default: a, burnt: b });
    expect(await codes(farPropGlb(1), farPropGlb(1))).toEqual([]);
    expect(await codes(farPropGlb(), farPropGlb())).toEqual([]);
    expect(await codes(farPropGlb(1), farPropGlb())).toEqual(["structure.tier_count"]);
    expect(await codes(farPropGlb(), farPropGlb(1))).toEqual(["structure.tier_count"]);
  });
});

test("a kind that budgets its tiers budgets its far mesh too", async () => {
  // The synthetic prop draws 60, 48, 36 and 24 triangles, and 12 per slab.
  const budgeted = async (budget: SceneryRule["tier_triangles"], far?: number) =>
    withKind(
      "test_budgeted",
      { states: ["default"], footprint: { kind: "prop" }, tier_triangles: budget },
      () => kiosk("test_budgeted", { default: farPropGlb(far) }),
    );
  expect(await budgeted([60, 48, 36, 24, 12], 1)).toEqual([]);
  expect(await budgeted([60, 48, 36, 24, 11], 1)).toEqual(["budget.tier_triangles"]);
  // Four entries budget a prop that names no far mesh, and refuse one that does.
  expect(await budgeted([60, 48, 36, 24])).toEqual([]);
  expect(await budgeted([60, 48, 36, 24], 1)).toEqual(["budget.tier_triangles"]);
});
