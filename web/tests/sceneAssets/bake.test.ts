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
import { FAR_SLAB, farPropGlb, postsGlb, tinyPropGlb, twoToneGlb, withFarCopy } from "./farProp";

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

/** The test catalog with the crate drawn by `source` on a box of `half`
 *  extents: its bundle, read back from the bake's files through the one
 *  transport. */
async function bakedCrate(source: Uint8Array, half?: [number, number, number]) {
  const sources: Record<string, Uint8Array> = {
    ...testSources(),
    "assets/source/test-crate.glb": source,
  };
  const catalog = testCatalog();
  if (half) catalog.appearances.crate.footprint_half_m = half;
  const result = await bakeCatalog(catalog, async (path) => sources[path], {
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

test("a far mesh, generated or authored, leaves the prop's tiers 0 to 3 as they were", async () => {
  const four = (b: Awaited<ReturnType<typeof bakedCrate>>) =>
    encodeBundle({ ...b, states: b.states.map((s) => ({ ...s, tiers: s.tiers.slice(0, 4) })) });
  const generated = await bakedCrate(farPropGlb());
  const authored = await bakedCrate(farPropGlb(1));
  expect(generated.states[0].tiers).toHaveLength(5);
  expect(four(generated)).toEqual(four(authored));
});

/** A baked mesh's faces: each triangle's outward normal axis ("+z", "-x"…),
 *  and the name of the material it draws in. */
function faces(
  mesh: {
    normals: Int16Array;
    indices: ArrayLike<number>;
    draws: { material: number; first: number; count: number }[];
  },
  materials: { name: string }[],
) {
  const out = new Map<string, Set<string>>();
  for (const d of mesh.draws)
    for (let i = d.first; i < d.first + d.count; i += 3) {
      const n = Array.from(mesh.normals.subarray(mesh.indices[i] * 4, mesh.indices[i] * 4 + 3));
      const axis = n.findIndex((x) => Math.abs(x) > 16000);
      const key = `${n[axis] > 0 ? "+" : "-"}${"xyz"[axis]}`;
      if (!out.has(key)) out.set(key, new Set());
      out.get(key)!.add(materials[d.material].name);
    }
  return Object.fromEntries([...out].map(([k, v]) => [k, [...v]]));
}

test("a prop that names no far mesh gets a box: tier 3's height and plan, its top in what shows from above, its sides in what shows from the sides", async () => {
  const block = await bakedCrate(twoToneGlb(), [5, 4, 3.1]);
  const far = block.states[0].tiers[4];
  expect(far.indices.length / 3).toBe(10);
  const b = positionsBounds(far.positions);
  [b.min, b.max].forEach((corner, i) =>
    corner.forEach((v, axis) =>
      expect(v).toBeCloseTo(
        [
          [-5, -4, 0],
          [5, 4, 6.2],
        ][i][axis],
        4,
      ),
    ),
  );
  // No bottom: it stands on the ground.
  expect(faces(far, block.materials)).toEqual({
    "+z": ["roof"],
    "-x": ["paint"],
    "+x": ["paint"],
    "-y": ["paint"],
    "+y": ["paint"],
  });
});

test("an airy prop's box covers about its plan, not its bounds", async () => {
  // Two 0.2 m posts 4 m apart cover 0.08 m² of a 4.2 m square.
  const posts = await bakedCrate(postsGlb(), [2.1, 2.1, 1.5]);
  const b = positionsBounds(posts.states[0].tiers[4].positions);
  const area = (b.max[0] - b.min[0]) * (b.max[1] - b.min[1]);
  expect(area).toBeGreaterThan(0.04);
  expect(area).toBeLessThan(0.16);
  // About where the posts are on average, and as tall as they are.
  expect(Math.abs(b.min[0] + b.max[0])).toBeLessThan(0.2);
  expect(Math.abs(b.min[1] + b.max[1])).toBeLessThan(0.2);
  expect(b.max[2]).toBeCloseTo(3, 4);
});

test("a prop whose tier 3 is already smaller than a box gets no far mesh", async () => {
  const tiny = await bakedCrate(tinyPropGlb());
  expect(tiny.states.map((s) => s.tiers.length)).toEqual([4]);
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
    expect(await codes(tinyPropGlb(), tinyPropGlb())).toEqual([]);
    expect(await codes(farPropGlb(1), tinyPropGlb())).toEqual(["structure.tier_count"]);
    expect(await codes(tinyPropGlb(), farPropGlb(1))).toEqual(["structure.tier_count"]);
    // A generated box gives way where another state can have none.
    expect(await codes(farPropGlb(), tinyPropGlb())).toEqual([]);
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
  // Four entries budget a prop with no far mesh, and refuse one that carries
  // one, authored or generated.
  const tiny = () =>
    withKind(
      "test_budgeted",
      { states: ["default"], footprint: { kind: "prop" }, tier_triangles: [60, 48, 36, 24] },
      () => kiosk("test_budgeted", { default: tinyPropGlb() }),
    );
  expect(await tiny()).toEqual([]);
  expect(await budgeted([60, 48, 36, 24], 1)).toEqual(["budget.tier_triangles"]);
  expect(await budgeted([60, 48, 36, 24])).toEqual(["budget.tier_triangles"]);
});
