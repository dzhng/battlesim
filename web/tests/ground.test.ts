// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, WorldView } from "@wasm/game_wasm.js";
import { cellPatchRuns, groundRunCells } from "./groundRuns";
import { GroundView, type GroundRunsPatch } from "../src/battle/sim/ground";
import {
  ObservationDecoder,
  type GroundLayout,
  type ObservationLayout,
} from "../src/battle/sim/observation";
import { TEST_RULES, labScenario } from "./catalog";
import { type LabEvent } from "@apps/battle-lab/src/scenarios";
import { loadMap } from "@web/maps/node";

const groundMap = loadMap("ground").definition;

// Whole battles run to a late state; under a loaded `bun run check` they
// can pass Vitest's 5 s default without anything being wrong.
const BATTLE_TEST_TIMEOUT_MS = 30_000;

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

const GRID: GroundLayout = {
  count: "groundRunCount",
  fields: ["tile", "span", "craterScorch", "tracksTrampledCleared"],
  tileSize: 16,
  maxRecordBytes: 64 * 1024 * 1024,
  cellM: 1,
  cols: 4,
  rows: 4,
  sides: ["blue", "red"],
};

function patch(
  view: GroundView,
  epoch: number,
  base: number,
  revision: number,
  cells: [number, number[]][],
  full = false,
): GroundRunsPatch {
  return cellPatchRuns(view.cols, {
    epoch,
    side: "blue",
    baseRevision: base,
    revision,
    full,
    cells: Uint32Array.from(cells.map(([c]) => c)),
    marks: Uint8Array.from(cells.flatMap(([, m]) => m)),
    cleared: new Uint8Array(cells.length),
  });
}

test("untouched full-extent ground costs no cell arrays and edge edits stay sparse", () => {
  const view = new GroundView({ ...GRID, cols: 18_000, rows: 18_000 });
  expect(view.byteLength).toBeLessThan(4096);
  const edge = 18_000 * 18_000 - 1;
  view.applyRuns(
    patch(
      view,
      1,
      0,
      1,
      [
        [edge, [17, 29, 31, 43]],
        [0, [3, 5, 7, 11]],
      ],
      true,
    ),
  );
  expect(view.at(17_999.5, 17_999.5)).toEqual({ crater: 17, scorch: 29, tracks: 31, trampled: 43 });
  expect(view.at(9000, 9000)).toEqual({ crater: 0, scorch: 0, tracks: 0, trampled: 0 });
  expect(view.byteLength).toBeLessThan(4096);
  const marked: number[] = [];
  view.forEachMarked((_, __, k) => marked.push(k));
  expect(marked).toEqual([0, edge]);
});

test("a view follows its epoch: snapshots replace, deltas continue, stale patches drop", () => {
  const view = new GroundView(GRID);
  expect(view.applyRuns(patch(view, 1, 0, 2, [[5, [10, 0, 0, 0]]], true))).toBe("applied");
  expect(view.applyRuns(patch(view, 1, 2, 3, [[6, [0, 20, 30, 40]]]))).toBe("applied");
  expect(view.cell(1, 1)).toEqual({ crater: 10, scorch: 0, tracks: 0, trampled: 0 });
  expect(view.at(2.5, 1.2)).toEqual({ crater: 0, scorch: 20, tracks: 30, trampled: 40 });
  expect(view.revision).toBe(3);
  // A delta that skips a revision means a lost patch: never silently applied.
  expect(() => view.applyRuns(patch(view, 1, 4, 5, []))).toThrow();
  // A new epoch must open with a snapshot, which replaces everything.
  expect(() => view.applyRuns(patch(view, 2, 3, 4, []))).toThrow();
  expect(view.applyRuns(patch(view, 2, 0, 7, [[0, [1, 1, 1, 1]]], true))).toBe("applied");
  const marked: number[] = [];
  view.forEachMarked((_, __, k) => marked.push(k));
  expect(marked).toEqual([0]);
  // A patch from an older epoch arriving late is dropped.
  expect(view.applyRuns(patch(view, 1, 3, 4, [[9, [9, 9, 9, 9]]]))).toBe("stale");
  expect(view.cell(1, 2).crater).toBe(0);
});

test("after invalidate, the old epoch's in-flight patches are stale until a new snapshot", () => {
  const view = new GroundView(GRID);
  view.applyRuns(patch(view, 1, 0, 1, [[3, [5, 0, 0, 0]]], true));
  view.invalidate();
  expect(view.cell(3, 0).crater).toBe(0);
  expect(view.applyRuns(patch(view, 1, 1, 2, [[4, [6, 0, 0, 0]]]))).toBe("stale");
  expect(view.cell(0, 1).crater).toBe(0);
  expect(view.applyRuns(patch(view, 2, 0, 2, [[4, [6, 0, 0, 0]]], true))).toBe("applied");
  expect(view.cell(0, 1).crater).toBe(6);
});

test("changes are reported exactly, and as everything after a snapshot or past a bound", () => {
  const view = new GroundView({ ...GRID, cols: 64, rows: 64 });
  view.applyRuns(patch(view, 1, 0, 1, [[3, [5, 0, 0, 0]]], true));
  expect(view.takeChanges()).toEqual({ all: true });
  view.applyRuns(patch(view, 1, 1, 2, [[7, [1, 0, 0, 0]]]));
  view.applyRuns(patch(view, 1, 2, 3, [[2, [1, 0, 0, 0]]]));
  expect(view.takeChanges()).toEqual({ all: false, cells: Uint32Array.from([7, 2]) });
  expect(view.takeChanges()).toEqual({ all: false, cells: new Uint32Array(0) });
  // Nobody took them: past a sixteenth of the map it is "everything" again.
  const many = Array.from({ length: 300 }, (_, k): [number, number[]] => [k, [2, 0, 0, 0]]);
  view.applyRuns(patch(view, 1, 3, 4, many));
  expect(view.takeChanges()).toEqual({ all: true });
});

/** Two tanks lay tracks across the lab field; bursts dig on both sides. */
const SCENARIO = (() => {
  const bursts: LabEvent[] = [
    { tick: 2, burst: { point: [200, 110], weapon: "tank_he" } },
    { tick: 2, burst: { point: [520, 300], weapon: "tank_he" } },
  ];
  return labScenario(
    groundMap,
    [
      { side: "blue", kind: "test_tank", position: [110, 110] },
      { side: "red", kind: "test_tank", position: [540, 330], yaw: Math.PI },
    ],
    bursts,
    [
      {
        tick: 1,
        side: "blue",
        order: { kind: "move", units: [0], gesture: 1, goal: [250, 110], route: "shortest" },
      },
      {
        tick: 1,
        side: "red",
        order: { kind: "move", units: [1], gesture: 2, goal: [540, 420], route: "shortest" },
      },
    ],
  );
})();

const decoders = new WeakMap<Battle, ObservationDecoder>();
function record(battle: Battle, layout: ObservationLayout, side: "blue" | "red") {
  const length = battle.publish(side);
  let decoder = decoders.get(battle);
  if (!decoder) decoders.set(battle, (decoder = new ObservationDecoder(layout)));
  return decoder.decode(new Float32Array(memory.buffer, battle.publication_ptr(), length).slice())!
    .groundPatch;
}

test(
  "a battle's patches rebuild the side's ground: deltas equal a fresh snapshot",
  () => {
    const battle = new Battle(SCENARIO, 5);
    const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
    expect([layout.ground.cols, layout.ground.rows]).toEqual([600, 440]);
    const view = new GroundView(layout.ground);
    let deltaCells = 0;
    for (let t = 0; t < 150; t++) {
      battle.step();
      // Some ticks go unpublished (a stalled consumer): the next delta covers them.
      if (t % 7 === 3) continue;
      const p = record(battle, layout, "blue");
      if (!p.full) deltaCells += groundRunCells(p);
      view.applyRuns(p);
    }
    expect(deltaCells).toBeGreaterThan(50);
    battle.resync_observation();
    const snapshot = record(battle, layout, "blue");
    expect(snapshot.full && snapshot.epoch === view.epoch + 1).toBe(true);
    const fresh = new GroundView(layout.ground);
    fresh.applyRuns(snapshot);
    const cells = (v: GroundView) => {
      const out: unknown[] = [];
      v.forEachMarked((i, j, k) => out.push([k, v.cell(i, j)]));
      return out;
    };
    expect(cells(fresh)).toEqual(cells(view));
    expect(view.at(200.5, 110.5)!.crater).toBeGreaterThan(0);

    // Red's ground is its own: blue's crater and tracks are not in it.
    const red = new GroundView(layout.ground);
    red.applyRuns(record(battle, layout, "red"));
    expect(red.side).toBe("red");
    expect(red.at(200.5, 110.5)!.crater).toBe(0);
    expect(red.at(520.5, 300.5)!.crater).toBeGreaterThan(0);
    expect(view.at(520.5, 300.5)!.crater).toBe(0);
    battle.free();
  },
  BATTLE_TEST_TIMEOUT_MS,
);

test("run snapshots stay compressed and retain exact distant learned cells through deltas and epochs", () => {
  const view = new GroundView({ cellM: 1, cols: 18000, rows: 18000 });
  const tile = 1125 * 1125 - 1;
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 0,
    revision: 1,
    full: true,
    runs: Float32Array.of(
      0,
      65536,
      19 + 29 * 256,
      31 + 43 * 256 + 255 * 65536,
      tile,
      255 + 256,
      73,
      255 * 65536,
    ),
  });
  expect(view.byteLength).toBeLessThan(128);
  expect(view.hasMarks).toBe(true);
  expect(view.at(15.5, 15.5)).toEqual({ crater: 19, scorch: 29, tracks: 255, trampled: 43 });
  expect(view.at(17999.5, 17999.5)!.crater).toBe(73);
  expect(view.clearedCount).toBe(257);
  view.takeChanges();
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 1,
    revision: 2,
    full: false,
    runs: Float32Array.of(0, 17 + 256, 911 % 256, 0),
  });
  expect(view.cell(1, 1).crater).toBe(911 % 256);
  expect(view.cell(2, 1).crater).toBe(19);
  expect(view.clearedCount).toBe(256);
  expect([...view.clearedRuns()]).toEqual([0, 17 * 256, 0, 18 + 238 * 256, tile, 255 + 256]);
  view.applyRuns({
    epoch: 2,
    side: "red",
    baseRevision: 0,
    revision: 1,
    full: true,
    runs: Float32Array.of(tile, 255 + 256, 31, 0),
  });
  expect(view.at(0.5, 0.5)!.crater).toBe(0);
  expect(view.at(17999.5, 17999.5)!.crater).toBe(31);
  expect(view.clearedCount).toBe(0);
  view.invalidate();
  expect(view.byteLength).toBe(0);
});

test("clearing-only runs project full visible wear without changing native raw channels", () => {
  const view = new GroundView({ cellM: 1, cols: 16, rows: 16 });
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 0,
    revision: 1,
    full: true,
    runs: Float32Array.of(0, 65536, 0, 255 * 65536),
  });
  expect(view.cell(2, 3).tracks).toBe(255);
  const marks = new Uint8Array(4);
  view.readMarks(2, 3, marks);
  expect([...marks]).toEqual([0, 0, 255, 0]);
  expect(view.hasMarks).toBe(true);
  expect([...view.clearedRuns()]).toEqual([0, 65536]);
});

test("exact default words retain holes, edge tiles and varying exceptions across learning epochs", () => {
  const view = new GroundView({ cellM: 1, cols: 33, rows: 17 });
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 0,
    revision: 1,
    full: true,
    runs: Float32Array.from([
      0,
      65536,
      19,
      0,
      1,
      65536,
      19,
      0,
      ...Array.from({ length: 16 }, (_, row) => [2, row * 16 + 256, 19, 0]).flat(),
      3,
      16 * 256,
      19,
      0,
      4,
      16 * 256,
      19,
      0,
    ]),
  });
  expect(view.scarDefault()).toMatchObject({ word: 19, exceptions: 1 });
  const exceptions: number[] = [];
  view.forEachScarException(19, (x, y) =>
    exceptions.push((y / 16) * Math.ceil(view.cols / 16) + x / 16),
  );
  expect(exceptions).toEqual([5]);
  expect(view.uniformMarks(32, 16)).toBe(0);
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 1,
    revision: 2,
    full: false,
    runs: Float32Array.of(0, 256, 20, 0, 5, 256, 19, 0),
  });
  expect(view.scarDefault()).toMatchObject({ word: 19, exceptions: 1 });
  exceptions.length = 0;
  view.forEachScarException(19, (x, y) =>
    exceptions.push((y / 16) * Math.ceil(view.cols / 16) + x / 16),
  );
  expect(exceptions).toEqual([0]);
  view.invalidate();
  expect(view.scarDefault()).toMatchObject({ word: 0, exceptions: 0 });
});

test("scattered first-page arrivals retain exact holes without padded tail tiles", () => {
  const view = new GroundView({ cellM: 1, cols: 1648, rows: 16 }),
    tiles = 103;
  const touched = [0, 32, 31, tiles - 1, 17, 64, 63, 16, 1, tiles - 2];
  const all = new Set(touched);
  for (let k = 0; k < touched.length; k++)
    view.applyRuns({
      epoch: 1,
      side: "blue",
      baseRevision: k,
      revision: k + 1,
      full: k === 0,
      runs: Float32Array.of(touched[k], 65536, 19, 0),
    });
  // Default nonzero enumeration exercises occupancy metadata even though
  // production chooses zero while most of this grid is still missing.
  let count = 0,
    maximum = -1;
  view.forEachScarException(19, (x, y) => {
    const id = (y / 16) * 103 + x / 16;
    expect(all.has(id)).toBe(false);
    count++;
    maximum = Math.max(maximum, id);
  });
  expect(count).toBe(tiles - touched.length);
  expect(maximum).toBe(tiles - 3);
});

test("GPU run source carries exact projected values and reports the complete varying word budget", () => {
  const view = new GroundView({ cellM: 1, cols: 32, rows: 16 });
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 0,
    revision: 1,
    full: true,
    runs: Float32Array.of(
      0,
      65536,
      19,
      255 * 65536,
      1,
      256,
      37,
      255 * 65536,
      1,
      1 + 255 * 256,
      19,
      255 * 65536,
    ),
  });
  const scratch = new Uint32Array(512);
  const count = view.readScarRuns(16, 0, scratch);
  expect(count).toBe(2);
  expect([...scratch.subarray(0, count * 2)]).toEqual([1, 37 + 255 * 65536, 256, 19 + 255 * 65536]);
  expect(view.scarDefault().poolWords).toBe(16);
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 1,
    revision: 2,
    full: false,
    runs: Float32Array.of(1, 256, 19, 255 * 65536),
  });
  expect(view.scarDefault().poolWords).toBe(0);
});

test("foliage thins exactly where the side's cleared ground runs say", () => {
  const size = 160;
  const world = new WorldView(
    JSON.stringify({
      size: [size, size],
      fog_cell_m: 8,
      height_grid_m: 4,
      slope_cutoff_deg: 35,
      forests: [
        {
          shape: {
            kind: "polygon",
            ring: [
              [8, 8],
              [152, 8],
              [152, 152],
              [8, 152],
            ],
          },
        },
      ],
    }),
    JSON.stringify(TEST_RULES),
  );
  try {
    const cells = (f: Float32Array) =>
      new Map(
        Array.from({ length: (f.length - 3) / 4 }, (_, k) => [
          `${f[3 + k * 4]},${f[4 + k * 4]}`,
          [f[5 + k * 4], f[6 + k * 4]],
        ]),
      );
    const standing = cells(world.foliage());
    const cellM = world.foliage()[2];
    const ground = new GroundView({ cellM: 1, cols: size, rows: size });
    expect(cells(world.foliage_cleared(ground.clearedRuns(), size, 1))).toEqual(standing);

    // A cleared block that starts mid-tile and crosses 16-cell tile edges both ways.
    const [lo, hi] = [36, 92];
    const cleared: number[] = [];
    for (let j = lo; j < hi; j++) for (let i = lo; i < hi; i++) cleared.push(j * size + i);
    ground.applyRuns(
      cellPatchRuns(size, {
        epoch: 1,
        side: "blue",
        baseRevision: 0,
        revision: 1,
        full: true,
        cells: Uint32Array.from(cleared),
        marks: new Uint8Array(cleared.length * 4),
        cleared: new Uint8Array(cleared.length).fill(255),
      }),
    );
    const known = cells(world.foliage_cleared(ground.clearedRuns(), size, 1));
    // A fallen crown reaches no further than this past the block's edge.
    const reach = TEST_RULES.forests.rule.canopy_radius_m + 2 * cellM;
    let opened = 0;
    let untouched = 0;
    for (const [key, cell] of standing) {
      const [x, y] = key.split(",").map((index) => (Number(index) + 0.5) * cellM);
      const inside = (v: number) => v >= lo && v < hi;
      const beyond = (v: number) => v < lo - reach || v >= hi + reach;
      if (inside(x) && inside(y)) {
        expect(known.has(key), `cleared cell ${key}`).toBe(false);
        opened++;
      } else if (beyond(x) || beyond(y)) {
        expect(known.get(key), `distant cell ${key}`).toEqual(cell);
        untouched++;
      }
    }
    expect(opened).toBeGreaterThan(20);
    expect(untouched).toBeGreaterThan(20);
  } finally {
    world.free();
  }
});
