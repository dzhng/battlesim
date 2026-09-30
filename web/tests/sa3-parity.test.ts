// @vitest-environment node
// Immutable original outputs are the oracle for a representation-only change.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, WorldView } from "@wasm/game_wasm.js";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
import { cellPatchRuns, canonicalGround } from "./groundRuns";
import { GroundView } from "../src/battle/sim/ground";
import { VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import ordered from "../../specs/city-maps/assets/ground-baseline/ordered-patches.json";
import foliage from "../../specs/city-maps/assets/ground-baseline/foliage.json";
let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});
const decoders = new WeakMap<Battle, ObservationDecoder>();
function record(b: Battle, layout: ObservationLayout, side: "blue" | "red") {
  const length = b.publish(side);
  let decoder = decoders.get(b);
  if (!decoder) {
    decoder = new ObservationDecoder(layout);
    decoders.set(b, decoder);
  }
  const observation = decoder.decode(
    new Float32Array(memory.buffer, b.publication_ptr(), length).slice(),
  );
  expect(observation).not.toBeNull();
  return observation!.groundPatch;
}
test("sparse native truth preserves original digests and every ordered learned patch", () => {
  const scenario = structuredClone(ordered.scenario);
  const { fog_cell_m, ...sensors } = scenario.rules.sensors;
  const battle = new Battle(
    JSON.stringify({
      ...scenario,
      map: { ...scenario.map, fog_cell_m },
      rules: { ...scenario.rules, sensors },
    }),
    ordered.seed,
  );
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  let next = 0;
  for (let t = 0; t < 150; t++) {
    battle.step();
    if (t % 7 === 3) continue;
    const original = ordered.results[next++];
    const p = record(battle, layout, "blue");
    expect({
      tick: t + 1,
      digest: battle.digest(),
      ...canonicalGround(p, layout.ground.cols),
    }).toEqual({
      side: "blue",
      ...original,
    });
  }
  battle.resync_observation();
  expect(canonicalGround(record(battle, layout, "blue"), layout.ground.cols)).toEqual(ordered.blue);
  expect(canonicalGround(record(battle, layout, "red"), layout.ground.cols)).toEqual(ordered.red);
  battle.free();
}, 30000);
test("sparse foliage exports preserve original static and side-cleared cells", () => {
  for (const original of foliage) {
    const world = new WorldView(
      JSON.stringify({ ...original.map, fog_cell_m: 8 }),
      JSON.stringify(VILLAGE_RULES),
    );
    const rows = (f: Float32Array) => ({
      header: [...f.subarray(0, 3)],
      rows: Array.from({ length: (f.length - 3) / 4 }, (_, k) => [
        ...f.subarray(3 + k * 4, 7 + k * 4),
      ]),
    });
    expect(rows(world.foliage())).toEqual(original.original);
    const ground = new GroundView({
      cellM: original.cellM,
      cols: original.cols,
      rows: original.cols,
    });
    ground.applyRuns(
      cellPatchRuns(ground.cols, {
        epoch: 1,
        side: "blue",
        baseRevision: 0,
        revision: 1,
        full: true,
        cells: Uint32Array.from(original.cleared),
        marks: new Uint8Array(original.cleared.length * 4),
        cleared: new Uint8Array(original.cleared.length).fill(255),
      }),
    );
    expect(
      rows(world.foliage_cleared(ground.clearedRuns(), original.cols, original.cellM)),
    ).toEqual(original.known);
    world.free();
  }
});
