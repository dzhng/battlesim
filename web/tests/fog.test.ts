// @vitest-environment node
// Renderer fog's CPU seam (battle-look slice 14): the mirrors of Rust's sight
// shape, the map word, the static world fog reads, and what reaches the GPU
// from a side's knowledge. The GPU half runs in the `fog` scene.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import {
  initSync,
  Battle,
  WorldView,
  sight_multiplier_vectors,
  world_layout,
} from "@wasm/game_wasm.js";
import { decodeObservation, type ObservationLayout } from "../src/battle/sim/observation";
import { sightMultiplier } from "@packages/battle-renderer/src/sightOverlay";
import {
  fogEyes,
  fogWorld,
  knownOccluders,
  type FogSight,
} from "@packages/battle-renderer/src/frame/fogInputs";
import {
  f16Bits,
  f16Value,
  packFogWord,
  unpackFogWord,
} from "@packages/battle-renderer/src/frame/fogOracle";
import type { WorldExports, WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { labScenario, type LabEvent } from "@apps/battle-lab/src/scenarios";
import sensors from "@fixtures/sensors-lab.json";
import village from "@fixtures/village.json";

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

function staticWorld(map: unknown): { exports: WorldExports; layout: WorldLayout } {
  const view = new WorldView(JSON.stringify(map));
  const exports = {
    positions: view.terrain_positions(),
    indices: view.terrain_indices(),
    triangleSurfaces: view.terrain_triangle_surfaces(),
    props: view.props(),
    water: view.water(),
    forests: view.forests(),
  };
  view.free();
  return { exports, layout: JSON.parse(world_layout()) as WorldLayout };
}

test("the TypeScript sight shape matches sim::sight::multiplier's oracle vectors", () => {
  const rows = sight_multiplier_vectors();
  expect(rows.length).toBeGreaterThan(500);
  for (let i = 0; i < rows.length; i += 5) {
    const [front, side, rear, off, m] = rows.subarray(i, i + 5);
    expect(sightMultiplier({ front, side, rear }, off)).toBeCloseTo(m, 12);
  }
});

test("a map word keeps its horizon to f16, its jump to 1/255 and foliage to half metres", () => {
  for (const h of [-1e4, -0.3125, 0, 0.0625, 1.5, 6e4]) {
    expect(unpackFogWord(packFogWord(h, 0, 0))[0]).toBe(h);
  }
  // Round to nearest even at the f16 midpoint above 1.
  expect(f16Value(f16Bits(1 + 2 ** -11))).toBe(1);
  expect(f16Value(f16Bits(1 + 3 * 2 ** -11))).toBe(1 + 2 ** -9);
  expect(Math.abs(f16Value(f16Bits(0.1234)) - 0.1234)).toBeLessThan(0.1234 * 2 ** -11);
  const [, jump, foliage] = unpackFogWord(packFogWord(0.5, 0.5, 37.3));
  expect(jump).toBeCloseTo(128 / 255, 9);
  expect(foliage).toBe(37.5);
  // Foliage saturates at 8 bits.
  expect(unpackFogWord(packFogWord(0, 1, 500))[2]).toBe(127.5);
});

test("fog reads the simulation's terrain grid and forests", () => {
  const { exports } = staticWorld(sensors);
  const w = fogWorld(exports, village.sensors);
  expect(w.nx * w.ny).toBe(exports.positions.length / 3);
  expect(w.spacing).toBe(sensors.height_grid_m);
  // Vertex (i, j) sits at (i·spacing, j·spacing).
  const k = 3 * w.nx + 5;
  expect(exports.positions[k * 3]).toBe(5 * w.spacing);
  expect(exports.positions[k * 3 + 1]).toBe(3 * w.spacing);
  expect(w.heights[k]).toBe(exports.positions[k * 3 + 2]);
  expect(w.forests.length / 5).toBe(sensors.forests.length);
  // Foliage is stored in 8 bits of half metres: a full block past that is refused.
  expect(() => fogWorld(exports, { ...village.sensors, forest_full_block_m: 200 })).toThrow(
    /forest_full_block_m/,
  );
});

/** Blue's fog input at tick 40 of the sensors lab, with `events`. */
function blueFog(events: LabEvent[]): FogSight {
  const scenario = labScenario(
    sensors,
    [
      { side: "blue", kind: "recon", position: [300, 120] },
      { side: "blue", kind: "tank", position: [300, 70] },
      { side: "red", kind: "rifle", position: [1100, 110], yaw: Math.PI },
    ],
    events,
  );
  const battle = new Battle(scenario, 3);
  for (let t = 0; t < 40; t++) battle.step();
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const length = battle.publish("blue");
  const frame = decodeObservation(
    layout,
    new Float32Array(memory.buffer, battle.publication_ptr(), length).slice(),
  );
  battle.free();
  const { exports, layout: world } = staticWorld(sensors);
  return { eyes: fogEyes(frame.own), occluders: knownOccluders(exports, world, frame.knownProps) };
}

const wall = (center: [number, number]): LabEvent => ({
  tick: 5,
  add_prop: { kind: "wall", center, yaw: 0, half_extents: [0.5, 6, 3] },
});

test("a prop the side has not learned never reaches fog (metamorphic)", () => {
  const base = blueFog([]);
  expect(base.eyes.length).toBe(2);
  // Behind the ridge, out of blue's sight: the truth changes, the input does not.
  const hidden = blueFog([wall([840, 470])]);
  expect(hidden).toEqual(base);
  // In the open before blue's eyes: learned, so it occludes.
  const seen = blueFog([wall([420, 110])]);
  expect(seen.eyes).toEqual(base.eyes);
  expect(seen.occluders).toHaveLength(base.occluders.length + 1);
  expect(seen.occluders.at(-1)).toMatchObject({ x: 420, y: 110, hx: 0.5, hy: 6 });
});

test("a fallen building leaves fog's occluders and its known ruin takes its place", () => {
  const { exports, layout } = staticWorld(village.map);
  const building = layout.propFields.indexOf("id");
  const firstId = exports.props[building];
  const all = knownOccluders(exports, layout, []);
  const ruin = {
    kind: "ruin",
    center: [975, 752] as const,
    yaw: 0,
    half: [15, 12, 1] as const,
    baseZ: 0,
    replaces: firstId,
  };
  const after = knownOccluders(exports, layout, [ruin]);
  expect(after).toHaveLength(all.length);
  expect(after.at(-1)).toMatchObject({ x: 975, y: 752, top: 2 });
  expect(after.some((o) => o.top === all[0].top && o.x === all[0].x && o.y === all[0].y)).toBe(
    false,
  );
});

test("every eye of a garrison is its own fog eye, keyed by unit and slot", () => {
  const eyes = fogEyes([
    {
      id: 7,
      sight: {
        eyes: [
          [1, 2, 3],
          [4, 5, 6],
        ],
        forward: 0.5,
        shape: { front: 1, side: 1, rear: 1 },
        range: 600,
      },
    },
  ]);
  expect(eyes.map((e) => [e.key, e.position])).toEqual([
    ["7:0", [1, 2, 3]],
    ["7:1", [4, 5, 6]],
  ]);
});
