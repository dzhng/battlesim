// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, observation_layout } from "@wasm/game_wasm.js";
import { decodeObservation, type ObservationLayout } from "../src/battle/sim/observation";
import { labScenario } from "@apps/battle-lab/src/scenarios";
import sensors from "@fixtures/sensors-lab.json";
import weaponsMap from "@fixtures/weapons-lab.json";

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

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
  battle.free();
});

test("mount readiness and visible projectile segments decode", () => {
  const scenario = labScenario(weaponsMap, [
    { side: "blue", kind: "tank", position: [200, 250] },
    { side: "red", kind: "tank", position: [400, 250] },
  ]);
  const battle = new Battle(scenario, 3);
  const layout = JSON.parse(observation_layout()) as ObservationLayout;
  const decode = () =>
    decodeObservation(
      layout,
      new Float32Array(memory.buffer, battle.publication_ptr(), battle.publish("blue")).slice(),
    );
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
