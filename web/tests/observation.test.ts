// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, pack_observation, village_scenario } from "@wasm/game_wasm.js";
import { decodeObservation, type ObservationLayout } from "../src/battle/sim/observation";
import { sightMultiplier } from "@packages/battle-renderer/src/sightOverlay";
import { labScenario } from "@apps/battle-lab/src/scenarios";
import sensors from "@fixtures/sensors-lab.json";
import weaponsMap from "@fixtures/weapons-lab.json";
import deploymentMap from "@fixtures/deployment-lab.json";
import garrisonMap from "@fixtures/garrison-lab.json";
import village from "@fixtures/village.json";
import type { Order } from "../src/battle/sim/protocol";

// Whole battles run to a late state; under a loaded `bun run check` they
// can pass Vitest's 5 s default without anything being wrong.
const BATTLE_TEST_TIMEOUT_MS = 30_000;

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

/** Publish first: packing can grow WASM memory and move the buffer. */
function published(battle: Battle, layout: ObservationLayout, side: "blue" | "red" = "blue") {
  const length = battle.publish(side);
  return decodeObservation(
    layout,
    new Float32Array(memory.buffer, battle.publication_ptr(), length).slice(),
  );
}

test("a packed side frame decodes group by group through the published layout", () => {
  const scenario = labScenario(
    sensors,
    [
      { side: "blue", kind: "recon", position: [560, 480] },
      { side: "blue", kind: "rifle", position: [560, 430] },
      { side: "red", kind: "rifle", position: [840, 480] },
      { side: "red", kind: "tank", position: [700, 300] },
    ],
    [
      { tick: 5, fire: { unit: 2 } },
      {
        tick: 5,
        add_prop: { kind: "wall", center: [620, 470], yaw: 0, half_extents: [0.5, 3, 2] },
      },
    ],
  );
  const battle = new Battle(scenario, 3);
  for (let t = 0; t < 15; t++) battle.step();
  const length = battle.publish("blue");
  const frame = decodeObservation(
    JSON.parse(battle.observation_layout()) as ObservationLayout,
    new Float32Array(memory.buffer, battle.publication_ptr(), length).slice(),
  );
  expect(frame.tick).toBe(15);
  expect(frame.own.map((u) => u.kind)).toEqual(["recon", "rifle"]);
  expect(frame.own[1].members).toHaveLength(8);
  expect(frame.identified.map((e) => e.kind)).toEqual(["tank"]);
  expect(frame.contacts).toHaveLength(1);
  const c = frame.contacts[0];
  expect(c.source).toBe("firing");
  expect(c.radius).toBe(100);
  expect(Math.hypot(c.center[0] - 840, c.center[1] - 480)).toBeLessThanOrEqual(100);
  expect(frame.audible.every((a) => a.sector >= 0 && a.sector < 8)).toBe(true);
  expect(frame.knownProps.map((p) => p.kind)).toEqual(["wall"]);
  expect(frame.fog.nx).toBeGreaterThan(0);
  const rifle = frame.own[1];
  expect(rifle.engagement).toBe("fire_at_will");
  // Rifles and grenade launcher, each with its own readiness.
  expect(rifle.mounts.map((m) => m.mount)).toEqual([0, 1]);
  expect(rifle.mounts.every((m) => m.ammo.length === 1)).toBe(true);
  expect(rifle.mounts[0].ammo).toEqual([null]);
  expect(rifle.memberHp).toEqual(Array(8).fill(100));
  expect(rifle.suppression).toBe(0);
  expect(frame.own[0].hp).toBe(0);
  battle.free();
});

test("each own unit's sight decodes: eyes, forward, shape and range", () => {
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "tank", position: [200, 250], yaw: 0.5 },
    { side: "blue", kind: "rifle", position: [200, 300], yaw: 1.5 },
  ]);
  const battle = new Battle(scenario, 3);
  battle.step();
  const [tank, rifle] = published(
    battle,
    JSON.parse(battle.observation_layout()) as ObservationLayout,
  ).own;
  const s = village.sensors;
  // Float32 transport: values survive to single precision.
  expect(tank.sight.forward).toBeCloseTo(0.5, 6);
  for (const k of ["front", "side", "rear"] as const)
    expect(tank.sight.shape[k]).toBeCloseTo(s.sight_shape.tank[k], 6);
  expect(tank.sight.range).toBe(s.tank_ground_m);
  expect(tank.sight.eyes).toHaveLength(1);
  const [x, y, z] = tank.sight.eyes[0];
  expect([x, y]).toEqual([tank.position[0], tank.position[1]]);
  expect(z).toBeCloseTo(tank.position[2] + village.physics.tank_eye_m, 4);
  expect(rifle.sight.shape).toEqual({ front: 1, side: 1, rear: 1 });
  expect(rifle.sight.range).toBe(s.infantry_ground_m);
  // The published reach matches the shape's anchors.
  const reach = (off: number) => tank.sight.range * sightMultiplier(tank.sight.shape, off);
  expect(reach(0)).toBeCloseTo(s.tank_ground_m * s.sight_shape.tank.front, 3);
  expect(reach(Math.PI / 2)).toBeCloseTo(s.tank_ground_m * s.sight_shape.tank.side, 3);
  expect(reach(Math.PI)).toBeCloseTo(s.tank_ground_m * s.sight_shape.tank.rear, 3);
  battle.free();
});

test("mount readiness and visible projectile segments decode", () => {
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "tank", position: [200, 250] },
    { side: "red", kind: "tank", position: [400, 250] },
  ]);
  const battle = new Battle(scenario, 3);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const decode = () => published(battle, layout);
  let flying = null;
  for (let t = 0; t < 300 && !flying; t++) {
    battle.step();
    const frame = decode();
    if (frame.projectiles.length > 0) flying = frame;
  }
  expect(flying).not.toBeNull();
  const cannon = flying!.own[0].mounts[0];
  // The cannon holds AP and HE on one mount; AP is chosen against armour.
  expect(cannon.ammo).toHaveLength(2);
  expect(cannon.target?.kind).toBe("identified");
  expect(layout.actionReasons).toContain(cannon.reason);
  for (const p of flying!.projectiles) {
    expect(p.path.length).toBeGreaterThanOrEqual(2);
    expect(p.path.flat().every(Number.isFinite)).toBe(true);
  }
  battle.free();
});

test("deployment progress, its target and the packing state decode", () => {
  const scenario = labScenario(deploymentMap, [
    { side: "blue", kind: "supply", position: [100, 100] },
    { side: "blue", kind: "tank", position: [100, 60] },
  ]);
  const battle = new Battle(scenario, 3);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const decode = () => published(battle, layout);
  const send = (seq: number, order: Order) =>
    JSON.parse(battle.accept(JSON.stringify({ side: "blue", seq, order, queued: false })));
  const ticks = village.service.deploy_and_pack_s * village.tick_hz;
  for (let t = 0; t < ticks / 2; t++) battle.step();
  let [supply, tank] = decode().own;
  // A stopped supply unit sets up where it stands; a tank never deploys.
  expect(supply.deployment).toEqual({ progress: 0.5, target: "deployed" });
  expect(tank.deployment).toBeNull();
  expect(send(1, { kind: "set_deployment", units: [0], deployed: false }).error).toBeNull();
  battle.step();
  [supply] = decode().own;
  expect(supply.deployment!.target).toBe("packed");
  expect(supply.deployment!.progress).toBeCloseTo(0.5 - 1 / ticks, 6);
  send(2, { kind: "move", units: [0], gesture: 1, goal: [150, 100], route: "shortest" });
  battle.step();
  [supply] = decode().own;
  expect(supply.state).toBe("packing");
  expect(layout.postures).toEqual(["packed", "deployed"]);
  battle.free();
});

test("casualties, health and corpses decode", () => {
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "tank", position: [200, 250] },
    { side: "red", kind: "rifle", position: [320, 250], engagement: "return_fire_only" },
  ]);
  const battle = new Battle(scenario, 6);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const decode = (side: "blue" | "red") => published(battle, layout, side);
  let red = decode("red");
  for (let t = 0; t < 900 && red.corpses.length === 0; t++) {
    battle.step();
    red = decode("red");
  }
  expect(red.corpses.length).toBeGreaterThan(0);
  expect(red.corpses.every((c) => c.own)).toBe(true);
  const squad = red.own.find((u) => u.kind === "rifle");
  if (squad) {
    expect(squad.memberHp).toHaveLength(squad.members.length);
    expect(squad.members.length).toBe(8 - red.corpses.length);
  }
  expect(decode("blue").own[0].hp).toBeGreaterThan(0);
  battle.free();
});

test("guided missiles and their launcher's support decode", () => {
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "at", position: [200, 250] },
    { side: "red", kind: "tank", position: [600, 250], engagement: "return_fire_only" },
  ]);
  const battle = new Battle(scenario, 4);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  let frame = published(battle, layout);
  for (let t = 0; t < 600 && frame.guided.length === 0; t++) {
    battle.step();
    frame = published(battle, layout);
  }
  expect(frame.guided).toHaveLength(1);
  expect(frame.guided[0].supported).toBe(true);
  expect(frame.own[0].mounts.some((m) => m.guiding)).toBe(true);
  expect(layout.actionReasons).toContain("no_own_sight");
  battle.free();
});

test(
  "garrison phase, progress and a ruin standing in for its building decode",
  () => {
    // Blue's squads garrison the building and hold fire; a red spotter north
    // sees them, and red's tank shells them from afar until the building falls.
    // Blue's scouts watch from the west.
    const scenario = labScenario(garrisonMap, [
      { side: "blue", kind: "rifle", position: [300, 250], engagement: "return_fire_only" },
      { side: "blue", kind: "rifle", position: [300, 265], engagement: "return_fire_only" },
      { side: "blue", kind: "recon", position: [200, 250], engagement: "return_fire_only" },
      { side: "red", kind: "tank", position: [580, 450] },
      { side: "red", kind: "rifle", position: [360, 350], engagement: "return_fire_only" },
    ]);
    const battle = new Battle(scenario, 7);
    const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
    const decode = () => published(battle, layout);
    const order: Order = { kind: "garrison", units: [0, 1], building: 0 };
    const ack = JSON.parse(
      battle.accept(JSON.stringify({ side: "blue", seq: 1, order, queued: false })),
    );
    expect(ack.error).toBeNull();
    expect(layout.garrisonPhases).toEqual(["entering", "waiting_for_room", "inside", "exiting"]);
    let frame = decode();
    expect(frame.own[0].garrison).toBeNull();
    let entering = null;
    for (let t = 0; t < 1200 && frame.own[0].garrison?.phase !== "inside"; t++) {
      battle.step();
      frame = decode();
      if (frame.own[0].garrison?.phase === "entering") entering ??= frame.own[0].garrison;
    }
    expect(entering).toMatchObject({ building: 0, phase: "entering" });
    expect(entering!.progress).toBeGreaterThan(0);
    expect(entering!.progress).toBeLessThan(1);
    expect(frame.own[0].garrison).toEqual({ building: 0, phase: "inside", progress: 1 });
    // Occupants stand at slots just outside the 24 × 24 m footprint.
    for (const [x, y] of frame.own[0].members) {
      expect(Math.max(Math.abs(x - 360), Math.abs(y - 250))).toBeCloseTo(12.45, 4);
    }
    let ruin = null;
    for (let t = 0; t < 6000 && !ruin; t++) {
      battle.step();
      ruin = decode().knownProps.find((p) => p.kind === "ruin") ?? null;
    }
    expect(ruin).toMatchObject({ kind: "ruin", replaces: 0, center: [360, 250] });
    expect(ruin!.half[2] * 2).toBe(village.props.building.destroyed.into.height_m);
    battle.free();
  },
  BATTLE_TEST_TIMEOUT_MS,
);

test("supply stock and each unit's service status decode", () => {
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "supply", position: [200, 250], stock: 55 },
    {
      side: "blue",
      kind: "rifle",
      position: [220, 250],
      engagement: "return_fire_only",
      condition: { casualties: 2 },
    },
  ]);
  const battle = new Battle(scenario, 5);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  let frame = published(battle, layout);
  expect(frame.own[0].stock).toBe(55);
  expect(frame.own[1].stock).toBeNull();
  expect(layout.serviceStatuses).toContain("no_stock");
  for (let t = 0; t < 600; t++) battle.step();
  frame = published(battle, layout);
  expect(frame.own[1].service).toBe("serving");
  expect(frame.own[0].stock).toBeLessThan(55);
  battle.free();
});

test("the encounter status decodes, and is absent outside an encounter", () => {
  const lab = new Battle(
    labScenario(weaponsMap, [{ side: "blue", kind: "rifle", position: [200, 250] }]),
    1,
  );
  const layout = JSON.parse(lab.observation_layout()) as ObservationLayout;
  lab.step();
  expect(published(lab, layout).encounter).toBeNull();
  lab.free();
  const battle = new Battle(village_scenario(JSON.stringify(village), "ordinary"), 1);
  battle.step();
  expect(published(battle, layout, "red").encounter).toEqual({ heldS: 0, result: "running" });
  expect(layout.encounterResults).toEqual(["running", "captured", "defeated", "inconclusive"]);
  battle.free();
});

test("every animation-feed field and ground patch round-trips, integers exact past 2^24", () => {
  const lab = new Battle(labScenario(weaponsMap, []), 1);
  const layout = JSON.parse(lab.observation_layout()) as ObservationLayout;
  lab.free();
  const big = 2 ** 24 + 1;
  const pose = (mount: number, shots: number) => ({ mount, bearing: 1.5, elevation: -0.25, shots });
  const readiness = (mount: number) => ({
    mount,
    loaded: null,
    ammo: [null],
    aim: 1,
    reload: 0,
    reloading: null,
    target: null,
    reason: "firing",
    guiding: false,
  });
  // An ObservationFrame exactly as the simulation serializes one.
  const frame = {
    tick: 7,
    own: [
      {
        id: 0,
        kind: "rifle",
        position: [10, 20, 1],
        yaw: 0.5,
        goal: null,
        policy: null,
        direction: null,
        reversing: false,
        state: "idle",
        blocker: null,
        route: [],
        queue: [],
        members: [
          [1, 2, 3],
          [4, 5, 6],
        ],
        member_ids: [3, big + 2],
        member_orders: [
          { spot: [7, 8], cover_now: null, cover_there: "heavy" },
          { spot: [9, 10], cover_now: "light", cover_there: "medium" },
        ],
        member_leans: [null, { side: "right", at: [4.5, 5.5] }],
        area: { anchor: [2, 3], radius: 14 },
        final_facing: 1.25,
        sees: [],
        engagement: "fire_at_will",
        mounts: [readiness(0), readiness(1)],
        weapon_poses: [pose(0, big + 4), pose(1, 2 ** 32 - 1)],
        deployment: null,
        hp: 0,
        member_hp: [100, 50],
        suppression: 0,
        stock: null,
        service: "full",
        garrison: null,
        sight: { eyes: [], forward: 0, shape: { front: 1, side: 1, rear: 1 }, range: 600 },
      },
    ],
    identified: [
      {
        id: 4,
        kind: "tank",
        cost: 10,
        position: [300, 20, 0],
        yaw: 3,
        velocity: [0, 0],
        members: [],
        member_ids: [],
        member_leans: [],
        weapon_poses: [pose(0, 9), pose(1, big + 6)],
        reversing: true,
      },
    ],
    contacts: [],
    audible: [],
    known_props: [],
    projectiles: [
      {
        path: [
          [0, 0, 1],
          [4, 1, 1.5],
          [6, -1, 2],
          [8, 0, 1],
        ],
        ricochets: [
          { point: 1, normal: [0, -1, 0] },
          { point: 2, normal: [0, 0, 1] },
        ],
        own: false,
        kind: 2,
        shooter_member: big + 8,
        hit: "hull",
        impact_normal: [0, -1, 0],
      },
      {
        path: [
          [0, 0, 1],
          [8, 0, 1],
        ],
        ricochets: [],
        own: true,
        kind: 0,
        shooter_member: null,
        hit: "none",
        impact_normal: null,
      },
    ],
    blasts: [{ point: [5, 6, 0.5], radius: 12, kind: 3 }],
    corpses: [{ position: [2, 3, 0], own: false, soldier: big + 10, kind: "at", yaw: -1.25 }],
    guided: [],
    encounter: null,
    ground_visibility: { cell_m: 8, nx: 2, ny: 2, bits: [5] },
  };
  const patch = {
    epoch: 4,
    side: "red",
    base_revision: 6,
    revision: 9,
    full: false,
    cells: [
      { cell: big + 12, crater: 255, scorch: 0, tracks: 17, trampled: 200, cleared: 0 },
      { cell: 0, crater: 1, scorch: 2, tracks: 3, trampled: 4, cleared: 255 },
    ],
  };
  const o = decodeObservation(
    layout,
    new Float32Array(pack_observation(JSON.stringify(frame), JSON.stringify(patch))),
  );
  expect(o.groundPatch).toEqual({
    epoch: 4,
    side: "red",
    baseRevision: 6,
    revision: 9,
    full: false,
    cells: Uint32Array.from([big + 12, 0]),
    // A cleared cell draws as full track wear.
    marks: Uint8Array.from([255, 0, 17, 200, 1, 2, 255, 4]),
    cleared: Uint8Array.from([0, 255]),
  });
  expect(o.own[0].memberIds).toEqual([3, big + 2]);
  expect(o.own[0].memberOrders).toEqual([
    { spot: [7, 8], coverNow: null, coverThere: "heavy" },
    { spot: [9, 10], coverNow: "light", coverThere: "medium" },
  ]);
  expect(o.own[0].finalFacing).toBe(1.25);
  expect(o.own[0].memberLeans).toEqual([null, { side: "right", at: [4.5, 5.5] }]);
  expect(o.own[0].area).toEqual({ anchor: [2, 3], radius: 14 });
  expect(o.own[0].weaponPoses).toEqual([
    { mount: 0, bearing: 1.5, elevation: -0.25, shots: big + 4 },
    { mount: 1, bearing: 1.5, elevation: -0.25, shots: 2 ** 32 - 1 },
  ]);
  expect(o.own[0].mounts.map((m) => m.mount)).toEqual([0, 1]);
  expect(o.identified[0].memberIds).toEqual([]);
  expect(o.identified[0].weaponPoses.map((p) => p.shots)).toEqual([9, big + 6]);
  expect(o.identified[0].reversing).toBe(true);
  expect(o.projectiles).toEqual([
    {
      path: [
        [0, 0, 1],
        [4, 1, 1.5],
        [6, -1, 2],
        [8, 0, 1],
      ],
      ricochets: [
        { point: 1, normal: [0, -1, 0] },
        { point: 2, normal: [0, 0, 1] },
      ],
      own: false,
      kind: layout.roundKinds[2],
      shooterMember: big + 8,
      hit: "hull",
      impactNormal: [0, -1, 0],
    },
    {
      path: [
        [0, 0, 1],
        [8, 0, 1],
      ],
      ricochets: [],
      own: true,
      kind: layout.roundKinds[0],
      shooterMember: null,
      hit: "none",
      impactNormal: null,
    },
  ]);
  expect(o.blasts).toEqual([{ point: [5, 6, 0.5], radius: 12, kind: layout.roundKinds[3] }]);
  expect(o.corpses).toEqual([
    { position: [2, 3, 0], own: false, soldier: big + 10, kind: "at", yaw: -1.25 },
  ]);
  // Round kinds are the fixture's weapon rows, in name order.
  expect(layout.roundKinds).toEqual(Object.keys(village.weapons).sort());
});

test("a live battle publishes poses, soldier ids, tracer kinds and blasts", () => {
  // Blue's tank shells red's squad; red shoots back.
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "tank", position: [200, 250] },
    { side: "red", kind: "rifle", position: [420, 250] },
  ]);
  const battle = new Battle(scenario, 7);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const kinds = new Set<string>();
  let blast = null;
  let frame = published(battle, layout);
  for (let t = 0; t < 900 && !(blast && kinds.has("rifle")); t++) {
    battle.step();
    frame = published(battle, layout);
    for (const p of frame.projectiles) if (!p.own) kinds.add(p.kind);
    blast ??= frame.blasts.find((b) => b.kind === "tank_he") ?? null;
  }
  expect(kinds.has("rifle")).toBe(true);
  expect(blast?.radius).toBe(village.weapons.tank_he.blast_radius_m);
  const tank = frame.own[0];
  expect(tank.weaponPoses.map((p) => p.mount)).toEqual([0, 1]);
  expect(tank.weaponPoses[0].shots).toBeGreaterThan(0);
  const red = published(battle, layout, "red").own[0];
  expect(red.memberIds).toHaveLength(red.members.length);
  expect(new Set(red.memberIds).size).toBe(red.memberIds.length);
  battle.free();
});
