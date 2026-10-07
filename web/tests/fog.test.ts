// @vitest-environment node
// Renderer fog's CPU seam: the mirrors of Rust's sight
// shape, the map word, the static world fog reads, and what reaches the GPU
// from a side's knowledge. The GPU half runs in the `fog` scene.
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
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
import { sightMultiplier } from "@packages/battle-renderer/src/sightOverlay";
import {
  fogEyes,
  fogWorld,
  knownOccluders,
  mapOccluders,
  wholeWords,
  WHOLE_BOX_WORDS,
  WHOLE_TEXTURE_WIDTH,
  type FogOccluder,
  type FogSight,
} from "@packages/battle-renderer/src/frame/fogInputs";
import {
  f16Bits,
  f16Value,
  packFogWord,
  unpackFogWord,
} from "@packages/battle-renderer/src/frame/fogOracle";
import { groundHeight } from "@packages/battle-renderer/src/terrain/terrainGrid";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";
import { labScenario } from "./catalog";
import { type LabEvent } from "@apps/battle-lab/src/scenarios";
import { loadMap } from "@web/maps/node";
import game from "@fixtures/game.json";

const sensors = loadMap("sensors").definition;
const villageMap = loadMap("village").definition;

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

test("the TypeScript sight shape matches sim::sight::multiplier's oracle vectors", () => {
  const rows = sight_multiplier_vectors();
  expect(rows.length).toBeGreaterThan(500);
  for (let i = 0; i < rows.length; i += 5) {
    const [front, side, rear, off, m] = rows.subarray(i, i + 5);
    expect(sightMultiplier({ front, side, rear }, off)).toBeCloseTo(m, 12);
  }
});

test("a map word keeps its horizon to f16, its jump to 1/255 and foliage depth to 1/200", () => {
  for (const h of [-1e4, -0.3125, 0, 0.0625, 1.5, 6e4]) {
    expect(unpackFogWord(packFogWord(h, 0, 0))[0]).toBe(h);
  }
  // Round to nearest even at the f16 midpoint above 1.
  expect(f16Value(f16Bits(1 + 2 ** -11))).toBe(1);
  expect(f16Value(f16Bits(1 + 3 * 2 ** -11))).toBe(1 + 2 ** -9);
  expect(Math.abs(f16Value(f16Bits(0.1234)) - 0.1234)).toBeLessThan(0.1234 * 2 ** -11);
  const [, jump, foliage] = unpackFogWord(packFogWord(0.5, 0.5, 0.3731));
  expect(jump).toBeCloseTo(128 / 255, 9);
  expect(foliage).toBeCloseTo(0.375, 9);
  // Foliage saturates at 8 bits.
  expect(unpackFogWord(packFogWord(0, 1, 5))[2]).toBeCloseTo(1.275, 9);
});

test("fog reads the simulation's terrain grid and the foliage its trees give", () => {
  const { exports } = staticWorld(sensors);
  const w = fogWorld(exports, game.sensors);
  expect(w.spacing).toBe(sensors.height_grid_m);
  expect((w.nx - 1) * w.spacing).toBe(sensors.size[0]);
  expect((w.ny - 1) * w.spacing).toBe(sensors.size[1]);
  // Mesh coalescing and sampled height pages describe the same surface.
  for (let k = 0; k < exports.positions.length; k += 3)
    expect(groundHeight(w, exports.positions[k], exports.positions[k + 1])).toBeCloseTo(
      exports.positions[k + 2],
      5,
    );
  // The foliage grid: its header, then a canopy and a depth per 8 m cell,
  // some of them under the lab's trees.
  const [nx, ny, cell] = w.foliage;
  expect(cell).toBe(8);
  expect(w.foliage.length).toBeLessThan(3 + 2 * nx * ny);
  let canopied = 0;
  for (let i = 5; i < w.foliage.length; i += 4) if (w.foliage[i] > 0) canopied++;
  expect(canopied).toBeGreaterThan(0);
  // Foliage is stored in 8 bits: a full block past that is refused.
  expect(() => fogWorld(exports, { ...game.sensors, foliage_full_block: 2 })).toThrow(
    /foliage_full_block/,
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
  const frame = new ObservationDecoder(layout).decode(
    new Float32Array(memory.buffer, battle.publication_ptr(), length).slice(),
  )!;
  battle.free();
  const { exports, layout: world } = staticWorld(sensors);
  return {
    eyes: fogEyes(frame.own),
    occluders: knownOccluders(mapOccluders(exports, world), frame.knownProps),
  };
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
  const { exports, layout } = staticWorld(villageMap);
  const firstId =
    exports.props[layout.propFields.indexOf("idLo")] +
    exports.props[layout.propFields.indexOf("idHi")] * 2 ** layout.limbBits;
  const all = knownOccluders(mapOccluders(exports, layout), []);
  const ruin = {
    kind: "ruin",
    center: [975, 752] as const,
    yaw: 0,
    half: [15, 12, 1] as const,
    baseZ: 0,
    replaces: firstId,
    authoredProp: firstId,
  };
  const after = knownOccluders(mapOccluders(exports, layout), [ruin]);
  expect(after).toHaveLength(all.length);
  expect(after.at(-1)).toMatchObject({ x: 975, y: 752, top: 2 });
  expect(after.some((o) => o.top === all[0].top && o.x === all[0].x && o.y === all[0].y)).toBe(
    false,
  );
});

test("every eye of a garrison is its own fog eye, keyed by unit and eye index", () => {
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

test("every point inside a structure's box finds that structure through the whole-fog grid", () => {
  // Turned, touching and far-apart boxes, as a village's buildings, ruins and walls.
  const boxes: FogOccluder[] = [
    { x: 975, y: 752, yaw: 0, hx: 15, hy: 12, base: 10, top: 18 },
    { x: 1047, y: 814, yaw: 0.6, hx: 18, hy: 9, base: 11, top: 21 },
    { x: 1000, y: 752, yaw: 0, hx: 10, hy: 12, base: 10, top: 12 },
    { x: 200, y: 1400, yaw: 2.2, hx: 0.25, hy: 20, base: 3, top: 5 },
  ];
  const margin = 0.2;
  const { words, params, flagsRow, flagRows } = wholeWords(boxes, margin);
  const floats = new Float32Array(words.buffer);
  const lookup = (x: number, y: number): number[] => {
    const cx = Math.floor((x - params.wholeOrigin[0]) / params.wholeCellM);
    const cy = Math.floor((y - params.wholeOrigin[1]) / params.wholeCellM);
    if (cx < 0 || cy < 0 || cx >= params.wholeNx || cy >= params.wholeNy) return [];
    const cell = words[cy * params.wholeNx + cx];
    const start = params.wholeItemsBase + (cell >>> 8);
    return [...words.subarray(start, start + (cell & 0xff))];
  };
  boxes.forEach((b, i) => {
    // The box as the fragment reads it back.
    const o = params.wholeBoxesBase + i * WHOLE_BOX_WORDS;
    expect([...floats.subarray(o, o + WHOLE_BOX_WORDS)]).toEqual(
      [b.x, b.y, Math.cos(b.yaw), Math.sin(b.yaw), b.hx, b.hy, b.base, b.top].map(Math.fround),
    );
    // Its corners, grown by the margin, and points across it.
    for (const u of [-1, -0.5, 0, 0.5, 1])
      for (const v of [-1, -0.5, 0, 0.5, 1]) {
        const lu = u * (b.hx + margin);
        const lv = v * (b.hy + margin);
        const x = b.x + lu * Math.cos(b.yaw) - lv * Math.sin(b.yaw);
        const y = b.y + lu * Math.sin(b.yaw) + lv * Math.cos(b.yaw);
        expect(lookup(x, y)).toContain(i);
      }
  });
  // Seen flags start on a row of their own, one per box, and start unseen.
  expect(params.wholeFlagsBase).toBe(flagsRow * WHOLE_TEXTURE_WIDTH);
  expect(flagRows * WHOLE_TEXTURE_WIDTH).toBeGreaterThanOrEqual(boxes.length);
  expect([...words.subarray(params.wholeFlagsBase)].every((w) => w === 0)).toBe(true);
  expect(params.wholeCount).toBe(boxes.length);
});
