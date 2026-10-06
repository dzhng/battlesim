// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { initSync, Battle } from "@wasm/game_wasm.js";
import record from "@fixtures/parity/skirmish-purchases.json";
import { ObservationDecoder, type ObservationLayout } from "@web/battle/sim/observation";

test("native purchases replay in WASM and publish the side wallet, queue and physical entry", () => {
  const memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
  const battle = new Battle(JSON.stringify(record.scenario), record.seed);
  try {
    const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
    const decoder = new ObservationDecoder(layout);
    const frame = (side: "blue" | "red") => {
      const length = battle.publish(side);
      return decoder.decode(
        new Float32Array(memory.buffer, battle.publication_ptr(), length).slice(),
      )!;
    };
    expect(JSON.parse(battle.accept(JSON.stringify(record.commands[0]))).error).toBeNull();
    battle.step();
    const prep = frame("blue").skirmish!;
    expect(prep.credits).toBe(800);
    expect(prep.pending).toEqual([
      { id: 0, kind: "tank", destination: [400, 300], confirmedTick: 1, blocked: false },
    ]);
    expect(frame("red").skirmish).toMatchObject({ credits: 1000, pending: [] });
    for (const command of record.commands.slice(1))
      expect(JSON.parse(battle.accept(JSON.stringify(command))).error).toBeNull();
    for (let t = 0; t < record.ticks - 1; t++) battle.step();
    const paired = new Battle(JSON.stringify(record.scenario), record.seed);
    try {
      for (const command of record.commands) paired.accept(JSON.stringify(command));
      for (let t = 0; t < record.ticks; t++) paired.step();
      expect(paired.digest()).toBe(record.digest);
    } finally {
      paired.free();
    }
    const active = frame("blue");
    expect(active.skirmish).toMatchObject({ phase: "active", occupiedSlots: 1, pending: [] });
    expect(active.own).toHaveLength(1);
    expect(active.own[0].position[1]).toBeGreaterThan(10);
    expect(active.own[0].position[1]).toBeLessThan(100);
  } finally {
    battle.free();
  }
});
