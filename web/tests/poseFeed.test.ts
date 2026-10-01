// @vitest-environment node
// The battle's pose seam on packed frames: decoded observations, blended by
// the presentation clock (`TickInterpolator`), fed per soldier by member id
// (`ObservationFeed`) to the pose driver. What the side published decides
// every pose; nothing reads a formation slot or the squad's heading.
import village from "@fixtures/village.json";
import { expect, test } from "vitest";
import { ObservationFeed, villagePose } from "@apps/battle-lab/src/poseFeed";
import { effectPublication } from "@apps/battle-lab/src/effectFeed";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { shippedMounts } from "./shippedMounts";
import { LaunchTracker } from "@packages/battle-renderer/src/effects/launches";
import { PoseDriver, type PoseFrame } from "@packages/battle-renderer/src/models/poseDriver";
import { TickInterpolator } from "../src/battle/present/interpolate";
import type {
  CorpseView,
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
  kind: "rifle",
  position: soldiers[0]?.at ?? [0, 0, 0],
  yaw: Math.PI / 2,
  goal: null,
  policy: null,
  direction: null,
  reversing: false,
  state: "idle",
  blocker: null,
  route: [],
  queue: [],
  members: soldiers.map((s) => s.at),
  memberIds: soldiers.map((s) => s.id),
  memberSlots: soldiers.map(() => 0),
  memberOrders: [],
  memberLeans: soldiers.map((s) => (s.lean ? { side: "left", at: s.lean } : null)),
  area: null,
  finalFacing: 0,
  sees: [],
  engagement: "fire_at_will",
  mounts: [],
  weaponPoses: [{ mount: 0, bearing, elevation: 0, shots }],
  hp: 100,
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
  kind: "rifle",
  cost: 1,
  position: soldiers[0]?.at ?? [0, 0, 0],
  yaw: 0,
  velocity: [0, 0],
  members: soldiers.map((s) => s.at),
  memberIds: soldiers.map((s) => s.id),
  memberSlots: soldiers.map(() => 0),
  memberLeans: soldiers.map((s) => (s.lean ? { side: "right", at: s.lean } : null)),
  weaponPoses: [{ mount: 0, bearing: Math.PI, elevation: 0, shots }],
  reversing: false,
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
  guided: [],
  encounter: null,
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
function battle() {
  const interpolator = new TickInterpolator(TICK_MS);
  const feed = new ObservationFeed("blue", UNITS);
  const driver = new PoseDriver({
    units: UNITS,
    mounts: shippedMounts,
    clip: (_kind, name) => CLIPS[name] ?? null,
    feel: villagePose,
    leanHold: village.cover.lean_hold_s,
  });
  let latest: ObservationView | null = null;
  return {
    /** Publish a tick, arriving at wall time `at` ms. */
    publish(o: ObservationView, at: number) {
      latest = o;
      interpolator.push(o, at);
    },
    /** The poses drawn at wall time `now` ms. */
    draw(now: number): PoseFrame {
      const own = interpolator.sample(now);
      const identified = interpolator.sampleIdentified(now);
      return driver.update(feed.frame(latest!, own, identified, interpolator.time(now)!));
    },
  };
}

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
    { id: 2, at: [3, 0, 0] as Point3 },
  ];
  // Soldier 1's earlier round is still flying (it began at tick 5) when the
  // squad's counter rises at tick 6; the round that rise names, soldier 2's,
  // first flies at tick 7. Soldier 1's round is in the rising publication,
  // soldier 2's is not.
  const at = (tick: number) =>
    observation(tick, [squad(7, men, { shots: tick >= 6 ? 5 : 4, bearing: 1 })], {
      projectiles: [
        ...(tick === 5 ? [round(1, [0, 0, 1])] : []),
        ...(tick === 6 ? [round(1, [5, 0, 1])] : []),
        ...(tick >= 7 ? [round(2, [3 + 5 * (tick - 7), 0, 1])] : []),
      ],
    });
  // Who the flashes and the sounds say fired: the launches of the same publications.
  const launches = new LaunchTracker();
  const fired = new Set<number | null>();
  for (let tick = 0; tick <= 10; tick++)
    for (const l of launches.note(effectPublication(at(tick), "blue", UNITS), false))
      fired.add(l.soldier);
  expect([...fired]).toEqual([2]);

  play(b, 0, 5, at);
  const posed = play(b, 6, 10, at);
  expect(clips(posed)).toEqual({ 1: "idle", 2: "kneel_fire" });
  // Aiming, a still soldier turns (at his turn rate, from wherever he was
  // looking) to the weapon's bearing, not the squad's heading.
  expect(posed.soldiers.map((s) => s.facing)).toEqual([1, 1].map(() => expect.closeTo(1, 1)));
});

test("an own tank and an identified enemy with the same id keep their own mounts", () => {
  // Identified enemies' handles restart at 1: own tank 3 and enemy 3 share a frame.
  const feed = new ObservationFeed("blue", UNITS);
  const own: OwnUnitView = {
    ...squad(3, []),
    kind: "tank",
    weaponPoses: [{ mount: 0, bearing: 0.4, elevation: 0, shots: 1 }],
  };
  const seen: IdentifiedView = {
    ...enemy(3, []),
    kind: "tank",
    weaponPoses: [{ mount: 0, bearing: 2, elevation: 0, shots: 7 }],
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
  const frame = feed.frame(o, [pose(3)], [pose(3)], 0);
  expect(frame.units.map((u) => [u.side, u.mounts[0].bearing, u.mounts[0].shots])).toEqual([
    ["blue", 0.4, 1],
    ["red", 2, 7],
  ]);
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
    { position: [0, 0, 0], own: true, soldier: 1, kind: "rifle", slot: 0, yaw: 2 },
    // Enemy soldier 50 fell where blue never saw him alive.
    { position: [80, 0, 0], own: false, soldier: 50, kind: "rifle", slot: 0, yaw: 1 },
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
