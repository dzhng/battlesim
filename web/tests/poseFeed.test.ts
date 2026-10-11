// @vitest-environment node
// The battle's pose seam on packed frames: decoded observations, blended by
// the presentation clock (`TickInterpolator`), fed per soldier by member id
// (`ObservationFeed`) to the pose driver. What the side published decides
// every pose; nothing reads a formation slot or the squad's heading.
import game from "@fixtures/game.json";
import { expect, test } from "vitest";
import { ObservationFeed, gamePose } from "@apps/battle-lab/src/poseFeed";
import { drawnMuzzleSource, effectPublication } from "@apps/battle-lab/src/effectFeed";
import { UNITS } from "./catalog";
import { UnitCatalog } from "@packages/scene-assets/src/units";
import { shippedMounts } from "./shippedMounts";
import { LaunchTracker } from "@packages/battle-renderer/src/effects/launches";
import {
  PoseDriver,
  stanceManner,
  type PoseFrame,
} from "@packages/battle-renderer/src/models/poseDriver";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { poseFrameInstances } from "@packages/battle-renderer/src/models/modelInstances";
import { DrawnModels } from "@packages/battle-renderer/src/models/drawnModels";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader";
import { AUTHORITY, testCatalog, testSources } from "./sceneAssets/synthetic";
import { sideKey } from "@packages/battle-renderer/src/sideKey";
import { TickInterpolator } from "../src/battle/present/interpolate";
import type {
  CorpseView,
  FallingAirframeView,
  IdentifiedView,
  ObservationView,
  OwnUnitView,
  Point2,
  Point3,
  ProjectileView,
  SuppressionTier,
} from "../src/battle/sim/observation";

const TICK_MS = 1000 / 30;
const CLIPS: Record<string, { duration: number; loop: boolean; stride_m: number | null }> = {
  idle: { duration: 2, loop: true, stride_m: null },
  walk: { duration: 1, loop: true, stride_m: 1.25 },
  run: { duration: 0.8, loop: true, stride_m: 2.7 },
  kneel_fire: { duration: 1, loop: true, stride_m: null },
  prone_pinned: { duration: 2, loop: true, stride_m: null },
  death: { duration: 1, loop: false, stride_m: null },
};

interface Soldier {
  id: number;
  at: Point3;
  /** Where he stands out on his lean, if he leans. */
  lean?: Point2;
}

const squad = (
  id: number,
  soldiers: Soldier[],
  { shots = 0, suppression = "none" as SuppressionTier, bearing = 0 } = {},
): OwnUnitView => ({
  id,
  kind: "test_rifle",
  position: soldiers[0]?.at ?? [0, 0, 0],
  yaw: Math.PI / 2,
  goal: null,
  policy: null,
  direction: null,
  reversing: false,
  withdrawing: false,
  protection: null,
  state: "idle",
  blocker: null,
  route: [],
  queue: [],
  members: soldiers.map((s) => s.at),
  memberIds: soldiers.map((s) => s.id),
  memberSlots: soldiers.map(() => 0),
  memberActiveMounts: soldiers.map(() => 0),
  memberOrders: [],
  memberLeans: soldiers.map((s) => (s.lean ? { side: "left", at: s.lean } : null)),
  area: null,
  finalFacing: 0,
  sees: [],
  engagement: "fire_at_will",
  mounts: [],
  weaponPoses: [{ mount: 0, operator: null, bearing, elevation: 0, shots }],
  hp: 100,
  smoking: false,
  memberHp: soldiers.map(() => 10),
  suppression,
  concealed: false,
  deployment: null,
  garrison: null,
  stock: null,
  service: "out_of_range",
  sight: { eyes: [], forward: 0, shape: { front: 1, side: 1, rear: 1 }, range: 350 },
});

const enemy = (id: number, soldiers: Soldier[], shots = 0): IdentifiedView => ({
  id,
  kind: "test_rifle",
  cost: 1,
  position: soldiers[0]?.at ?? [0, 0, 0],
  yaw: 0,
  velocity: [0, 0],
  members: soldiers.map((s) => s.at),
  memberIds: soldiers.map((s) => s.id),
  memberSlots: soldiers.map(() => 0),
  memberActiveMounts: soldiers.map(() => 0),
  memberLeans: soldiers.map((s) => (s.lean ? { side: "right", at: s.lean } : null)),
  weaponPoses: [{ mount: 0, operator: null, bearing: Math.PI, elevation: 0, shots }],
  reversing: false,
  smoking: false,
});

/** A stretch of soldier `shooter`'s round this tick, from `from` 5 m east. */
const round = (shooter: number, from: Point3 = [0, 0, 1]): ProjectileView => ({
  path: [from, [from[0] + 5, from[1], from[2]]],
  ricochets: [],
  own: true,
  kind: "rifle",
  shooterMember: shooter,
  hit: "none",
  impactNormal: null,
});

const observation = (
  tick: number,
  own: OwnUnitView[],
  {
    identified = [] as IdentifiedView[],
    corpses = [] as CorpseView[],
    projectiles = [] as ProjectileView[],
    crashes = [] as FallingAirframeView[],
  } = {},
): ObservationView => ({
  tick,
  own,
  identified,
  contacts: [],
  audible: [],
  knownProps: [],
  projectiles,
  blasts: [],
  corpses,
  fallenBodies: [],
  crashes,
  guided: [],
  encounter: null,
  skirmish: null,
  fog: { cellM: 8, nx: 0, ny: 0, bits: new Uint32Array(0) },
  groundPatch: {
    epoch: 0,
    side: "blue",
    baseRevision: 0,
    revision: 0,
    full: false,
    runs: new Float32Array(0),
  },
});

/** The live battle's path: publications into the interpolator, one feed and
 *  one driver, sampled at wall times like animation frames. */
function battle(units = UNITS) {
  const interpolator = new TickInterpolator(TICK_MS);
  const feed = new ObservationFeed("blue", units);
  const driver = new PoseDriver({
    units,
    mounts: shippedMounts,
    clip: (_kind, name) => CLIPS[name] ?? null,
    feel: gamePose,
    leanHold: game.cover.lean_hold_s,
  });
  return {
    /** Publish a tick, arriving at wall time `at` ms. */
    publish(o: ObservationView, at: number) {
      interpolator.push(o, at);
    },
    /** The poses drawn at wall time `now` ms. */
    draw(now: number): PoseFrame {
      return driver.update(feed.frame(interpolator.frame(now)!));
    },
  };
}

test("the launcher appearance follows the published operator across a handoff", () => {
  // Only the asset-loading seam is synthetic; feed, posing, and model selection are real.
  const appearances = new Map(
    [
      "test_rifle",
      "test_rifle_b",
      "test_rifle_c",
      "test_at",
      "test_at_b",
      "test_at_c",
      "test_at_carried",
      "test_at_carried_b",
      "test_at_carried_c",
    ].map((name) => [
      name,
      {
        bundle: {
          kind: "skinned",
          skeleton:
            name.startsWith("test_at") && !name.startsWith("test_at_carried")
              ? "launcher"
              : "rifle",
        },
      },
    ]),
  );
  const catalog = new AppearanceCatalog(
    { appearances, sides: { blue: [1, 1, 1], red: [1, 1, 1] } } as unknown as InstalledAppearances,
    UNITS,
  );
  const b = battle();
  const team = {
    ...squad(0, [
      { id: 10, at: [0, 0, 0] },
      { id: 11, at: [3, 0, 0] },
      { id: 12, at: [0, 3, 0] },
    ]),
    kind: "test_at",
    memberSlots: [0, 1, 2],
    memberActiveMounts: [1, 0, 0],
    weaponPoses: [{ mount: 1, operator: 10, bearing: 0, elevation: 0, shots: 0 }],
  };
  const models = (frame: PoseFrame) =>
    poseFrameInstances([], frame, (kind, side, id, slot, mount, active) =>
      catalog.resolve(kind, side, id, slot, mount, active),
    );
  b.publish(observation(1, [team]), 0);
  expect(models(b.draw(0)).map((m) => m.appearance)).toEqual([
    "test_at_b",
    "test_rifle_c",
    "test_rifle",
  ]);
  b.publish(observation(2, [{ ...team, memberActiveMounts: [0, 0, 0] }]), TICK_MS);
  expect(models(b.draw(TICK_MS)).map((m) => m.appearance)).toEqual([
    "test_at_carried_b",
    "test_rifle_c",
    "test_rifle",
  ]);
  b.publish(
    observation(3, [
      {
        ...team,
        members: team.members.slice(1),
        memberIds: [11, 12],
        memberSlots: [1, 2],
        memberActiveMounts: [1, 0],
        weaponPoses: [{ ...team.weaponPoses[0], operator: 11 }],
      },
    ]),
    TICK_MS,
  );
  const survivorModels = models(b.draw(2 * TICK_MS + 150));
  expect(survivorModels.map((m) => m.appearance)).toEqual(["test_at_c", "test_rifle"]);
});

const clips = (frame: PoseFrame) =>
  Object.fromEntries(frame.soldiers.map((s) => [s.soldier, s.clip]));

/** Publish ticks `from`..`to` at 30 Hz wall time, drawing every frame at 120 Hz. */
function play(
  b: ReturnType<typeof battle>,
  from: number,
  to: number,
  at: (tick: number) => ObservationView,
): PoseFrame {
  let last!: PoseFrame;
  for (let tick = from; tick <= to; tick++) {
    b.publish(at(tick), tick * TICK_MS);
    for (let k = 0; k < 4; k++) last = b.draw(tick * TICK_MS + (k * TICK_MS) / 4);
  }
  return last;
}

test("a soldier out on his lean, own or seen, is drawn at his lean point, standing still", () => {
  const b = battle();
  // Own soldier 1 and seen enemy 9 each tucked in at their places, leaning
  // out from tick 10 on; soldier 2 never leans.
  const at = (tick: number) => {
    const out = tick >= 10;
    const own = [
      { id: 1, at: [0, 0, 0] as Point3, lean: out ? ([0, 0.8] as Point2) : undefined },
      { id: 2, at: [4, 0, 0] as Point3 },
    ];
    const seen = [
      { id: 9, at: [50, 0, 0] as Point3, lean: out ? ([50, -0.8] as Point2) : undefined },
    ];
    return observation(tick, [squad(7, own)], { identified: [enemy(3, seen)] });
  };
  const tucked = play(b, 0, 9, at);
  const by = (f: PoseFrame) => new Map(f.soldiers.map((s) => [s.soldier, s]));
  expect(by(tucked).get(1)!.position[1]).toBeCloseTo(0, 5);
  const leaning = by(play(b, 10, 30, at));
  expect(leaning.get(1)!.position[1]).toBeCloseTo(0.8, 5);
  expect(leaning.get(9)!.position[1]).toBeCloseTo(-0.8, 5);
  expect(leaning.get(2)!.position[1]).toBeCloseTo(0, 5);
  for (const s of leaning.values()) expect(["walk", "run"]).not.toContain(s.clip);
});

test("each soldier walks or runs by his own published positions, whatever the member order", () => {
  const b = battle();
  // Soldier 1 walks east at 1.2 m/s, soldier 2 runs north at 4 m/s; the
  // member list swaps order every tick, which must not blend one into the other.
  const at = (tick: number) => {
    const t = tick / 30;
    const walker = { id: 1, at: [1.2 * t, 0, 0] as Point3 };
    const runner = { id: 2, at: [10, 4 * t, 0] as Point3 };
    return observation(tick, [squad(7, tick % 2 ? [walker, runner] : [runner, walker])]);
  };
  const out = play(b, 0, 30, at);
  expect(clips(out)).toEqual({ 1: "walk", 2: "run" });
  const by = new Map(out.soldiers.map((s) => [s.soldier, s]));
  expect(by.get(1)!.facing).toBeCloseTo(0, 3);
  expect(by.get(2)!.facing).toBeCloseTo(Math.PI / 2, 3);
});

test("a paused battle holds every pose, and resuming carries on from it", () => {
  const b = battle();
  const at = (tick: number) =>
    observation(tick, [squad(7, [{ id: 1, at: [(1.2 * tick) / 30, 0, 0] }])]);
  play(b, 0, 20, at);
  // No publication arrives while paused: many frames later, nothing moved.
  const held = { ...b.draw(21 * TICK_MS).soldiers[0] };
  const later = b.draw(60_000).soldiers[0];
  expect(later.clip).toBe("walk");
  expect(later.phase).toBeCloseTo(held.phase, 9);
  expect(later.position).toEqual(held.position);
  // Resumed: the next tick blends on from where it stood, still walking.
  b.publish(at(21), 60_000);
  const resumed = b.draw(60_000 + TICK_MS / 2).soldiers[0];
  expect(resumed.clip).toBe("walk");
  expect(resumed.phase).toBeGreaterThan(held.phase);
  expect(resumed.phase - held.phase).toBeLessThan(0.05);
});

test("a single rifle shot poses exactly the soldier whose flash and sound fire", () => {
  const b = battle();
  const men = [
    { id: 1, at: [0, 0, 0] as Point3 },
    { id: 3, at: [3, 0, 0] as Point3 },
  ];
  // Soldier 1's earlier round is still flying (it began at tick 5) when the
  // squad's counter rises at tick 6; the round that rise names, soldier 3's,
  // first flies at tick 7. Soldier 1's round is in the rising publication,
  // soldier 3's is not.
  const at = (tick: number) =>
    observation(tick, [squad(7, men, { shots: tick >= 6 ? 5 : 4, bearing: 1 })], {
      projectiles: [
        ...(tick === 5 ? [round(1, [0, 0, 1])] : []),
        ...(tick === 6 ? [round(1, [5, 0, 1])] : []),
        ...(tick >= 7 ? [round(3, [3 + 5 * (tick - 7), 0, 1])] : []),
      ],
    });
  // Who the flashes and the sounds say fired: the launches of the same publications.
  const launches = new LaunchTracker();
  const fired = new Set<number | null>();
  for (let tick = 0; tick <= 10; tick++)
    for (const l of launches.note(effectPublication(at(tick), "blue", UNITS), false))
      fired.add(l.soldier);
  expect([...fired]).toEqual([3]);

  play(b, 0, 5, at);
  const posed = play(b, 6, 10, at);
  // The shooter takes his own firing stance; the other man stays at ease.
  const stance = stanceManner(3, gamePose.stance).firing;
  const firingClip =
    stance === "prone" ? "prone_pinned" : stance === "kneel" ? "kneel_fire" : "stand_aim";
  expect(clips(posed)).toEqual({ 1: "idle", 3: firingClip });
  // Aiming, a still soldier turns (at his turn rate, from wherever he was
  // looking) to the weapon's bearing, not the squad's heading.
  expect(posed.soldiers.map((s) => s.facing)).toEqual([1, 1].map(() => expect.closeTo(1, 1)));
});

test("a delayed rifle launch cannot fire the launcher selected on the following tick", () => {
  const b = battle();
  const at = (tick: number) => {
    const team: OwnUnitView = {
      ...squad(7, [{ id: 1, at: [0, 0, 0] }]),
      kind: "test_at",
      memberActiveMounts: [tick >= 7 ? 1 : 0],
      weaponPoses: [
        { mount: 0, operator: null, bearing: 0, elevation: 0, shots: tick >= 6 ? 5 : 4 },
        { mount: 1, operator: 1, bearing: 1, elevation: 0, shots: 0 },
      ],
    };
    return observation(tick, [team], {
      projectiles: tick === 7 ? [round(1)] : [],
    });
  };
  const launches = new LaunchTracker();
  const fired = [5, 6, 7].flatMap((tick) =>
    launches.note(effectPublication(at(tick), "blue", UNITS), false),
  );
  expect(fired.map((l) => [l.soldier, l.mount])).toEqual([[1, 0]]);
  play(b, 5, 6, at);
  const pose = play(b, 7, 7, at).soldiers[0];
  expect(pose.activeMount).toBe(1);
  expect(pose.clip).toBe("idle");
});

test("a rifle shot still poses its shooter when he receives a carried launcher", () => {
  const b = battle();
  const at = (tick: number) => {
    const team: OwnUnitView = {
      ...squad(7, [{ id: 1, at: [0, 0, 0] }]),
      kind: "test_at",
      memberSlots: [1],
      memberActiveMounts: [0],
      weaponPoses: [
        { mount: 0, operator: null, bearing: 0, elevation: 0, shots: tick >= 6 ? 5 : 4 },
        { mount: 1, operator: tick >= 7 ? 1 : null, bearing: 1, elevation: 0, shots: 0 },
      ],
    };
    return observation(tick, [team], {
      projectiles: tick === 7 ? [round(1)] : [],
    });
  };
  play(b, 5, 6, at);
  const pose = play(b, 7, 7, at).soldiers[0];
  expect(pose.operatorMount).toBe(1);
  expect(pose.activeMount).toBe(0);
  expect(pose.clip).toBe("kneel_fire");
});

test("a previous weapon's flash cannot attach to the soldier's currently drawn muzzle", async () => {
  const sources = testSources();
  const baked = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  expect(baked.ok).toBe(true);
  const files = new Map([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(baked.runtime))],
    ...baked.files,
  ]);
  const installed = await new AppearanceLibrary(memoryFetch(files, "/assets/")).load("/assets/");
  const drawn = new DrawnModels(
    installed,
    () => ({ appearance: "rifleman", tint: [1, 1, 1] }),
    UNITS,
  );
  const b = battle();
  const team: OwnUnitView = {
    ...squad(7, [{ id: 1, at: [0, 0, 0] }]),
    kind: "test_at",
    memberActiveMounts: [1],
    weaponPoses: [{ mount: 1, operator: 1, bearing: 0, elevation: 0, shots: 0 }],
  };
  b.publish(observation(5, [team]), 0);
  drawn.update(b.draw(0));
  const source = drawnMuzzleSource(drawn, "blue");
  const at: Point3 = [0, 0, 0];
  const shooter = sideKey(7, "blue", "blue");
  expect(source.muzzle(shooter, 1, 1, at)).toBe(true);
  expect(source.muzzle(shooter, 0, 1, at)).toBe(false);
});

test("the supported hold aligns the drawn bore and moving carried kit rejects its launcher flash", async () => {
  const sources = testSources();
  const baked = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  expect(baked.ok).toBe(true);
  const files = new Map([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(baked.runtime))],
    ...baked.files,
  ]);
  const installed = await new AppearanceLibrary(memoryFetch(files, "/assets/")).load("/assets/");
  const units = new UnitCatalog({
    ...UNITS.view,
    units: UNITS.view.units.map((u) =>
      u.id === "test_at"
        ? {
            ...u,
            mounts: u.mounts.map((m, i) =>
              i === 1
                ? {
                    ...m,
                    operator_appearance: {
                      active: ["rifleman"],
                      carried: ["rifleman"],
                      active_pose: { clip: "kneel_fire", phase: 0 },
                    },
                  }
                : m,
            ),
          }
        : u,
    ),
  });
  const drawn = new DrawnModels(
    installed,
    () => ({ appearance: "rifleman", tint: [1, 1, 1] }),
    units,
  );
  const b = battle(units);
  const team = (x: number): OwnUnitView => ({
    ...squad(7, [{ id: 1, at: [x, 0, 0] }]),
    kind: "test_at",
    memberActiveMounts: [1],
    weaponPoses: [{ mount: 1, operator: 1, bearing: Math.PI / 2, elevation: 0, shots: 0 }],
  });
  b.publish(observation(0, [team(0)]), 0);
  const held = b.draw(0);
  expect([held.soldiers[0].clip, held.soldiers[0].phase, held.soldiers[0].facing]).toEqual([
    "kneel_fire",
    0,
    Math.PI / 2,
  ]);
  drawn.update(held);
  const at: Point3 = [0, 0, 0];
  expect(drawn.soldier("blue", 1, 1, at)).toBe(true);
  // The baked socket is [.7,-.2,1.4]; +90° facing turns it into [.2,.7,1.4].
  at.forEach((v, i) => expect(v).toBeCloseTo([0.2, 0.7, 1.4][i], 5));
  b.publish(observation(15, [team(1)]), 500);
  const moving = b.draw(600);
  expect(moving.soldiers[0].activeMount).toBeNull();
  drawn.update(moving);
  expect(drawn.soldier("blue", 1, 1, at)).toBe(false);
});

test("an own tank and an identified enemy with the same id keep their own mounts", () => {
  // Identified enemies' handles restart at 1: own tank 3 and enemy 3 share a frame.
  const feed = new ObservationFeed("blue", UNITS);
  const own: OwnUnitView = {
    ...squad(3, []),
    kind: "test_tank",
    weaponPoses: [{ mount: 0, operator: null, bearing: 0.4, elevation: 0, shots: 1 }],
  };
  const seen: IdentifiedView = {
    ...enemy(3, []),
    kind: "test_tank",
    weaponPoses: [{ mount: 0, operator: null, bearing: 2, elevation: 0, shots: 7 }],
  };
  const o = observation(1, [own], { identified: [seen] });
  const pose = (id: number) => ({
    id,
    position: [0, 0, 0] as Point3,
    yaw: 0,
    members: [],
    memberIds: [],
    deployment: null,
  });
  const frame = feed.frame({
    observation: o,
    own: [pose(3)],
    identified: [pose(3)],
    crashes: [],
    time: 0,
  });
  expect(frame.units.map((u) => [u.side, u.mounts[0].bearing, u.mounts[0].shots])).toEqual([
    ["blue", 0.4, 1],
    ["red", 2, 7],
  ]);
});

test("a helicopter shot down falls as itself, rotors still turning, tipping into its fall", () => {
  const b = battle();
  const heli: OwnUnitView = {
    ...squad(4, []),
    kind: "test_heli",
    position: [50, 60, 20],
    yaw: 0,
    weaponPoses: [],
  };
  const flying = play(b, 0, 5, (tick) => observation(tick, [heli])).vehicles;
  expect(flying.map((v) => [v.unit, v.tilt])).toEqual([[4, null]]);
  const spun = flying[0].articulation.rotor;
  // Shot down: no longer a unit, its airframe falls nose down, leaning.
  const falling = (tick: number): FallingAirframeView => ({
    id: 4,
    own: true,
    kind: "test_heli",
    position: [50 + (tick - 5), 60, 20 - (tick - 5) * 0.5],
    yaw: 0.1 * (tick - 5),
    pitch: -0.3,
    roll: 0.2,
  });
  const fell = play(b, 6, 9, (tick) => observation(tick, [], { crashes: [falling(tick)] }));
  expect(fell.vehicles).toHaveLength(1);
  const v = fell.vehicles[0];
  expect([v.unit, v.side, v.kind]).toEqual([4, "blue", "test_heli"]);
  // Drawn between its last two published places, as a unit is.
  expect(v.position[0]).toBeGreaterThan(52);
  expect(v.position[0]).toBeLessThanOrEqual(54);
  expect(v.articulation.rotor).toBeGreaterThan(spun);
  // Its nose (+X) drops below its foot, and its right side (-Y) dips.
  const tilt = v.tilt!;
  expect(tilt[2]).toBeCloseTo(-Math.sin(0.3), 6);
  expect(-tilt[6]).toBeLessThan(0);
  // Landed: its wreck takes over and the airframe is gone.
  expect(play(b, 10, 11, (tick) => observation(tick, [])).vehicles).toEqual([]);
});

test("a rise no launch explains poses no one: no flash, no sound, no kneel", () => {
  const b = battle();
  const men = [
    { id: 1, at: [0, 0, 0] as Point3 },
    { id: 2, at: [3, 0, 0] as Point3 },
  ];
  play(b, 0, 5, (tick) => observation(tick, [squad(7, men, { shots: 4 })]));
  const unseen = play(b, 6, 10, (tick) => observation(tick, [squad(7, men, { shots: 7 })]));
  for (const clip of Object.values(clips(unseen))) expect(clip).not.toBe("kneel_fire");
});

test("prone while the published tier is pinned, standing again as it recovers", () => {
  const b = battle();
  const men = [{ id: 1, at: [0, 0, 0] as Point3 }];
  const under = play(b, 0, 5, (tick) =>
    observation(tick, [squad(7, men, { suppression: "suppressed" })]),
  );
  expect(clips(under)).toEqual({ 1: "idle" });
  const pinned = play(b, 6, 10, (tick) =>
    observation(tick, [squad(7, men, { suppression: "pinned" })]),
  );
  expect(clips(pinned)).toEqual({ 1: "prone_pinned" });
  const recovered = play(b, 11, 30, (tick) =>
    observation(tick, [squad(7, men, { suppression: "suppressed" })]),
  );
  expect(clips(recovered)).toEqual({ 1: "idle" });
  expect(recovered.soldiers[0].blend).toBeNull();
});

test("a reinforcement joins as himself: idle, not kneeling for rounds fired before he came", () => {
  const b = battle();
  const first = { id: 1, at: [0, 0, 0] as Point3 };
  play(b, 0, 5, (tick) => observation(tick, [squad(7, [first], { shots: 12 })]));
  const joined = play(b, 6, 8, (tick) =>
    observation(tick, [squad(7, [first, { id: 9, at: [2, 0, 0] }], { shots: 12 })]),
  );
  expect(clips(joined)).toEqual({ 1: "idle", 9: "idle" });
});

test("an enemy lost and reacquired starts fresh: no kneel for rounds fired unseen", () => {
  const b = battle();
  const men = [{ id: 40, at: [50, 0, 0] as Point3 }];
  play(b, 0, 5, (tick) => observation(tick, [], { identified: [enemy(30, men, 2)] }));
  // Hidden for a while (fired 6 rounds meanwhile), then identified again.
  play(b, 6, 40, (tick) => observation(tick, []));
  const back = play(b, 41, 43, (tick) =>
    observation(tick, [], { identified: [enemy(30, men, 8)] }),
  );
  expect(clips(back)).toEqual({ 40: "idle" });
  expect(back.soldiers[0].side).toBe("red");
});

test("a death seen plays out then lies static; a death unseen is only ever a corpse", () => {
  const b = battle();
  const man = { id: 1, at: [0, 0, 0] as Point3 };
  play(b, 0, 5, (tick) => observation(tick, [squad(7, [man])]));
  const fallen: CorpseView[] = [
    { position: [0, 0, 0], own: true, soldier: 1, kind: "test_rifle", slot: 0, yaw: 2 },
    // Enemy soldier 50 fell where blue never saw him alive.
    { position: [80, 0, 0], own: false, soldier: 50, kind: "test_rifle", slot: 0, yaw: 1 },
  ];
  const dying = play(b, 6, 8, (tick) => observation(tick, [squad(7, [])], { corpses: fallen }));
  expect(clips(dying)).toEqual({ 1: "death" });
  expect(dying.corpses.map((c) => c.soldier)).toEqual([50]);
  expect(dying.corpses[0]).toMatchObject({ side: "red", yaw: 1 });
  const version = dying.corpsesVersion;
  // A second later his death has played out: both lie static, and the static
  // list changed exactly once more.
  const lying = play(b, 9, 45, (tick) => observation(tick, [squad(7, [])], { corpses: fallen }));
  expect(lying.soldiers).toEqual([]);
  expect(lying.corpses.map((c) => c.soldier).sort()).toEqual([1, 50]);
  expect(lying.corpsesVersion).toBe(version + 1);
});

test("a soldier who falls in a tick the page has not rendered yet still plays his death", () => {
  // The session renders React's observation once an animation frame, while
  // each publication reaches the interpolator as it arrives. Here tick 7,
  // where he falls, arrives after React captured tick 6, and a frame is drawn
  // before React catches up: the frame must read who has fallen from tick 7.
  const b = battle();
  const man = { id: 1, at: [0, 0, 0] as Point3 };
  const mate = { id: 2, at: [4, 0, 0] as Point3 };
  play(b, 0, 6, (tick) => observation(tick, [squad(7, [man, mate])]));
  const fell: CorpseView = {
    position: [0, 0, 0],
    own: true,
    soldier: 1,
    kind: "test_rifle",
    slot: 0,
    yaw: 2,
  };
  b.publish(observation(7, [squad(7, [mate])], { corpses: [fell] }), 7 * TICK_MS);
  const first = b.draw(7 * TICK_MS);
  expect(clips(first)[1]).toBe("death");
  expect(first.corpses).toEqual([]);
  // The frames after carry his death on until it has played out.
  const later = play(b, 8, 12, (tick) =>
    observation(tick, [squad(7, [mate])], { corpses: [fell] }),
  );
  expect(clips(later)[1]).toBe("death");
  expect(later.corpses).toEqual([]);
});

/** A drawn sample of `observation` with nothing posed. */
const still = (observation: ObservationView, time: number) => ({
  observation,
  own: [],
  identified: [],
  crashes: [],
  time,
});

test("immutable corpse input keeps its converted list across active observation changes", () => {
  const feed = new ObservationFeed("blue", UNITS);
  const corpse: CorpseView = {
    soldier: 12,
    position: [2, 3, 4],
    yaw: 0,
    kind: "test_rifle",
    slot: 0,
    own: true,
  };
  Object.freeze(corpse.position);
  Object.freeze(corpse);
  const corpses = Object.freeze([corpse]);
  const first = { ...observation(1, []), corpses };
  const before = feed.frame(still(first, 1)).fallen;
  const after = feed.frame(still({ ...first, tick: 2 }, 2)).fallen;
  expect(after).toBe(before);
  expect(after[0]).toBe(before[0]);
  expect(after[0].position).toEqual([2, 3, 4]);
  const shifted: CorpseView = { ...corpse, position: [2, 3, 1] };
  const changed = feed.frame(still({ ...first, tick: 3, corpses: [shifted] }, 3)).fallen;
  expect(changed).not.toBe(before);
  expect(changed[0].position).toEqual([2, 3, 1]);
  expect(before[0].position).toEqual([2, 3, 4]);
});
