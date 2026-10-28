// @vitest-environment node
// Fog of war's seam with the presentation crate, which decides it
// (`presentation::fog::sight`, whose own tests hold it to the browser's
// former TypeScript): the mirror of Rust's sight shape, what the static
// world gives it, and that a side's fog sees only what it has learned. The
// GPU half runs in the `fog` scene.
import { TEST_RULES } from "./catalog";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import {
  initSync,
  Battle,
  WorldView,
  sight_multiplier_vectors,
  world_layout,
} from "@wasm/game_wasm.js";
import type { ObservationLayout } from "../src/battle/sim/observationView";
import { ScenePresentation, loadPresentation } from "../src/battle/sim/presentation";
import { sightMultiplier } from "@packages/battle-renderer/src/sightOverlay";
import { CAMERA_WORDS, FogParamsAt } from "@packages/battle-renderer/src/records.gen";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";
import { gameBiome } from "@apps/battle-lab/src/gameBiome";
import { labScenario } from "./catalog";
import { type LabEvent } from "@apps/battle-lab/src/scenarios";
import { loadMap } from "@web/maps/node";
import game from "@fixtures/game.json";

await loadPresentation();

const sensors = loadMap("sensors").definition;

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

function staticWorld(map: unknown): { exports: WorldExports; layout: WorldLayout } {
  const view = new WorldView(JSON.stringify(map), JSON.stringify(TEST_RULES));
  const exports = readWorldExports(view);
  view.free();
  return {
    exports,
    layout: JSON.parse(world_layout(JSON.stringify(TEST_RULES))) as WorldLayout,
  };
}

/** A camera over the map: what it frames is not the question. */
const OVERHEAD = {
  eye: [0, 0, 800],
  target: [1, 1, 0],
  up: [0, 0, 1],
  fovY: 0.8,
  near: 1,
  width: 1920,
  height: 1080,
};

/** A presentation of `map`'s ground with fog on, fed `publication` (none:
 *  no battle), and its first fog upload. */
function fogOver(
  map: unknown,
  sensorRules: object,
  publication?: { layout: ObservationLayout; words: Float32Array },
) {
  const decoder = new ScenePresentation(publication?.layout);
  const { exports, layout } = staticWorld(map);
  decoder.setGround(
    { exports, layout: JSON.stringify(layout), biome: gameBiome, family: null },
    null,
  );
  if (publication) decoder.decode(publication.words);
  decoder.frame(0, OVERHEAD, { selected: [], xray: false, ghosts: [] }, 0);
  decoder.setFog({ sensors: sensorRules });
  const upload = decoder.presentedGround()!.prepare_fog(new Float32Array(CAMERA_WORDS), 64, 64, 0)!;
  return { decoder, upload, exports };
}

test("the TypeScript sight shape matches sim::sight::multiplier's oracle vectors", () => {
  const rows = sight_multiplier_vectors();
  expect(rows.length).toBeGreaterThan(500);
  for (let i = 0; i < rows.length; i += 5) {
    const [front, side, rear, off, m] = rows.subarray(i, i + 5);
    expect(sightMultiplier({ front, side, rear }, off)).toBeCloseTo(m, 12);
  }
});

test("fog reads the simulation's terrain grid and the foliage its trees give", () => {
  const { decoder, upload, exports } = fogOver(sensors, game.sensors);
  try {
    const words = upload.params();
    const floats = new Float32Array(words.buffer);
    expect(floats[FogParamsAt.height_spacing]).toBe(sensors.height_grid_m);
    expect((words[FogParamsAt.height_nx] - 1) * sensors.height_grid_m).toBe(sensors.size[0]);
    expect((words[FogParamsAt.height_ny] - 1) * sensors.height_grid_m).toBe(sensors.size[1]);
    expect([words[FogParamsAt.height_nx], words[FogParamsAt.height_ny]]).toEqual([
      exports.terrain.nx,
      exports.terrain.ny,
    ]);
    // The foliage grid: a canopy and a depth per 8 m cell, some of them
    // under the lab's trees.
    expect(floats[FogParamsAt.foliage_cell_m]).toBe(8);
    const cells = upload.foliage()!;
    expect(cells).toEqual(exports.foliage.subarray(3));
    let canopied = 0;
    for (let i = 2; i < cells.length; i += 4) if (cells[i] > 0) canopied++;
    expect(canopied).toBeGreaterThan(0);
  } finally {
    upload.free();
    decoder.free();
  }
  // Foliage is stored in 8 bits: a full block past that is refused.
  expect(() => fogOver(sensors, { ...game.sensors, foliage_full_block: 2 })).toThrow(
    /foliage_full_block/,
  );
});

/** Blue's fog at tick 40 of the sensors lab, with `events`: the eyes it
 *  sees from and the boxes it is cut by. */
function blueFog(events: LabEvent[]) {
  const scenario = labScenario(
    sensors,
    [
      { side: "blue", kind: "test_recon", position: [300, 120] },
      { side: "blue", kind: "test_tank", position: [300, 70] },
      { side: "red", kind: "test_rifle", position: [1100, 110], yaw: Math.PI },
    ],
    events,
  );
  const battle = new Battle(scenario, 3);
  for (let t = 0; t < 40; t++) battle.step();
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const length = battle.publish("blue");
  const words = new Float32Array(memory.buffer, battle.publication_ptr(), length).slice();
  battle.free();
  const { decoder, upload } = fogOver(sensors, game.sensors, { layout, words });
  try {
    return { eyes: decoder.fogEyes(), occluders: decoder.fogOccluders() };
  } finally {
    upload.free();
    decoder.free();
  }
}

const wall = (center: [number, number]): LabEvent => ({
  tick: 5,
  add_prop: { kind: "wall", center, yaw: 0, half_extents: [0.5, 6, 3] },
});

test("a prop the side has not learned never reaches fog (metamorphic)", () => {
  const base = blueFog([]);
  expect(base.eyes.length).toBe(2);
  // Behind the ridge, out of blue's sight: the truth changes, the fog does not.
  const hidden = blueFog([wall([840, 470])]);
  expect(hidden).toEqual(base);
  // In the open before blue's eyes: learned, so it occludes.
  const seen = blueFog([wall([420, 110])]);
  expect(seen.eyes).toEqual(base.eyes);
  expect(seen.occluders).toHaveLength(base.occluders.length + 1);
  expect(seen.occluders.at(-1)).toMatchObject({ x: 420, y: 110, hx: 0.5, hy: 6 });
});
