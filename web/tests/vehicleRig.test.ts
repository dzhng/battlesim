// @vitest-environment node
// The vehicle rig seam, end to end: what the simulation published about a
// vehicle (its heading, each mount's weapon pose, its motion and deployment)
// through the pose driver and the articulation onto the bundle's nodes. The
// gun and HMG point where the published poses say, elevation eases in, a
// shot recoils the gun, and the running gear rolls with ground covered.
import game from "@fixtures/game.json";
import { expect, test } from "vitest";
import { mat4, vec3, type Mat4, type Vec3 } from "math";
import {
  DEFAULT_PITCH_LIMITS,
  REST_ARTICULATION,
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
  PoseDriver,
  type FeedMount,
  type FeedUnit,
  type PoseFeel,
} from "@packages/battle-renderer/src/models/poseDriver.ts";
import { DrawnModels } from "@packages/battle-renderer/src/models/drawnModels.ts";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";
import { gamePose } from "@apps/battle-lab/src/poseFeed";
import { UNITS } from "./catalog";
import { shippedMounts } from "./shippedMounts";
import {
  AUTHORITY,
  TOLERANCES,
  syntheticUnits,
  tankGlb,
  tankMounts,
  TANK_DRAWS,
} from "./sceneAssets/synthetic";

const bundle = (async () => {
  const result = await validateAppearance(
    {
      name: "test_tank",
      entry: { unit: "vehicle", source: "a.glb", basis_yaw_deg: 0, mounts: TANK_DRAWS },
      files: { "a.glb": tankGlb({ muzzleX: 5.9 }) },
    },
    {
      authority: { ...AUTHORITY, units: syntheticUnits({ mounts: tankMounts(5.9) }) },
      tolerances: TOLERANCES,
    },
  );
  return result.preview as ArticulatedBundle;
})();

const driver = () =>
  new PoseDriver({
    units: UNITS,
    mounts: shippedMounts,
    clip: () => null,
    feel: gamePose,
    leanHold: game.cover.lean_hold_s,
  });

const MOUNT_FEEL = gamePose.mount;

const tank = (x: number, yaw: number, gun: FeedMount, hmg: FeedMount): FeedUnit => ({
  id: 7,
  kind: "test_tank",
  side: "blue",
  position: [x, 0, 0],
  yaw,
  soldiers: [],
  mounts: [gun, hmg],
  deployment: null,
  pinned: false,
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

test("a new published elevation is eased to at the gun's rate, never snapped", async () => {
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
  expect(pitchAt(0.1)).toBeCloseTo(MOUNT_FEEL.gun_elevation_rad_s * 0.1, 6);
  expect(pitchAt(0.2)).toBeCloseTo(MOUNT_FEEL.gun_elevation_rad_s * 0.2, 6);
  expect(pitchAt(2)).toBeCloseTo(0.15, 6);
  // And the drawn gun never goes past its model's pitch limit.
  const high = { ...raised, elevation: 1 };
  const posed = d.update({ time: 10, units: [tank(0, 0, high, hmg)], fallen: [] }).vehicles[0];
  const at = await posedWorld(posed.articulation, 0);
  expect(aim(at("gun"), at("muzzle")).elevation).toBeCloseTo(DEFAULT_PITCH_LIMITS.gun[1], 6);
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
  expect(recoil(2, 5)).toBeCloseTo(MOUNT_FEEL.recoil_m, 6);
  const early = recoil(2.1, 5);
  const late = recoil(2.5, 5);
  expect(early).toBeGreaterThan(late);
  expect(late).toBeGreaterThan(0);
  expect(recoil(2 + MOUNT_FEEL.recoil_return_s + 0.01, 5)).toBe(0);
  // HMG bursts never move the cannon.
  const burst = d.update({
    time: 4,
    units: [tank(0, 0, { bearing: 0, elevation: 0, shots: 5 }, { ...hmg, shots: 30 })],
    fallen: [],
  });
  expect(burst.vehicles[0].articulation.recoil).toBe(0);
});

/** A kick feel of the test's own: a main-gun round kicks fully, an HMG's not at all. */
const KICK: PoseFeel["hull_kick"] = {
  rounds: { default: 0, tank_ap: 1 },
  rock_deg: 4,
  slide_m: 0.2,
  hz: 2,
  damping: 0.3,
};

/** The tank's drawn motion through `seconds` at `hz` frames a second, firing
 *  its gun along `bearing` (and its HMG `hmgShots` times) at 1 s. */
function kicked(bearing: number, seconds: number, hz = 60, gunShots = 1, hmgShots = 0) {
  const d = new PoseDriver({
    units: UNITS,
    mounts: shippedMounts,
    clip: () => null,
    feel: { ...gamePose, hull_kick: KICK },
    leanHold: game.cover.lean_hold_s,
  });
  const at = (time: number) => {
    const fired = time >= 1;
    const gun = { bearing, elevation: 0, shots: fired ? gunShots : 0 };
    const hmg = { bearing, elevation: 0, shots: fired ? hmgShots : 0 };
    return d.update({ time, units: [tank(0, 0, gun, hmg)], fallen: [] }).vehicles[0].motion;
  };
  const frames: { time: number; motion: Mat4 | null }[] = [];
  for (let k = 0; k <= seconds * hz; k++) {
    const m = at(k / hz);
    frames.push({ time: k / hz, motion: m && mat4.clone(m) });
  }
  return frames;
}

/** Where the hull's point `p` (its own frame) is drawn under `motion`. */
const moved = (motion: Mat4 | null, p: Vec3): Vec3 =>
  motion ? vec3.transformMat4(vec3.create(), p, motion) : vec3.clone(p);

test("a main-gun shot rocks the hull away from its target on its springs, then it settles", () => {
  const frames = kicked(0, 6);
  // Still until it fires.
  expect(frames.filter((f) => f.time < 1).every((f) => f.motion === null)).toBe(true);
  const roof: Vec3 = [0, 0, 2];
  const back = frames.map((f) => -moved(f.motion, roof)[0]);
  const nose = frames.map((f) => moved(f.motion, [4, 0, 0])[2]);
  // Fired forward, the roof is thrown back and the nose lifts at once…
  const peak = back.indexOf(Math.max(...back));
  expect(frames[peak].time - 1).toBeLessThan(0.25);
  expect(nose[peak]).toBeGreaterThan(0);
  // …at most the full kick's rock (and slide) away from rest.
  const rock = (KICK.rock_deg * Math.PI) / 180;
  expect(back[peak]).toBeGreaterThan(0.8 * (2 * Math.sin(rock) + KICK.slide_m));
  expect(back[peak]).toBeLessThanOrEqual(2 * Math.sin(rock) + KICK.slide_m + 1e-6);
  // The springs throw it back past rest, less far, before it settles.
  const after = back.slice(peak);
  expect(Math.min(...after)).toBeLessThan(-0.1 * back[peak]);
  expect(-Math.min(...after)).toBeLessThan(back[peak]);
  // And it settles: drawn as it stood, with no motion left to draw.
  expect(frames.at(-1)!.motion).toBeNull();
  // It never sideslips firing straight ahead.
  for (const f of frames) expect(Math.abs(moved(f.motion, roof)[1])).toBeLessThan(1e-9);
});

test("a shot over the side rocks the hull away from that side", () => {
  // Fired to the left (+Y): the roof is thrown to the right.
  const frames = kicked(Math.PI / 2, 2);
  const right = frames.map((f) => -moved(f.motion, [0, 0, 2])[1]);
  // Its widest swing, the first, is to the right.
  const widest = right.reduce((w, r) => (Math.abs(r) > Math.abs(w) ? r : w), 0);
  expect(widest).toBeGreaterThan(0.1);
  expect(Math.max(...frames.map((f) => Math.abs(moved(f.motion, [0, 0, 2])[0])))).toBeLessThan(
    1e-9,
  );
});

test("a round with no kick never moves the hull, however many fly", () => {
  const frames = kicked(0, 3, 60, 0, 40);
  expect(frames.every((f) => f.motion === null)).toBe(true);
});

test("the drawn muzzle rides the rocking hull, so the flash stays on the barrel", async () => {
  const b = await bundle;
  const units = syntheticUnits({ mounts: tankMounts(5.9) });
  const drawn = new DrawnModels(
    {
      appearances: new Map([["test_tank", { bundle: b, mounts: TANK_DRAWS }]]),
    } as unknown as InstalledAppearances,
    () => ({ appearance: "test_tank", tint: [1, 1, 1] }),
    units,
  );
  const frames = kicked(0, 1.5);
  const motion = frames.find((f) => f.time >= 1.15)!.motion!;
  const muzzle = (m: Mat4 | null) => {
    const v = { unit: 7, kind: "tank", side: "blue" as const, position: [10, 0, 0] as Vec3 };
    drawn.update({
      soldiers: [],
      vehicles: [
        { ...v, yaw: 0, airborne: false, articulation: { ...REST_ARTICULATION }, motion: m },
      ],
      corpses: [],
      corpsesVersion: 0,
      fading: [],
    });
    const at = vec3.create();
    expect(drawn.vehicle("blue", 7, 0, at)).toBe(true);
    return at;
  };
  const still = muzzle(null);
  const rocking = muzzle(motion);
  // The hull's motion carries the muzzle about the hull's foot.
  const expected = vec3.add(
    vec3.create(),
    moved(motion, vec3.sub(vec3.create(), still, [10, 0, 0])),
    [10, 0, 0],
  );
  for (let i = 0; i < 3; i++) expect(rocking[i]).toBeCloseTo(expected[i], 6);
  expect(rocking[2]).toBeGreaterThan(still[2] + 0.05);
});

test("the hull rocks the same however often it is drawn", () => {
  const fine = kicked(0, 3, 120);
  const coarse = kicked(0, 3, 8);
  for (const c of coarse) {
    const f = fine.find((x) => Math.abs(x.time - c.time) < 1e-9)!;
    const [a, b] = [moved(c.motion, [0, 0, 2]), moved(f.motion, [0, 0, 2])];
    for (let i = 0; i < 3; i++) expect(a[i]).toBeCloseTo(b[i], 9);
  }
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
