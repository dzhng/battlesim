// @vitest-environment node
// The side's learned ground through the page's presentation binding: a real
// battle's patches, a side switch, and the foliage it clears. How the ground
// learns and caches its scars is held in `crates/presentation/tests/scars.rs`.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle, WorldView } from "@wasm/game_wasm.js";
import { cellPatchRuns, groundRecords } from "./groundRuns";
import type { ObservationLayout } from "../src/battle/sim/observationView";
import { ScenePresentation, loadPresentation } from "../src/battle/sim/presentation";
import type { LearnedGround } from "../src/battle/sim/presentationGround";
import { TEST_RULES, labScenario } from "./catalog";
import { type LabEvent } from "@apps/battle-lab/src/scenarios";
import { loadMap } from "@web/maps/node";

await loadPresentation();

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

/** The crater a learned ground draws at (x, y), metres. */
function crater(ground: LearnedGround, x: number, y: number): number {
  const index = Math.floor(y / ground.cellM) * ground.cols + Math.floor(x / ground.cellM);
  const marked = ground.markedCells();
  for (let k = 0; k < marked.length; k += 2) if (marked[k] === index) return marked[k + 1] & 255;
  return 0;
}

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

function publication(battle: Battle, side: "blue" | "red"): Float32Array {
  const length = battle.publish(side);
  return new Float32Array(memory.buffer, battle.publication_ptr(), length).slice();
}

test(
  "a battle's patches rebuild the side's ground: deltas equal a fresh snapshot",
  () => {
    const battle = new Battle(SCENARIO, 5);
    const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
    expect([layout.ground.cols, layout.ground.rows]).toEqual([600, 440]);
    const decoder = new ScenePresentation(layout);
    let deltas = 0;
    for (let t = 0; t < 150; t++) {
      battle.step();
      // Some ticks go unpublished (a stalled consumer): the next delta covers them.
      if (t % 7 === 3) continue;
      const o = decoder.decode(publication(battle, "blue"))!;
      if (!o.groundPatch.full && decoder.groundPatchCells() > 0) deltas++;
    }
    expect(deltas).toBeGreaterThan(3);
    const view = decoder.ground!;
    battle.resync_observation();
    const fresh = new ScenePresentation(layout);
    const snapshot = fresh.decode(publication(battle, "blue"))!;
    expect(snapshot.groundPatch.full && snapshot.groundPatch.epoch === view.epoch + 1).toBe(true);
    expect([...fresh.ground!.markedCells()]).toEqual([...view.markedCells()]);
    expect(crater(view, 200.5, 110.5)).toBeGreaterThan(0);

    // Red's ground is its own: blue's crater and tracks are not in it.
    const red = new ScenePresentation(layout);
    red.decode(publication(battle, "red"));
    expect(red.ground!.side).toBe("red");
    expect(crater(red.ground!, 200.5, 110.5)).toBe(0);
    expect(crater(red.ground!, 520.5, 300.5)).toBeGreaterThan(0);
    expect(crater(view, 520.5, 300.5)).toBe(0);

    // Observing the other side forgets what was learned until its snapshot.
    decoder.invalidate("red");
    expect(view.markedCells().length).toBe(0);
    battle.resync_observation();
    decoder.decode(publication(battle, "red"));
    expect(view.side).toBe("red");
    expect([...view.markedCells()]).toEqual([...red.ground!.markedCells()]);
    battle.free();
  },
  BATTLE_TEST_TIMEOUT_MS,
);

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
  const battle = new Battle(labScenario(groundMap, []), 1);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  battle.free();
  layout.ground = { ...layout.ground, cols: size, rows: size, cellM: 1 };
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
    const decoder = new ScenePresentation(layout);
    const ground = decoder.ground!;
    expect(cells(world.foliage_cleared(ground.clearedRuns(), size, 1))).toEqual(standing);

    // A cleared block that starts mid-tile and crosses 16-cell tile edges both ways.
    const [lo, hi] = [36, 92];
    const cleared: number[] = [];
    for (let j = lo; j < hi; j++) for (let i = lo; i < hi; i++) cleared.push(j * size + i);
    decoder.decode(
      groundRecords(layout)(
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
      ),
    );
    expect(ground.clearedCount).toBe(cleared.length);
    expect(ground.isCleared(lo, lo) && !ground.isCleared(lo - 1, lo)).toBe(true);
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
