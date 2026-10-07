// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { Battle, initSync } from "@wasm/game_wasm.js";
import { labScenario, TEST_RULES } from "./catalog";
import { projectileReviewRules } from "@apps/battle-lab/src/projectileReview";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

test("review combat keeps firing with unlimited reserves and leaves every body alive", () => {
  const rules = projectileReviewRules(TEST_RULES);
  const battle = new Battle(
    labScenario(
      {
        size: [500, 500],
        fog_cell_m: 8,
        height_grid_m: 4,
        slope_cutoff_deg: 35,
        relief: [],
        forests: [],
        props: [],
      },
      [
        { side: "blue", kind: "test_tank", position: [100, 100] },
        { side: "blue", kind: "test_rifle", position: [100, 180] },
        { side: "blue", kind: "test_at", position: [100, 260] },
        { side: "red", kind: "test_tank", position: [180, 100], yaw: Math.PI },
        { side: "red", kind: "test_rifle", position: [180, 180], yaw: Math.PI },
        { side: "red", kind: "test_at", position: [180, 260], yaw: Math.PI },
      ],
      [],
      [],
      rules,
    ),
    12,
  );
  const decoder = new ObservationDecoder(
    JSON.parse(battle.observation_layout()) as ObservationLayout,
  );
  const observe = (side: string) => {
    const length = battle.publish(side);
    return decoder.decode(
      new Float32Array(memory.buffer, battle.publication_ptr(), length).slice(),
    )!;
  };
  try {
    battle.step();
    const bodies = (side: string) =>
      observe(side).own.map((u) => [u.id, u.hp, u.memberIds, u.memberHp]);
    const initial = { blue: bodies("blue"), red: bodies("red") };
    for (let tick = 0; tick < 600; tick++) battle.step();
    for (const side of ["blue", "red"] as const) {
      const o = observe(side);
      expect(bodies(side)).toEqual(initial[side]);
      expect(o.own.flatMap((u) => u.mounts).every((m) => m.ammo.every((a) => a === null))).toBe(
        true,
      );
      expect(o.own.flatMap((u) => u.weaponPoses).some((m) => m.shots > 0)).toBe(true);
    }
  } finally {
    battle.free();
  }
});
