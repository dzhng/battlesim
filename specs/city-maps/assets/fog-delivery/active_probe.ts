import { readFileSync, writeFileSync } from "node:fs";
import { initSync, Battle } from "../../../../web/src/wasm/game_wasm.js";
const oracle = JSON.parse(readFileSync("specs/city-maps/assets/fog-delivery/oracle.json", "utf8"));
const original = JSON.parse(
  readFileSync("specs/city-maps/assets/fog-delivery/original-wire.json", "utf8"),
);
const memory = initSync({ module: readFileSync("web/src/wasm/game_wasm_bg.wasm") }).memory;
const battle = new Battle(JSON.stringify(oracle.scenario), oracle.seed);
const layout = JSON.parse(battle.observation_layout());
const rows = [];
try {
  for (let i = 0; i < oracle.rows.length; i++) {
    const r = oracle.rows[i];
    battle.step();
    if (r.resync) battle.resync_observation();
    const count = battle.publish(r.side);
    const record = new Float32Array(memory.buffer, battle.publication_ptr(), count);
    const field = (name: string) => record[layout.header.indexOf(name)];
    rows.push({
      tick: r.decoded.tick,
      originalBytes: original.record_bytes[i],
      candidateBytes: count * 4,
      fogPayloadBytes: field("fogFloats") * 4,
      full: field("fogFull") === 1,
    });
  }
} finally {
  battle.free();
}
writeFileSync(
  "specs/city-maps/assets/fog-delivery/active.json",
  JSON.stringify({ runtime: "Bun " + Bun.version, rows }, null, 2) + "\n",
);
console.log(
  JSON.stringify({
    ticks: rows.length,
    originalBytes: rows.reduce((sum, r) => sum + r.originalBytes, 0),
    candidateBytes: rows.reduce((sum, r) => sum + r.candidateBytes, 0),
    fogFull: rows.filter((r) => r.full).length,
    fogDelta: rows.filter((r) => !r.full).length,
    unchanged: rows.filter((r) => !r.full && !r.fogPayloadBytes).length,
    largestSparseFogBytes: Math.max(...rows.filter((r) => !r.full).map((r) => r.fogPayloadBytes)),
  }),
);
