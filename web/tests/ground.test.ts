// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { GroundView } from "../src/battle/sim/ground";
import {
  ObservationDecoder,
  type GroundLayout,
  type GroundPatchView,
  type ObservationLayout,
} from "../src/battle/sim/observation";
import { labScenario, type LabEvent } from "@apps/battle-lab/src/scenarios";
import groundMap from "@fixtures/ground-lab.json";

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
  count: "groundCellCount",
  fields: ["cellLo", "cellHi", "craterScorch", "tracksTrampledCleared"],
  cellM: 1,
  cols: 4,
  rows: 4,
  sides: ["blue", "red"],
};

function patch(
  epoch: number,
  base: number,
  revision: number,
  cells: [number, number[]][],
  full = false,
): GroundPatchView {
  return {
    epoch,
    side: "blue",
    baseRevision: base,
    revision,
    full,
    cells: Uint32Array.from(cells.map(([c]) => c)),
    marks: Uint8Array.from(cells.flatMap(([, m]) => m)),
    cleared: new Uint8Array(cells.length),
  };
}

test("untouched full-extent ground costs no cell arrays and edge edits stay sparse", () => {
  const view = new GroundView({ ...GRID, cols: 18_000, rows: 18_000 });
  expect(view.byteLength).toBeLessThan(4096);
  const edge = 18_000 * 18_000 - 1;
  view.apply(
    patch(
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
  expect(view.apply(patch(1, 0, 2, [[5, [10, 0, 0, 0]]], true))).toBe("applied");
  expect(view.apply(patch(1, 2, 3, [[6, [0, 20, 30, 40]]]))).toBe("applied");
  expect(view.cell(1, 1)).toEqual({ crater: 10, scorch: 0, tracks: 0, trampled: 0 });
  expect(view.at(2.5, 1.2)).toEqual({ crater: 0, scorch: 20, tracks: 30, trampled: 40 });
  expect(view.revision).toBe(3);
  // A delta that skips a revision means a lost patch: never silently applied.
  expect(() => view.apply(patch(1, 4, 5, []))).toThrow();
  // A new epoch must open with a snapshot, which replaces everything.
  expect(() => view.apply(patch(2, 3, 4, []))).toThrow();
  expect(view.apply(patch(2, 0, 7, [[0, [1, 1, 1, 1]]], true))).toBe("applied");
  const marked: number[] = [];
  view.forEachMarked((_, __, k) => marked.push(k));
  expect(marked).toEqual([0]);
  // A patch from an older epoch arriving late is dropped.
  expect(view.apply(patch(1, 3, 4, [[9, [9, 9, 9, 9]]]))).toBe("stale");
  expect(view.cell(1, 2).crater).toBe(0);
});

test("after invalidate, the old epoch's in-flight patches are stale until a new snapshot", () => {
  const view = new GroundView(GRID);
  view.apply(patch(1, 0, 1, [[3, [5, 0, 0, 0]]], true));
  view.invalidate();
  expect(view.cell(3, 0).crater).toBe(0);
  expect(view.apply(patch(1, 1, 2, [[4, [6, 0, 0, 0]]]))).toBe("stale");
  expect(view.cell(0, 1).crater).toBe(0);
  expect(view.apply(patch(2, 0, 2, [[4, [6, 0, 0, 0]]], true))).toBe("applied");
  expect(view.cell(0, 1).crater).toBe(6);
});

test("changes are reported exactly, and as everything after a snapshot or past a bound", () => {
  const view = new GroundView({ ...GRID, cols: 64, rows: 64 });
  view.apply(patch(1, 0, 1, [[3, [5, 0, 0, 0]]], true));
  expect(view.takeChanges()).toEqual({ all: true });
  view.apply(patch(1, 1, 2, [[7, [1, 0, 0, 0]]]));
  view.apply(patch(1, 2, 3, [[2, [1, 0, 0, 0]]]));
  expect(view.takeChanges()).toEqual({ all: false, cells: Uint32Array.from([7, 2]) });
  expect(view.takeChanges()).toEqual({ all: false, cells: new Uint32Array(0) });
  // Nobody took them: past a sixteenth of the map it is "everything" again.
  const many = Array.from({ length: 300 }, (_, k): [number, number[]] => [k, [2, 0, 0, 0]]);
  view.apply(patch(1, 3, 4, many));
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
      { side: "blue", kind: "tank", position: [110, 110] },
      { side: "red", kind: "tank", position: [540, 330], yaw: Math.PI },
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
      if (!p.full) deltaCells += p.cells.length;
      view.apply(p);
    }
    expect(deltaCells).toBeGreaterThan(50);
    battle.resync_observation();
    const snapshot = record(battle, layout, "blue");
    expect(snapshot.full && snapshot.epoch === view.epoch + 1).toBe(true);
    const fresh = new GroundView(layout.ground);
    fresh.apply(snapshot);
    const cells = (v: GroundView) => {
      const out: unknown[] = [];
      v.forEachMarked((i, j, k) => out.push([k, v.cell(i, j)]));
      return out;
    };
    expect(cells(fresh)).toEqual(cells(view));
    expect(view.at(200.5, 110.5)!.crater).toBeGreaterThan(0);

    // Red's ground is its own: blue's crater and tracks are not in it.
    const red = new GroundView(layout.ground);
    red.apply(record(battle, layout, "red"));
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
