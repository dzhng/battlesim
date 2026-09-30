// @vitest-environment node
// The scar texture is the side's learned ground, kept
// in step by dirty-tile uploads. A real battle's patches drive a real
// `GroundView`; the texture is a CPU mirror of what the uploads wrote (the
// GPU queue is the only thing stood in for). How scars look is the scenes'.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { GroundView } from "../src/battle/sim/ground";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
import {
  SCAR_TILE,
  ScarSync,
  scarUploadRects,
  type ScarRect,
  type ScarTarget,
} from "@packages/battle-renderer/src/frame/scarTexture";
import { labScenario, type LabEvent } from "@apps/battle-lab/src/scenarios";
import groundMap from "@fixtures/ground-lab.json";

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

/** The texture as the uploads left it, and every texel each upload wrote. */
class MirrorTarget implements ScarTarget {
  texels = new Uint8Array(0);
  cols = 0;
  written: ScarRect[] = [];
  resize(cols: number, rows: number) {
    this.cols = cols;
    this.texels = new Uint8Array(cols * rows * 4);
  }
  write(marks: Uint8Array, cols: number, rect: ScarRect) {
    expect(cols).toBe(this.cols);
    for (let j = rect.y; j < rect.y + rect.h; j++) {
      const from = (j * cols + rect.x) * 4;
      this.texels.set(marks.subarray(from, from + rect.w * 4), from);
    }
    this.written.push(rect);
  }
}

/** Tanks lay tracks across the lab field; bursts dig on both sides, some
 *  after the first ticks so they arrive as deltas. */
const SCENARIO = (() => {
  const bursts: LabEvent[] = [
    { tick: 2, burst: { point: [200, 110], weapon: "tank_he" } },
    { tick: 40, burst: { point: [230, 118], weapon: "tank_he" } },
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

/** The first byte where two grids differ, or -1 (a deep equal of a 1 MB
 *  grid every tick is too slow to diff). */
function firstDifference(a: Uint8Array, b: Uint8Array) {
  if (a.length !== b.length) return Math.min(a.length, b.length);
  for (let k = 0; k < a.length; k++) if (a[k] !== b[k]) return k;
  return -1;
}

const tileOf = (i: number, j: number) =>
  `${Math.floor(i / SCAR_TILE)},${Math.floor(j / SCAR_TILE)}`;

test("the texture follows the side's ground through deltas, touching only changed tiles", () => {
  const battle = new Battle(SCENARIO, 5);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const view = new GroundView(layout.ground);
  const target = new MirrorTarget();
  const scars = new ScarSync(target);
  const whole = view.cols * view.rows * 4;
  let deltaUploads = 0;
  for (let t = 0; t < 150; t++) {
    battle.step();
    const patch = record(battle, layout, "blue");
    // What changed, as the patch says (the view's own list is the uploader's).
    const changed = new Set(
      [...patch.cells].map((c) => tileOf(c % view.cols, Math.floor(c / view.cols))),
    );
    view.apply(patch);
    target.written = [];
    scars.sync(view);
    expect(firstDifference(target.texels, view.marks)).toBe(-1);
    if (patch.full || scars.stats().fullUploads > 1) continue;
    // A delta writes exactly its tiles: every tile written holds a changed
    // cell, and every changed cell's tile is written.
    const written = new Set<string>();
    for (const r of target.written)
      for (let j = r.y; j < r.y + r.h; j += SCAR_TILE)
        for (let i = r.x; i < r.x + r.w; i += SCAR_TILE) written.add(tileOf(i, j));
    expect(written).toEqual(changed);
    if (patch.cells.length > 0) {
      deltaUploads++;
      expect(scars.stats().lastBytes).toBeLessThan(whole / 50);
    }
  }
  expect(deltaUploads).toBeGreaterThan(3);
  expect(view.at(230.5, 118.5)!.crater).toBeGreaterThan(0);
  // Nothing changed: nothing is written.
  target.written = [];
  expect(scars.sync(view)).toBe(false);
  expect(target.written).toEqual([]);
  battle.free();
});

test("a resync or side switch rewrites the whole texture: the other side's scars never linger", () => {
  const battle = new Battle(SCENARIO, 5);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const view = new GroundView(layout.ground);
  const target = new MirrorTarget();
  const scars = new ScarSync(target);
  for (let t = 0; t < 60; t++) {
    battle.step();
    view.apply(record(battle, layout, "blue"));
    scars.sync(view);
  }
  const blueCrater = (200 + 110 * view.cols) * 4;
  expect(target.texels[blueCrater]).toBeGreaterThan(0);

  // The lab's side switch: the view empties at once, then red's snapshot.
  view.invalidate();
  expect(scars.sync(view)).toBe(true);
  expect(target.texels.findIndex((b) => b !== 0)).toBe(-1);
  battle.resync_observation();
  view.apply(record(battle, layout, "red"));
  scars.sync(view);
  expect(firstDifference(target.texels, view.marks)).toBe(-1);
  expect(target.texels[blueCrater]).toBe(0);
  expect(target.texels[(520 + 300 * view.cols) * 4]).toBeGreaterThan(0);

  // A rebuilt frame (a fresh texture) given the same view draws it whole,
  // though the view has no changes left to report.
  const rebuilt = new MirrorTarget();
  new ScarSync(rebuilt).sync(view);
  expect(firstDifference(rebuilt.texels, view.marks)).toBe(-1);
  battle.free();
});

test("upload rects cover a tile row's dirty tiles in merged runs, clipped to the grid", () => {
  // A 40 × 20 grid: tiles 3 across (16, 16, 8) and 2 down (16, 4).
  const cols = 40;
  const at = (i: number, j: number) => j * cols + i;
  expect(scarUploadRects(Uint32Array.from([at(1, 1), at(20, 3), at(39, 19)]), cols, 20)).toEqual([
    { x: 0, y: 0, w: 32, h: 16 },
    { x: 32, y: 16, w: 8, h: 4 },
  ]);
  expect(scarUploadRects(Uint32Array.from([at(33, 0), at(0, 17), at(33, 1)]), cols, 20)).toEqual([
    { x: 32, y: 0, w: 8, h: 16 },
    { x: 0, y: 16, w: 16, h: 4 },
  ]);
});
