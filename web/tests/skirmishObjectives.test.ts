// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { initSync, Battle } from "@wasm/game_wasm.js";
import record from "@fixtures/parity/skirmish-objectives.json";
import { ObservationDecoder, type ObservationLayout } from "@web/battle/sim/observation";

test("objective capture and both scores publish exactly the native timeline", () => {
  const memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
  const battle = new Battle(JSON.stringify(record.scenario), record.seed);
  try {
    const decoder = new ObservationDecoder(
      JSON.parse(battle.observation_layout()) as ObservationLayout,
    );
    for (const command of record.commands)
      expect(JSON.parse(battle.accept(JSON.stringify(command))).error).toBeNull();
    for (const row of record.rows) {
      while (battle.tick() < row.tick) battle.step();
      expect(battle.digest()).toBe(row.digest);
      const length = battle.publish("blue");
      const view = decoder.decode(
        new Float32Array(memory.buffer, battle.publication_ptr(), length).slice(),
      )!.skirmish!;
      expect(view.result).toEqual(row.result);
      for (let side = 0; side < 2; side++)
        expect(view.scores[side]).toBeCloseTo(row.scores[side], 5);
      expect(view.objectives).toHaveLength(row.objectives.length);
      for (let i = 0; i < row.objectives.length; i++) {
        const native = row.objectives[i];
        expect(view.objectives[i]).toMatchObject({
          id: native.id,
          center: native.center,
          radiusM: native.radius_m,
          owner: native.owner,
          capturing: native.capturing,
          contested: native.contested,
        });
        expect(view.objectives[i].captureProgress).toBeCloseTo(native.capture_progress, 6);
      }
    }
  } finally {
    battle.free();
  }
});
