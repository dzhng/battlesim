// @vitest-environment node
// glTF import, engine basis and animated bounds (ported in technique from
// ~/dev/game's soldier-assets gltf, engine-basis and animated-bounds tests).
import { expect, test } from "vitest";
import { buildClips, buildSkinned } from "@packages/scene-assets/src/build.ts";
import { vec3 } from "math";
import { inverse, mul, pointAt } from "@packages/scene-assets/src/trs.ts";
import { sampleClip, skinPositions, worldTransforms } from "@packages/scene-assets/src/pose.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import {
  CHANNEL_ABSENT,
  CHANNEL_ANIMATED,
  CHANNEL_CONSTANT,
} from "@packages/scene-assets/src/schema.ts";
import { animatedBounds } from "@packages/scene-assets/src/validate.ts";
import { SKELETON_ENTRY, soldierGlb } from "./synthetic";

function bakeSoldier(yaw: number) {
  const bytes = soldierGlb();
  const { scene } = importScene(bytes, "s.glb", yaw);
  const clips = buildClips(scene!, "s.glb", "rig", 30, SKELETON_ENTRY.clips).built!;
  const body = buildSkinned(scene!, "s.glb", clips.joints).built!;
  return { scene: scene!, clips, body };
}

function posed(yaw: number, clipName: string | null, phase = 0) {
  const { clips, body } = bakeSoldier(yaw);
  const clip = clips.clips.find((c) => c.name === clipName);
  const locals = clip
    ? sampleClip(clips, clip, body.joints, phase)
    : body.joints.map((j) => j.bind);
  const worlds = worldTransforms(
    body.joints.map((j) => j.parent),
    locals,
  );
  const socket = (name: string) => {
    const s = body.sockets.find((x) => x.name === name)!;
    return pointAt(worlds[s.joint], s.offset.t);
  };
  return {
    positions: skinPositions(body.tiers[0], body.joints, worlds),
    socket,
    body,
    clips,
    worlds,
  };
}

test("glTF Y-up facing +Z becomes engine Z-up facing +X under a 90° yaw", () => {
  const { socket } = posed(90, null);
  const muzzle = socket("muzzle");
  const eye = socket("eye");
  // Authored: muzzle 0.7 m ahead (+Z) at 1.4 m, 0.2 m to the soldier's right (−X).
  expect(muzzle[0]).toBeCloseTo(0.7, 5);
  expect(muzzle[1]).toBeCloseTo(-0.2, 5); // right is −Y in engine space
  expect(muzzle[2]).toBeCloseTo(1.4, 5);
  expect(eye[2]).toBeCloseTo(1.6, 5);
});

test("the basis turns bind, inverse binds and clip tracks together: yaw only rotates the posed body", () => {
  for (const [clip, phase] of [
    ["walk", 0.3],
    ["death", 0.7],
  ] as const) {
    const a = posed(0, clip, phase).positions;
    const b = posed(90, clip, phase).positions;
    for (let v = 0; v < a.length; v += 3) {
      // +90° about Z: (x, y) → (−y, x)
      expect(b[v]).toBeCloseTo(-a[v + 1], 4);
      expect(b[v + 1]).toBeCloseTo(a[v], 4);
      expect(b[v + 2]).toBeCloseTo(a[v + 2], 4);
    }
  }
});

test("rigid kit parented to a bone follows that bone, skinned at bake", () => {
  const rest = posed(90, null);
  const aim = posed(90, "death", 1);
  // The rifle is the only geometry skinned wholly to hand_r.
  const hand = rest.body.joints.findIndex((j) => j.name === "hand_r");
  const mesh = rest.body.tiers[0];
  const rifle: number[] = [];
  for (let v = 0; v < mesh.positions.length / 3; v++)
    if (mesh.joints![v * 4] === hand && mesh.weights![v * 4] === 65535) rifle.push(v);
  expect(rifle.length).toBe(24);
  // At rest the rifle sits where the source placed it: along +X at 1.37–1.43 m.
  const restZ = rifle.map((v) => rest.positions[v * 3 + 2]);
  expect(Math.min(...restZ)).toBeCloseTo(1.37, 4);
  expect(Math.max(...restZ)).toBeCloseTo(1.43, 4);
  // Dead, every rifle vertex moves exactly as the hand does.
  const follow = mul(aim.worlds[hand], inverse(rest.worlds[hand]));
  for (const v of rifle) {
    const expected = pointAt(follow, vec3.fromBuffer(vec3.create(), rest.positions, v * 3));
    for (let c = 0; c < 3; c++) expect(aim.positions[v * 3 + c]).toBeCloseTo(expected[c], 4);
  }
  expect(Math.min(...rifle.map((v) => aim.positions[v * 3 + 2]))).toBeLessThan(0.5);
});

test("a body keeps an unweighted leaf joint when its skeleton has it, and drops it otherwise", () => {
  const { scene, clips } = bakeSoldier(90);
  expect(clips.joints.some((j) => j.name === "index_04_leaf_r")).toBe(false);
  const hand = clips.joints.findIndex((j) => j.name === "hand_r");
  const withLeaf = [...clips.joints, { name: "index_04_leaf_r", parent: hand }];
  const built = buildSkinned(scene, "s.glb", withLeaf);
  expect(built.findings).toEqual([]);
  expect(built.built!.joints.map((j) => j.name)).toEqual(withLeaf.map((j) => j.name));
});

test("clip channels are absent, constant or animated per joint", () => {
  const { clips } = bakeSoldier(90);
  const joint = (name: string) => clips.joints.findIndex((j) => j.name === name);
  const walk = clips.clips.find((c) => c.name === "walk")!;
  const aim = clips.clips.find((c) => c.name === "stand_aim")!;
  expect(walk.rotation_modes[joint("spine_01")]).toBe(CHANNEL_ANIMATED);
  expect(walk.rotation_modes[joint("hand_r")]).toBe(CHANNEL_ABSENT);
  expect(aim.rotation_modes[joint("spine_01")]).toBe(CHANNEL_CONSTANT);
  expect(walk.frames).toBe(31); // 1 s at 30 Hz, both ends
  expect(aim.frames).toBe(1);
});

test("animated bounds hold every pose, including between baked frames", () => {
  const { body, clips } = bakeSoldier(90);
  const bounds = animatedBounds(body.joints, body.tiers, clips);
  const parents = body.joints.map((j) => j.parent);
  const reached = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] };
  for (const clip of clips.clips)
    for (let k = 0; k <= 4 * clip.frames; k++) {
      const worlds = worldTransforms(
        parents,
        sampleClip(clips, clip, body.joints, k / (4 * clip.frames)),
      );
      const p = skinPositions(body.tiers[0], body.joints, worlds);
      for (let v = 0; v < p.length; v += 3)
        for (let c = 0; c < 3; c++) {
          reached.min[c] = Math.min(reached.min[c], p[v + c]);
          reached.max[c] = Math.max(reached.max[c], p[v + c]);
        }
    }
  // Every sampled vertex lies inside (1 mm: slerp between baked frames is not
  // the chord, but at 30 Hz the bulge is far below it).
  for (let c = 0; c < 3; c++) {
    expect(reached.min[c]).toBeGreaterThanOrEqual(bounds.min[c] - 1e-3);
    expect(reached.max[c]).toBeLessThanOrEqual(bounds.max[c] + 1e-3);
  }
  // The fallen body spans about its height along X; the bounds are tight, not padded.
  expect(bounds.max[0] - bounds.min[0]).toBeGreaterThan(1.5);
  for (let c = 0; c < 3; c++) {
    expect(reached.min[c] - bounds.min[c]).toBeLessThan(0.01);
    expect(bounds.max[c] - reached.max[c]).toBeLessThan(0.01);
  }
});
