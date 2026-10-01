// @vitest-environment node
// `fixtures/parity/` pins this build's sparse ground and foliage exports; their
// equivalence to the dense originals was proven at tag city-maps-evidence-2026-09-30.
import { readFileSync, writeFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, WorldView } from "@wasm/game_wasm.js";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
import { cellPatchRuns, canonicalGround } from "./groundRuns";
import { GroundView } from "../src/battle/sim/ground";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import ordered from "../../fixtures/parity/ground/ordered-patches.json";
import foliage from "../../fixtures/parity/ground/foliage.json";
import foliageMaps from "../../fixtures/parity/buildings/foliage-cutover-inputs.json";
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
test("sparse native truth matches frozen digests and every ordered learned patch", () => {
  const battle = new Battle(JSON.stringify(ordered.scenario), ordered.seed);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  let next = 0;
  for (let t = 0; t < 150; t++) {
    battle.step();
    if (t % 7 === 3) continue;
    const original = ordered.results[next++];
    const p = record(battle, layout, "blue");
    // Named simulation changes update digests and the ground their movement marks.
    if (process.env.BLESS_PARITY) {
      const { side, ...ground } = canonicalGround(p, layout.ground.cols);
      expect(side).toBe("blue");
      Object.assign(original, { digest: battle.digest(), ...ground });
    }
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
  const blue = canonicalGround(record(battle, layout, "blue"), layout.ground.cols);
  const red = canonicalGround(record(battle, layout, "red"), layout.ground.cols);
  if (process.env.BLESS_PARITY) {
    Object.assign(ordered.blue, blue);
    Object.assign(ordered.red, red);
  }
  expect(blue).toEqual(ordered.blue);
  expect(red).toEqual(ordered.red);
  battle.free();
  if (process.env.BLESS_PARITY)
    writeFileSync(
      new URL("../../fixtures/parity/ground/ordered-patches.json", import.meta.url),
      JSON.stringify(ordered),
    );
}, 30000);
test("sparse foliage exports preserve original static and side-cleared cells", () => {
  for (const original of foliage) {
    const world = new WorldView(
      JSON.stringify(foliageMaps[original.id as keyof typeof foliageMaps]),
      JSON.stringify(GAME_RULES),
    );
    const rows = (f: Float32Array) => ({
      header: [...f.subarray(0, 3)],
      rows: Array.from({ length: (f.length - 3) / 4 }, (_, k) => [
        ...f.subarray(3 + k * 4, 7 + k * 4),
      ]),
    });
    // BLESS_PARITY=1 rewrites these exports (a named behaviour change only).
    if (process.env.BLESS_PARITY) original.original = rows(world.foliage());
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
    const known = rows(world.foliage_cleared(ground.clearedRuns(), original.cols, original.cellM));
    if (process.env.BLESS_PARITY) original.known = known;
    expect(known).toEqual(original.known);
    world.free();
  }
  if (process.env.BLESS_PARITY)
    writeFileSync(
      new URL("../../fixtures/parity/ground/foliage.json", import.meta.url),
      JSON.stringify(foliage),
    );
});
