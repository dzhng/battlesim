// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { initSync, Battle } from "@wasm/game_wasm.js";
import record from "@fixtures/parity/exact-numbers.json";

// The record's numbers are ones serde_json's default reader takes one ULP off.
// The browser's simulation reaches native's digests only if its build reads
// them exactly, as native's does.
test("the browser's simulation reads a battle's numbers as native does", () => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  const battle = new Battle(JSON.stringify(record.scenario), record.seed);
  try {
    for (const row of record.rows) {
      while (battle.tick() < row.tick) battle.step();
      expect(battle.digest()).toBe(row.digest);
    }
  } finally {
    battle.free();
  }
});
