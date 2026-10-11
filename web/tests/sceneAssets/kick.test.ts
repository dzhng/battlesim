// @vitest-environment node
// A shot's kick on a soldier's body: his upper body, and everything it
// holds, turns back about his waist, the muzzle climbing and the head going
// back, while his hips and legs stay where the clip has them.
import { expect, test } from "vitest";
import { bakeCatalog } from "@packages/scene-assets/src/bake.ts";
import { poseWorlds } from "@packages/scene-assets/src/pose.ts";
import { mul, trsMatrix } from "@packages/scene-assets/src/trs.ts";
import type { SkeletonClips, SkinnedBundle } from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, bakedBundle, testCatalog, testSources } from "./synthetic";

async function rifleman() {
  const sources = testSources();
  const result = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  const body = (await bakedBundle(
    result,
    result.runtime.appearances.rifleman.bundle,
  )) as SkinnedBundle;
  const clips = (await bakedBundle(result, result.runtime.skeletons["test-rig"])) as SkeletonClips;
  return { body, clips };
}

test("a kick turns his upper body back about his waist: the muzzle climbs, the head goes back, the hips stay", async () => {
  const { body, clips } = await rifleman();
  const socket = body.sockets.find((s) => s.name === "muzzle")!;
  const joint = (name: string) => body.joints.findIndex((j) => j.name === name);
  const at = (kick?: { pitch: number; back: number }) => {
    const worlds = poseWorlds(body, clips, { clip: "stand_aim", phase: 0, kick });
    const muzzle = mul(worlds[socket.joint], trsMatrix(socket.offset));
    const point = (m: number[] | Float32Array) => [m[12], m[13], m[14]];
    return {
      muzzle: point(muzzle),
      head: point(worlds[joint("Head")]),
      pelvis: point(worlds[joint("pelvis")]),
    };
  };
  const rest = at();
  expect(at({ pitch: 0, back: 0 })).toEqual(rest);
  const kicked = at({ pitch: 0.15, back: 0.05 });
  // The muzzle (ahead of the waist) climbs; the head (above it) goes back.
  expect(kicked.muzzle[2]).toBeGreaterThan(rest.muzzle[2] + 0.02);
  expect(kicked.head[0]).toBeLessThan(rest.head[0] - 0.05);
  // Nothing turns sideways, and the hips and legs never move.
  expect(kicked.muzzle[1]).toBeCloseTo(rest.muzzle[1], 6);
  expect(kicked.pelvis).toEqual(rest.pelvis);
  // Thrown back alone, every carried point moves straight back by it.
  const shoved = at({ pitch: 0, back: 0.05 });
  for (const p of ["muzzle", "head"] as const) {
    expect(shoved[p][0]).toBeCloseTo(rest[p][0] - 0.05, 6);
    expect(shoved[p][2]).toBeCloseTo(rest[p][2], 6);
  }
});
