// @vitest-environment node
// The bake end to end: a valid catalog bakes to content-addressed bundles,
// deterministically, and the bundles decode to what the art says.
import { expect, test } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { sha256Hex } from "@packages/scene-assets/src/glb.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { AUTHORITY, bakedBundle, testCatalog, testSources } from "./synthetic";

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
