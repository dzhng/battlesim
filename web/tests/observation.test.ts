// @vitest-environment node
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, resolve_catalog, village_scenario } from "@wasm/game_wasm.js";
import type { CatalogView } from "@packages/scene-assets/src/units";
import { weaponLabel, weaponRows, type PanelRules } from "../src/battle/present/panelRows";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
import { GroundView } from "../src/battle/sim/ground";
import { sightMultiplier } from "@packages/battle-renderer/src/sightOverlay";
import { labScenario, VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import sensors from "@fixtures/sensors-lab.json";
import weaponsMap from "@fixtures/weapons-lab.json";
import deploymentMap from "@fixtures/deployment-lab.json";
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
const decoders = new WeakMap<Battle, ObservationDecoder>();
function published(battle: Battle, layout: ObservationLayout, side: "blue" | "red" = "blue") {
  const length = battle.publish(side);
  let decoder = decoders.get(battle);
  if (!decoder) decoders.set(battle, (decoder = new ObservationDecoder(layout)));
  return decoder.decode(new Float32Array(memory.buffer, battle.publication_ptr(), length).slice())!;
}

test("packed twin launchers keep separate readiness through to the panel rows", () => {
  const rules = structuredClone(VILLAGE_RULES);
  const docs = rules.catalog as Array<{ units?: Record<string, Record<string, unknown>> }>;
  const at = docs.find((d) => d.units?.at)?.units?.at;
  if (!at) throw new Error("no authored AT team");
  at.body = { squad: { slots: ["atgm_gunner", "atgm_gunner", "at_rifleman"] } };
  rules.weapons.atgm.ammo = 2;
  rules.weapons.atgm.aim_s = 0.5;
  rules.weapons.atgm.reload_s = 2;
  const scenario = JSON.parse(
    labScenario(weaponsMap, [
      { side: "blue", kind: "at", position: [200, 250] },
      { side: "red", kind: "tank", position: [600, 250], engagement: "return_fire_only" },
    ]),
  );
  scenario.rules = rules;
  const battle = new Battle(JSON.stringify(scenario), 4);
  try {
    const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
    let frame = published(battle, layout);
    for (let t = 0; t < 600 && frame.guided.length === 0; t++) {
      battle.step();
      frame = published(battle, layout);
    }
    battle.step();
    frame = published(battle, layout);
    const mounts = frame.own[0].mounts;
    expect(mounts.map((m) => [m.mount, m.ammo])).toEqual([
      [0, [null]],
      [1, [1]],
      [2, [1]],
    ]);
    expect(mounts.slice(1).every((m) => m.guiding && m.reloading === 0 && m.reload > 0)).toBe(true);
    const view = JSON.parse(resolve_catalog(JSON.stringify(rules.catalog))) as CatalogView;
    const equipment = view.units.find((u) => u.id === "at")!.mounts;
    expect(weaponRows(equipment, rules as unknown as PanelRules, mounts).map(weaponLabel)).toEqual([
      "RIFLE ∞",
      "ATGM 1 1",
      "ATGM 2 1",
    ]);
  } finally {
    battle.free();
  }
});

test(
  "a firing report decodes the weapon rows heard: a mount's every row, never a type",
  () => {
    // Blue's tank, behind the ridge from red's scout, shells empty ground.
    const scenario = labScenario(
      sensors,
      [
        { side: "red", kind: "recon", position: [560, 480] },
        { side: "blue", kind: "tank", position: [840, 480] },
      ],
      [],
      [
        {
          tick: 1,
          side: "blue",
          order: { kind: "attack", units: [1], target: { kind: "ground", point: [1000, 480, 0] } },
        },
      ],
    );
    const battle = new Battle(scenario, 3);
    const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
    for (let t = 0; t < 600; t++) {
      battle.step();
      const frame = published(battle, layout, "red");
      const c = frame.contacts[0];
      if (!c) continue;
      expect(c.kind).toBeNull();
      // The cannon is heard whole (AP and HE sound alike); the HMG is its own.
      const cannon = ["tank_ap", "tank_he"];
      const heardCannon = cannon.filter((r) => c.heard.includes(r));
      expect(heardCannon.length === 0 || heardCannon.length === 2).toBe(true);
      expect(c.heard.every((r) => [...cannon, "hmg"].includes(r)) && c.heard.length > 0).toBe(true);
      return;
    }
    throw new Error("the tank never fired");
  },
  BATTLE_TEST_TIMEOUT_MS,
);

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
  const frame = new ObservationDecoder(
    JSON.parse(battle.observation_layout()) as ObservationLayout,
  ).decode(new Float32Array(memory.buffer, battle.publication_ptr(), length).slice())!;
  expect(frame.tick).toBe(15);
  expect(frame.own.map((u) => u.kind)).toEqual(["recon", "rifle"]);
  expect(frame.own[1].members).toHaveLength(8);
  expect(frame.identified.map((e) => e.kind)).toEqual(["tank"]);
  expect(frame.contacts).toHaveLength(1);
  const c = frame.contacts[0];
  expect(c.source).toBe("firing");
  expect(c.primaryLabel).toBe(true);
  // The lab emitter's shot is of no weapon, and a report names no type.
  expect([c.kind, c.heard]).toEqual([null, []]);
  // 3 × a rifle squad's footprint: half its 12 m spread plus a soldier's 0.3 m.
  expect(c.radius).toBeCloseTo(18.9, 4);
  expect(Math.hypot(c.center[0] - 840, c.center[1] - 480)).toBeLessThanOrEqual(c.radius);
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
  expect(rifle.suppression).toBe("none");
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
  const s = UNITS.type("tank").sensors;
  // Float32 transport: values survive to single precision.
  expect(tank.sight.forward).toBeCloseTo(0.5, 6);
  for (const k of ["front", "side", "rear"] as const)
    expect(tank.sight.shape[k]).toBeCloseTo(s.sight_shape[k], 6);
  expect(tank.sight.range).toBe(s.ground_m);
  expect(tank.sight.eyes).toHaveLength(1);
  const [x, y, z] = tank.sight.eyes[0];
  expect([x, y]).toEqual([tank.position[0], tank.position[1]]);
  expect(z).toBeCloseTo(tank.position[2] + UNITS.hull("tank")!.eye_m, 4);
  expect(rifle.sight.shape).toEqual({ front: 1, side: 1, rear: 1 });
  expect(rifle.sight.range).toBe(UNITS.type("rifle").sensors.ground_m);
  // The published reach matches the shape's anchors.
  const reach = (off: number) => tank.sight.range * sightMultiplier(tank.sight.shape, off);
  expect(reach(0)).toBeCloseTo(s.ground_m * s.sight_shape.front, 3);
  expect(reach(Math.PI / 2)).toBeCloseTo(s.ground_m * s.sight_shape.side, 3);
  expect(reach(Math.PI)).toBeCloseTo(s.ground_m * s.sight_shape.rear, 3);
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
  const ticks = UNITS.type("supply").capabilities.deploy!.seconds * village.tick_hz;
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
  const battle = new Battle(village_scenario(JSON.stringify(VILLAGE_RULES), "ordinary"), 1);
  battle.step();
  expect(published(battle, layout, "red").encounter).toEqual({ heldS: 0, result: "running" });
  expect(layout.encounterResults).toEqual(["running", "captured", "defeated", "inconclusive"]);
  battle.free();
});

test("every frozen animation field and ground value decodes, integers exact past 2^24", () => {
  const lab = new Battle(labScenario(weaponsMap, []), 1);
  const layout = JSON.parse(lab.observation_layout()) as ObservationLayout;
  lab.free();
  const big = 2 ** 24 + 1;
  const vectors: Record<
    string,
    {
      bits: number[];
      patch: {
        cells: Array<{
          cell: number;
          crater: number;
          scorch: number;
          tracks: number;
          trampled: number;
          cleared: number;
        }>;
      };
    }
  > = JSON.parse(
    readFileSync(
      new URL(
        "../../specs/city-maps/assets/ground-transport/animation-codec-vectors.json",
        import.meta.url,
      ),
      "utf8",
    ),
  );
  // The frozen native vectors pin wide animation words independently of a
  // test-only wasm encoder. The intentional ground-tail rewrite is below.
  layout.ground.cols = 18000;
  layout.ground.rows = 18000;
  const decodeVector = (phase: string) => {
    const vector = vectors[phase];
    const previous = new Float32Array(Uint32Array.from(vector.bits).buffer);
    const patch = vector.patch;
    const sorted = [...patch.cells].sort((a, b) => {
      const tile = (c: { cell: number }) =>
        Math.floor(Math.floor(c.cell / 18000) / 16) * 1125 + Math.floor((c.cell % 18000) / 16);
      return tile(a) - tile(b) || a.cell - b.cell;
    });
    const record = previous.slice(0, previous.length - patch.cells.length * 4);
    const put = (name: string, value: number) => {
      record[layout.header.indexOf(name)] = value;
    };
    put(layout.ground.count, sorted.length);
    put("groundFull", 1);
    put("groundBase", 0);
    const runs = sorted.flatMap((c) => {
      const x = c.cell % 18000,
        y = Math.floor(c.cell / 18000);
      return [
        Math.floor(y / 16) * 1125 + Math.floor(x / 16),
        (y % 16) * 16 + (x % 16) + 256,
        c.crater + c.scorch * 256,
        c.tracks + c.trampled * 256 + c.cleared * 65536,
      ];
    });
    return new ObservationDecoder(layout).decode(new Float32Array([...record, ...runs]))!;
  };
  const o = decodeVector("base");
  expect([o.own[0].kind, o.identified[0].kind, o.corpses[0].kind]).toEqual(["rifle", "tank", "at"]);
  expect(o.knownProps.map((p) => p.kind)).toEqual(["tank_wreck"]);
  const ground = new GroundView(layout.ground);
  ground.applyRuns(o.groundPatch);
  const cell = big + 12;
  expect(ground.cell(cell % 18000, Math.floor(cell / 18000))).toEqual({
    crater: 255,
    scorch: 0,
    tracks: 17,
    trampled: 200,
  });
  expect(ground.cell(0, 0)).toEqual({ crater: 1, scorch: 2, tracks: 255, trampled: 4 });
  expect(ground.isCleared(0, 0)).toBe(true);
  expect(o.own[0].memberIds).toEqual([3, big + 2]);
  expect(o.own[0].memberSlots).toEqual([0, 7]);
  expect(o.own[0].memberOrders).toEqual([
    { spot: [7, 8], coverNow: null, coverThere: "heavy" },
    { spot: [9, 10], coverNow: "light", coverThere: "medium" },
  ]);
  expect(o.own[0].finalFacing).toBe(1.25);
  expect(o.own[0].suppression).toBe("pinned");
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
  // A guided id past a float's exact integers travels as limbs.
  expect(o.guided).toEqual([
    { id: big + 6, position: [1, 2, 3], point: [4, 5, 6], supported: true },
  ]);
  expect(o.blasts).toEqual([{ point: [5, 6, 0.5], radius: 12, kind: layout.roundKinds[3] }]);
  expect(o.corpses).toEqual([
    { position: [2, 3, 0], own: false, soldier: big + 10, kind: "at", slot: 2, yaw: -1.25 },
  ]);
  expect(o.own[0].garrison).toBeNull();
  expect(o.knownProps[0].replaces).toBeNull();
  expect(layout.garrisonPhases).toEqual(["entering", "inside", "exiting"]);
  // Exact wire cases don't depend on a particular tank winning a demolition battle.
  for (const [phase, progress] of [
    ["entering", 0.25],
    ["inside", 1],
    ["exiting", 0.75],
  ] as const) {
    const garrison = { building: 0, phase, progress, center: [360, 250], half: [12, 12] };
    const decoded = decodeVector(phase);
    expect(decoded.own[0].garrison).toEqual(garrison);
    expect(decoded.knownProps[0]).toEqual({
      kind: "ruin",
      center: [360, 250],
      yaw: 0.5,
      half: [12, 12, 1.5],
      baseZ: 1,
      replaces: 0,
      destroyed: false,
    });
  }
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
