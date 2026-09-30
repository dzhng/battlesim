// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test, vi } from "vitest";
import { initSync, Battle } from "@wasm/game_wasm.js";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry";
import { cellPatchRuns, groundRunCells } from "./groundRuns";
import { GroundView } from "../src/battle/sim/ground";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
import {
  SCAR_UNIFORM,
  SCAR_DENSE,
  SCAR_KEY_MASK,
  createScarTexture,
  ScarSync,
  scarHash,
  type ScarTarget,
  type ScarRegion,
} from "@packages/battle-renderer/src/frame/scarTexture";
import { labScenario, type LabEvent } from "@apps/battle-lab/src/scenarios";
import groundMap from "@fixtures/ground-lab.json";
let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

/** The actual upload boundary, mirrored in CPU memory instead of GPU queue IO. */
class MirrorTarget implements ScarTarget {
  pixels = new Uint8Array(64);
  words = new Uint32Array(2);
  written: number[] = [];
  defaultWord = 0;
  bounds: ScarRegion = [0, 0, -1, -1];
  resize(width: number, height: number) {
    if (width > 8192 || height > 8192) throw new Error("texture dimension exceeds device limit");
    this.pixels = new Uint8Array(width * height * 4);
  }
  directory(words: Uint32Array, defaultWord = 0, bounds: ScarRegion = [0, 0, -1, -1]) {
    this.defaultWord = defaultWord;
    this.bounds = bounds;
    this.words = words.slice();
  }
  write(slot: number, marks: Uint8Array, _side: number) {
    this.pixels.set(marks, slot * 4);
    this.written.push(slot);
  }
  sample(view: GroundView, x: number, y: number): number[] {
    x = Math.max(0.5, Math.min(view.cols - 0.5, x));
    y = Math.max(0.5, Math.min(view.rows - 0.5, y));
    const cell = (i: number, j: number): number[] => {
      i = Math.min(view.cols - 1, Math.max(0, i));
      j = Math.min(view.rows - 1, Math.max(0, j));
      const tx = Math.floor(i / 16),
        ty = Math.floor(j / 16),
        key = ty * Math.ceil(view.cols / 16) + tx,
        mask = this.words.length / 2 - 1;
      let at = scarHash(key, mask);
      while (this.words[at * 2] && (this.words[at * 2] & SCAR_KEY_MASK) !== key + 1)
        at = (at + 1) & mask;
      const encoded = this.words[at * 2],
        value = this.words[at * 2 + 1];
      if (!encoded) {
        const word =
          tx >= this.bounds[0] &&
          ty >= this.bounds[1] &&
          tx <= this.bounds[2] &&
          ty <= this.bounds[3]
            ? this.defaultWord
            : 0;
        return [word & 255, (word >>> 8) & 255, (word >>> 16) & 255, word >>> 24];
      }
      if (encoded & SCAR_UNIFORM)
        return [value & 255, (value >>> 8) & 255, (value >>> 16) & 255, value >>> 24];
      const local = (j % 16) * 16 + (i % 16);
      if (encoded & SCAR_DENSE)
        return [...this.pixels.subarray((value + local) * 4, (value + local + 1) * 4)];
      const offset = value >>> 8,
        count = value & 255;
      let lo = 0,
        hi = count;
      while (lo < hi) {
        const mid = (lo + hi) >>> 1,
          at = (offset + mid * 2) * 4,
          end = this.pixels[at] | (this.pixels[at + 1] << 8);
        if (end <= local) lo = mid + 1;
        else hi = mid;
      }
      return [...this.pixels.subarray((offset + lo * 2 + 1) * 4, (offset + lo * 2 + 2) * 4)];
    };
    const px = x - 0.5,
      py = y - 0.5,
      i = Math.floor(px),
      j = Math.floor(py),
      fx = px - i,
      fy = py - j;
    const a = cell(i, j),
      b = cell(i + 1, j),
      c = cell(i, j + 1),
      d = cell(i + 1, j + 1);
    return [0, 1, 2, 3].map(
      (k) => (a[k] + (b[k] - a[k]) * fx) * (1 - fy) + (c[k] + (d[k] - c[k]) * fx) * fy,
    );
  }
}
function matchUploadedJoins(target: MirrorTarget, view: GroundView) {
  for (let at = 0; at < target.words.length; at += 2) {
    const encoded = target.words[at];
    if (!encoded || encoded & SCAR_UNIFORM) continue;
    const key = (encoded & SCAR_KEY_MASK) - 1,
      x = (key % Math.ceil(view.cols / 16)) * 16,
      y = Math.floor(key / Math.ceil(view.cols / 16)) * 16;
    for (let j = -2; j < 18; j++)
      for (let i = -2; i < 18; i++)
        expect(target.sample(view, x + i + 0.5, y + j + 0.5)).toEqual(
          expected(view, x + i + 0.5, y + j + 0.5),
        );
  }
}

function expected(view: GroundView, x: number, y: number): number[] {
  const px = Math.max(0, Math.min(view.cols - 1, x - 0.5)),
    py = Math.max(0, Math.min(view.rows - 1, y - 0.5));
  const i = Math.floor(px),
    j = Math.floor(py),
    fx = px - i,
    fy = py - j;
  const a = view.cell(i, j),
    b = view.cell(Math.min(view.cols - 1, i + 1), j),
    c = view.cell(i, Math.min(view.rows - 1, j + 1)),
    d = view.cell(Math.min(view.cols - 1, i + 1), Math.min(view.rows - 1, j + 1));
  return (["crater", "scorch", "tracks", "trampled"] as const).map(
    (k) => (a[k] + (b[k] - a[k]) * fx) * (1 - fy) + (c[k] + (d[k] - c[k]) * fx) * fy,
  );
}
function match(target: MirrorTarget, view: GroundView) {
  view.forEachMarked((i, j) =>
    expect(target.sample(view, i + 0.5, j + 0.5)).toEqual(expected(view, i + 0.5, j + 0.5)),
  );
}
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

test("full-extent edge scars fit device limits and preserve bilinear taps across page joins", () => {
  const view = new GroundView({ cellM: 1, cols: 18000, rows: 18000 });
  const coords = [
    [0, 0],
    [15, 15],
    [16, 16],
    [17, 15],
    [17999, 17999],
  ];
  view.applyRuns(
    cellPatchRuns(view.cols, {
      epoch: 1,
      side: "blue",
      baseRevision: 0,
      revision: 1,
      full: true,
      cells: Uint32Array.from(coords.map(([i, j]) => j * view.cols + i)),
      marks: Uint8Array.from(
        coords.flatMap((_, k) => [37 + k * 31, 11 + k * 17, 23 + k * 19, 3 + k * 23]),
      ),
      cleared: new Uint8Array(coords.length),
    }),
  );
  const target = new MirrorTarget(),
    scars = new ScarSync(target);
  scars.sync(view);
  matchUploadedJoins(target, view);
  expect(scars.stats().textureBytes + scars.stats().directoryBytes).toBeLessThan(256_000);
  for (const [x, y] of [
    [0, 0],
    [15.2, 15.7],
    [16.1, 15.9],
    [16.5, 16.5],
    [17.2, 15.1],
    [18000, 18000],
    [9000, 9000],
  ])
    for (const [dx, dy] of [
      [0, 0],
      [-1.5, -1.5],
      [1.5, 1.5],
      [-0.75, 0],
      [0, 0.75],
    ])
      target
        .sample(view, x + dx, y + dy)
        .forEach((value, c) => expect(value).toBeCloseTo(expected(view, x + dx, y + dy)[c], 10));
});

test("the cache follows actual learned ground through deltas and keeps all known pages", () => {
  const battle = new Battle(SCENARIO, 5),
    layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const view = new GroundView(layout.ground),
    target = new MirrorTarget(),
    scars = new ScarSync(target);
  let deltas = 0;
  for (let t = 0; t < 150; t++) {
    battle.step();
    const p = record(battle, layout, "blue");
    view.applyRuns(p);
    scars.sync(view);
    match(target, view);
    if (!p.full && groundRunCells(p) > 0) deltas++;
  }
  expect(deltas).toBeGreaterThan(3);
  expect(view.at(230.5, 118.5)!.crater).toBeGreaterThan(0);
  target.written = [];
  expect(scars.sync(view)).toBe(false);
  expect(target.written).toEqual([]);
  battle.free();
}, 30000);

test("reset and side snapshots remove prior scars, and rebuilt frames restore every known page", () => {
  const battle = new Battle(SCENARIO, 5),
    layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const view = new GroundView(layout.ground),
    target = new MirrorTarget(),
    scars = new ScarSync(target);
  for (let t = 0; t < 60; t++) {
    battle.step();
    view.applyRuns(record(battle, layout, "blue"));
    scars.sync(view);
  }
  expect(target.sample(view, 200.5, 110.5)[0]).toBeGreaterThan(0);
  view.invalidate();
  expect(scars.sync(view)).toBe(true);
  expect(target.sample(view, 200.5, 110.5)).toEqual([0, 0, 0, 0]);
  battle.resync_observation();
  view.applyRuns(record(battle, layout, "red"));
  scars.sync(view);
  match(target, view);
  expect(target.sample(view, 200.5, 110.5)[0]).toBe(0);
  expect(target.sample(view, 520.5, 300.5)[0]).toBeGreaterThan(0);
  const rebuilt = new MirrorTarget();
  new ScarSync(rebuilt).sync(view);
  match(rebuilt, view);
  battle.free();
}, 30000);

test("camera-region cache stays bounded while switching between distant known marks", () => {
  const view = new GroundView({ cellM: 1, cols: 18000, rows: 18000 });
  const cells = Array.from(
    { length: 8000 },
    (_, k) => Math.floor(k / 100) * 32 * view.cols + (k % 100) * 32,
  );
  view.applyRuns(
    cellPatchRuns(view.cols, {
      epoch: 1,
      side: "blue",
      baseRevision: 0,
      revision: 1,
      full: true,
      cells: Uint32Array.from(cells),
      marks: Uint8Array.from(cells.flatMap((_, k) => [17 + (k % 239), 29, 31, 43])),
      cleared: new Uint8Array(cells.length),
    }),
  );
  const target = new MirrorTarget(),
    scars = new ScarSync(target);
  scars.sync(view, [0, 0, 1024, 1024]);
  expect(scars.stats().textureBytes + scars.stats().directoryBytes).toBeLessThan(8 * 1024 * 1024);
  expect(target.sample(view, 0.5, 0.5)).toEqual(expected(view, 0.5, 0.5));
  scars.sync(view, [2048, 2048, 3072, 3072]);
  expect(scars.stats().textureBytes + scars.stats().directoryBytes).toBeLessThan(8 * 1024 * 1024);
  expect(target.sample(view, 2048.5, 2048.5)).toEqual(expected(view, 2048.5, 2048.5));
  expect(view.at(0.5, 0.5)!.crater).toBe(17);
  scars.sync(view, [0, 0, 1024, 1024]);
  expect(target.sample(view, 0.5, 0.5)).toEqual(expected(view, 0.5, 0.5));
}, 30000);

test("production cache retains publication deltas and owns one bounded GPU pool", () => {
  vi.stubGlobal("GPUTextureUsage", { TEXTURE_BINDING: 4, COPY_DST: 2 });
  const writes: number[] = [];
  const resource = () => ({ destroy() {} });
  const device = {
    createBuffer: resource,
    createTexture: resource,
    limits: { maxTextureDimension2D: 8192 },
    queue: {
      writeTexture(_target: unknown, data: Uint8Array | Uint32Array) {
        writes.push(data.byteLength);
      },
    },
  } as unknown as GPUDevice;
  const registry = new GpuRegistry(device);
  try {
    const cache = createScarTexture(registry),
      view = new GroundView({ cellM: 1, cols: 600, rows: 440 });
    view.applyRuns(
      cellPatchRuns(view.cols, {
        epoch: 1,
        side: "blue",
        baseRevision: 0,
        revision: 1,
        full: true,
        cells: Uint32Array.of(80 * 600 + 32, 80 * 600 + 128, 80 * 600 + 400),
        marks: Uint8Array.of(19, 0, 0, 0, 19, 0, 0, 0, 19, 0, 0, 0),
        cleared: new Uint8Array(3),
      }),
    );
    cache.setGround(view);
    cache.prepare([0, 0, 600, 440]);
    const full = cache.stats();
    expect(registry.stats().textureBytes).toBeLessThan(9 * 1024 * 1024);
    const allocations = registry.stats();
    writes.length = 0;
    view.applyRuns(
      cellPatchRuns(view.cols, {
        epoch: 1,
        side: "blue",
        baseRevision: 1,
        revision: 2,
        full: false,
        cells: Uint32Array.of(80 * 600 + 33),
        marks: Uint8Array.of(73, 0, 0, 0),
        cleared: new Uint8Array(1),
      }),
    );
    cache.setGround(view);
    cache.prepare([0, 0, 600, 440]);
    expect(cache.stats().fullUploads).toBe(full.fullUploads);
    expect(cache.stats().lastBytes).toBeLessThan(full.lastBytes);
    expect(writes.length).toBeGreaterThan(0);
    expect(registry.stats()).toEqual(allocations);
    writes.length = 0;
    cache.setGround(view);
    cache.prepare([0, 0, 600, 440]);
    expect(writes).toEqual([]);
  } finally {
    registry.release();
    vi.unstubAllGlobals();
  }
  expect(registry.stats().textureBytes).toBe(0);
});

test("unadmitted Q8 grid axes retain the prior source and pending learned publication", () => {
  vi.stubGlobal("GPUTextureUsage", { TEXTURE_BINDING: 4, COPY_DST: 2 });
  const resource = () => ({ destroy() {} });
  const device = {
    createBuffer: resource,
    createTexture: resource,
    limits: { maxTextureDimension2D: 8192 },
    queue: { writeTexture() {} },
  } as unknown as GPUDevice;
  const registry = new GpuRegistry(device);
  try {
    const cache = createScarTexture(registry);
    const prior = new GroundView({ cellM: 1, cols: 16, rows: 16 });
    cache.setGround(prior);
    cache.prepare();
    const before = cache.stats();
    for (const [cols, rows] of [
      [65537, 1],
      [1, 65537],
    ]) {
      const view = new GroundView({ cellM: 1, cols, rows });
      view.applyRuns({
        epoch: 1,
        side: "blue",
        baseRevision: 0,
        revision: 1,
        full: true,
        runs: Float32Array.of(0, 256, 91, 0),
      });
      expect(() => cache.setGround(view)).toThrow(/Q8/);
      expect(cache.stats()).toEqual(before);
      expect(view.takeChanges()).toEqual({ all: true });
      expect(view.cell(0, 0).crater).toBe(91);
    }
    const edge = new GroundView({ cellM: 1, cols: 65536, rows: 1 });
    cache.setGround(edge);
    cache.prepare();
    expect(cache.stats().cols).toBe(65536);
  } finally {
    registry.release();
    vi.unstubAllGlobals();
  }
});

test("uniform learned tiles stay in directory words and bilinear joins survive dense page promotion", () => {
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
      19 + 29 * 256,
      31 + 43 * 256,
      1,
      65536,
      73 + 91 * 256,
      101 + 113 * 256,
    ),
  });
  const target = new MirrorTarget(),
    scars = new ScarSync(target);
  scars.sync(view);
  expect(target.written).toEqual([]);
  for (const x of [15.25, 15.5, 15.75, 16, 16.25, 16.5])
    target
      .sample(view, x, 8.25)
      .forEach((v, c) => expect(v).toBeCloseTo(expected(view, x, 8.25)[c], 10));
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 1,
    revision: 2,
    full: false,
    runs: Float32Array.of(0, 15 + 8 * 16 + 256, 181, 0),
  });
  scars.sync(view);
  expect(target.written.length).toBeGreaterThan(0);
  matchUploadedJoins(target, view);
  for (const x of [15.25, 15.75, 16.25])
    target
      .sample(view, x, 8.5)
      .forEach((v, c) => expect(v).toBeCloseTo(expected(view, x, 8.5)[c], 10));
  target.written = [];
  expect(scars.sync(view)).toBe(false);
  expect(target.written).toEqual([]);
});

test("resident capacity is judged after uniform promotion and demotion in one publication", () => {
  const view = new GroundView({ cellM: 1, cols: 32, rows: 16 });
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 0,
    revision: 1,
    full: true,
    runs: Float32Array.of(0, 65536, 19, 0, 1, 256, 20, 0, 1, 1 + 255 * 256, 19, 0),
  });
  const target = new MirrorTarget(),
    scars = new ScarSync(target, 512);
  scars.sync(view);
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 1,
    revision: 2,
    full: false,
    runs: Float32Array.of(0, 256, 20, 0, 1, 256, 19, 0),
  });
  expect(() => scars.sync(view)).not.toThrow();
  matchUploadedJoins(target, view);
  for (const x of [0.5, 15.75, 16.25, 31.5])
    expect(target.sample(view, x, 0.5)).toEqual(expected(view, x, 0.5));
});

test("a common word cache retains exact zero holes, local exceptions and partial-edge filter joins", () => {
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
  const target = new MirrorTarget(),
    scars = new ScarSync(target);
  scars.sync(view);
  expect(target.defaultWord).toBe(19);
  expect(scars.stats().directoryEntries).toBeLessThan(8);
  expect(target.written).toEqual([]);
  for (const p of [
    [0.5, 0.5],
    [31.75, 16.25],
    [32.5, 16.5],
    [33, 17],
  ])
    expect(target.sample(view, ...(p as [number, number]))).toEqual(
      expected(view, ...(p as [number, number])),
    );
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 1,
    revision: 2,
    full: false,
    runs: Float32Array.of(0, 256, 20, 0, 5, 256, 19, 0),
  });
  scars.sync(view);
  for (const p of [
    [0.5, 0.5],
    [1, 0.5],
    [31.75, 16.25],
    [32.5, 16.5],
  ])
    expect(target.sample(view, ...(p as [number, number]))).toEqual(
      expected(view, ...(p as [number, number])),
    );
  scars.sync(view, [0, 0, 16, 16]);
  expect(target.sample(view, 32.5, 16.5)).toEqual([0, 0, 0, 0]);
  scars.sync(view);
  expect(target.sample(view, 32.5, 16.5)).toEqual(expected(view, 32.5, 16.5));
});

test("thousands of locally varying pages fit one exact compressed cache without halo materialization", () => {
  const view = new GroundView({ cellM: 1, cols: 1600, rows: 800 }),
    cells = Array.from(
      { length: 5000 },
      (_, key) => (Math.floor(key / 100) * 16 + 8) * 1600 + (key % 100) * 16 + 8,
    );
  view.applyRuns(
    cellPatchRuns(view.cols, {
      epoch: 1,
      side: "blue",
      baseRevision: 0,
      revision: 1,
      full: true,
      cells: Uint32Array.from(cells),
      marks: Uint8Array.from(cells.flatMap((_, k) => [19 + (k % 200), 31, 43, 57])),
      cleared: new Uint8Array(cells.length),
    }),
  );
  const target = new MirrorTarget(),
    scars = new ScarSync(target);
  expect(() => scars.sync(view)).not.toThrow();
  expect(scars.stats().pages).toBe(5000);
  expect(scars.stats().lastTextureBytes).toBeLessThan(5000 * 64);
  for (const key of [0, 31, 99, 100, 2047, 4096, 4999]) {
    const x = (key % 100) * 16 + 8.25,
      y = Math.floor(key / 100) * 16 + 8.75;
    expect(target.sample(view, x, y)).toEqual(expected(view, x, y));
  }
  target.written = [];
  expect(scars.sync(view)).toBe(false);
  expect(target.written).toEqual([]);
});

test("fitting churn preserves learned pages and the immutable normalization words", () => {
  const view = new GroundView({ cellM: 1, cols: 832, rows: 16 });
  const initial: number[] = [];
  for (let tile = 0; tile < 52; tile++)
    if (tile < 32)
      initial.push(tile, 136 * 256, 255, 0, tile, 136 + 256, 19, 0, tile, 137 + 119 * 256, 255, 0);
    else initial.push(tile, 65536, 255, 0);
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 0,
    revision: 1,
    full: true,
    runs: Float32Array.from(initial),
  });
  const target = new MirrorTarget();
  target.resize(32, 32);
  const normalization = Uint8Array.from({ length: 512 * 4 }, (_, i) => i % 251);
  target.pixels.set(normalization);
  const scars = new ScarSync(target, 1024, 512, true);
  scars.sync(view);
  const runs: number[] = [];
  for (let c = 0; c < 256; c++) runs.push(0, c + 256, 50 + (c % 200), 0);
  for (let tile = 1; tile < 30; tile += 2) runs.push(tile, 136 + 256, 255, 0);
  view.applyRuns({
    epoch: 1,
    side: "blue",
    baseRevision: 1,
    revision: 2,
    full: false,
    runs: Float32Array.from(runs),
  });
  expect(() => scars.sync(view)).not.toThrow();
  expect(target.pixels.subarray(0, normalization.length)).toEqual(normalization);
  for (const x of [0.5, 8.25, 15.75, 16.25, 32.5, 488.5, 504.5])
    expect(target.sample(view, x, 8.5)).toEqual(expected(view, x, 8.5));
});
