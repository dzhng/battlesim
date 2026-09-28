// @vitest-environment node
// The vehicle rig seam, end to end: what the simulation published about a
// vehicle (its heading, each mount's weapon pose, its motion and deployment)
// through the pose driver and the articulation onto the bundle's nodes. The
// gun and HMG point where the published poses say, elevation eases in, a
// shot recoils the gun, and the running gear rolls with ground covered.
import { expect, test } from "vitest";
import { vec3, type Vec3 } from "math";
import {
  PITCH_LIMITS,
  articulate,
  articulationRig,
  restLocals,
  type Articulation,
} from "@packages/scene-assets/src/articulation.ts";
import { worldTransforms } from "@packages/scene-assets/src/pose.ts";
import { pointAt } from "@packages/scene-assets/src/trs.ts";
import type { ArticulatedBundle } from "@packages/scene-assets/src/schema.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import {
  MOUNT_FEEL,
  PoseDriver,
  type FeedMount,
  type FeedUnit,
} from "@packages/battle-renderer/src/models/poseDriver.ts";
import { AUTHORITY, TOLERANCES, tankGlb, tankMounts } from "./sceneAssets/synthetic";

const bundle = (async () => {
  const result = await validateAppearance(
    {
      name: "tank",
      entry: { unit: "tank", source: "a.glb", basis_yaw_deg: 0 },
      files: { "a.glb": tankGlb({ muzzleX: 5.9 }) },
    },
    {
      authority: { ...AUTHORITY, mounts: { ...AUTHORITY.mounts, tank: tankMounts(5.9) } },
      tolerances: TOLERANCES,
    },
  );
  return result.preview as ArticulatedBundle;
})();

const driver = () =>
  new PoseDriver({
    mounts: { tank: ["gun", "hmg"] },
    clip: () => null,
    halfTrack: { tank: 1.4 },
    pinned: 0.6,
  });

const tank = (x: number, yaw: number, gun: FeedMount, hmg: FeedMount): FeedUnit => ({
  id: 7,
  kind: "tank",
  side: "blue",
  position: [x, 0, 0],
  yaw,
  soldiers: [],
  mounts: [gun, hmg],
  deployment: null,
  suppression: 0,
});

/** World points of the posed tank's named nodes, the hull turned by `yaw`. */
async function posedWorld(articulation: Articulation, yaw: number) {
  const b = await bundle;
  const nodes = b.nodes;
  const locals = articulate(restLocals(nodes), nodes, articulationRig(nodes), articulation);
  const worlds = worldTransforms(
    nodes.map((n) => n.parent),
    locals,
  );
  return (name: string): Vec3 => {
    const local = pointAt(worlds[nodes.findIndex((n) => n.name === name)], [0, 0, 0]);
    return vec3.rotateZ(local, local, [0, 0, 0], yaw);
  };
}

/** Bearing and elevation of the ray from `a` to `b`. */
function aim(a: Vec3, b: Vec3) {
  const d = vec3.sub(vec3.create(), b, a);
  return { bearing: Math.atan2(d[1], d[0]), elevation: Math.atan2(d[2], Math.hypot(d[0], d[1])) };
}

test("the gun and HMG point along their published bearings and elevations", async () => {
  const yaw = 0.8;
  const gun = { bearing: 2.1, elevation: 0.05, shots: 0 };
  const hmg = { bearing: -0.4, elevation: 0.3, shots: 0 };
  const pose = driver().update({ time: 0, units: [tank(0, yaw, gun, hmg)], fallen: [] })
    .vehicles[0];
  const at = await posedWorld(pose.articulation, yaw);
  const cannon = aim(at("gun"), at("muzzle"));
  expect(cannon.bearing).toBeCloseTo(gun.bearing, 5);
  expect(cannon.elevation).toBeCloseTo(gun.elevation, 5);
  const mg = aim(at("hmg_gun"), at("hmg_muzzle"));
  expect(mg.bearing).toBeCloseTo(hmg.bearing, 5);
  expect(mg.elevation).toBeCloseTo(hmg.elevation, 5);
});

test("a new published elevation is eased to at the gun's rate, never snapped", () => {
  const d = driver();
  const hmg = { bearing: 0, elevation: 0, shots: 0 };
  d.update({
    time: 0,
    units: [tank(0, 0, { bearing: 0, elevation: 0, shots: 0 }, hmg)],
    fallen: [],
  });
  // The round that raised the published elevation.
  const raised = { bearing: 0, elevation: 0.15, shots: 1 };
  const pitchAt = (time: number) =>
    d.update({ time, units: [tank(0, 0, raised, hmg)], fallen: [] }).vehicles[0].articulation
      .gun_pitch;
  expect(pitchAt(0.1)).toBeCloseTo(MOUNT_FEEL.gunElevationRate * 0.1, 6);
  expect(pitchAt(0.2)).toBeCloseTo(MOUNT_FEEL.gunElevationRate * 0.2, 6);
  expect(pitchAt(2)).toBeCloseTo(0.15, 6);
  // And never past the gun's presentation limit.
  const high = { ...raised, elevation: 1 };
  const limited = d.update({ time: 10, units: [tank(0, 0, high, hmg)], fallen: [] }).vehicles[0]
    .articulation.gun_pitch;
  expect(limited).toBeCloseTo(PITCH_LIMITS.gun[1], 6);
});

test("each rise of the gun's shot counter recoils it, and it runs out to battery", () => {
  const d = driver();
  const hmg = { bearing: 0, elevation: 0, shots: 0 };
  const recoil = (time: number, shots: number) =>
    d.update({ time, units: [tank(0, 0, { bearing: 0, elevation: 0, shots }, hmg)], fallen: [] })
      .vehicles[0].articulation.recoil;
  // Seen with rounds already fired: nothing to recoil from.
  expect(recoil(0, 4)).toBe(0);
  expect(recoil(1, 4)).toBe(0);
  expect(recoil(2, 5)).toBeCloseTo(MOUNT_FEEL.recoilM, 6);
  const early = recoil(2.1, 5);
  const late = recoil(2.5, 5);
  expect(early).toBeGreaterThan(late);
  expect(late).toBeGreaterThan(0);
  expect(recoil(2 + MOUNT_FEEL.recoilReturnS + 0.01, 5)).toBe(0);
  // HMG bursts never move the cannon.
  const burst = d.update({
    time: 4,
    units: [tank(0, 0, { bearing: 0, elevation: 0, shots: 5 }, { ...hmg, shots: 30 })],
    fallen: [],
  });
  expect(burst.vehicles[0].articulation.recoil).toBe(0);
});

test("driving forward rolls every wheel by ground covered over its radius", async () => {
  const d = driver();
  const still = { bearing: 0, elevation: 0, shots: 0 };
  d.update({ time: 0, units: [tank(0, 0, still, still)], fallen: [] });
  const a = d.update({ time: 1, units: [tank(3, 0, still, still)], fallen: [] }).vehicles[0]
    .articulation;
  const b = await bundle;
  const rig = articulationRig(b.nodes);
  const locals = articulate(restLocals(b.nodes), b.nodes, rig, a);
  for (const wheel of rig.wheels) {
    const turn = 2 * Math.acos(Math.min(1, Math.abs(locals[wheel.node].r[3])));
    const expected = (3 / wheel.radius) % (2 * Math.PI);
    expect(Math.min(turn, 2 * Math.PI - turn)).toBeCloseTo(
      Math.min(expected, 2 * Math.PI - expected),
      4,
    );
  }
});
