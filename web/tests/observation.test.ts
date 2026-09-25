// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, observation_layout } from "@wasm/game_wasm.js";
import { decodeObservation, type ObservationLayout } from "../src/battle/sim/observation";
import { labScenario } from "@apps/battle-lab/src/scenarios";
import sensors from "@fixtures/sensors-lab.json";
import weaponsMap from "@fixtures/weapons-lab.json";
import deploymentMap from "@fixtures/deployment-lab.json";
import garrisonMap from "@fixtures/garrison-lab.json";
import village from "@fixtures/village.json";
import type { Order } from "../src/battle/sim/protocol";

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

/** Blue's frame, packed before its pointer is read: packing may move the buffer. */
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
    JSON.parse(observation_layout()) as ObservationLayout,
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

test("mount readiness and visible projectile segments decode", () => {
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "tank", position: [200, 250] },
    { side: "red", kind: "tank", position: [400, 250] },
  ]);
  const battle = new Battle(scenario, 3);
  const layout = JSON.parse(observation_layout()) as ObservationLayout;
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
    expect([...p.from, ...p.to].every(Number.isFinite)).toBe(true);
  }
  battle.free();
});

test("deployment progress, its target and the packing state decode", () => {
  const scenario = labScenario(deploymentMap, [
    { side: "blue", kind: "supply", position: [100, 100] },
    { side: "blue", kind: "tank", position: [100, 60] },
  ]);
  const battle = new Battle(scenario, 3);
  const layout = JSON.parse(observation_layout()) as ObservationLayout;
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
  const layout = JSON.parse(observation_layout()) as ObservationLayout;
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

test("garrison phase, progress and a ruin standing in for its building decode", () => {
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
  const layout = JSON.parse(observation_layout()) as ObservationLayout;
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
  expect(ruin!.half[2] * 2).toBe(village.buildings.ruin_height_m);
  battle.free();
});
