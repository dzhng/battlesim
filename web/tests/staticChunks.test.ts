// @vitest-environment node
// The static chunk owner, for any population: records of any stride kept in
// chunk order, a level chosen per chunk and drawn as merged ranges, and the
// near chunks left to their layer.
import { expect, test } from "vitest";
import { box3 } from "math/shapes";
import {
  NEAR,
  createStagedLevels,
  createStaticChunks,
  selectChunks,
  selectEveryChunk,
  stageNear,
  type PlacedRecords,
  type StaticChunks,
} from "@packages/battle-renderer/src/frame/staticChunks";
import {
  createDetailView,
  setDetailView,
  type DetailView,
} from "@packages/battle-renderer/src/frame/detailView";

const CHUNK_M = 64;
const SIZE_M = 2;

/** Instances `SIZE_M` across at `at`, `stride` floats each: its position,
 *  then its own index repeated, so a record is recognisable wherever it lands. */
function placed(at: [number, number][], stride: number, kinds?: number[]): PlacedRecords {
  const records = new Float32Array(at.length * stride);
  at.forEach(([x, y], i) => {
    records.fill(i, i * stride, (i + 1) * stride);
    records.set([x, y, 0], i * stride);
  });
  return {
    records,
    stride,
    kinds: Uint16Array.from(kinds ?? at.map(() => 0)),
    sizes: new Float32Array(at.length).fill(SIZE_M),
    bound(i, box) {
      const [x, y] = at[i];
      box3.expandByPoint(box, box, [x - 1, y - 1, 0]);
      box3.expandByPoint(box, box, [x + 1, y + 1, SIZE_M]);
    },
  };
}

/** A camera `distance` metres from (x, 1000), looking north and down at it. */
function view(x: number, distance: number): DetailView {
  return setDetailView(
    createDetailView(),
    {
      target: [x, 1000, 0],
      distance,
      pitch: 0.85,
      yaw: Math.PI / 2,
      fovY: 0.8,
      aspect: 16 / 9,
      near: 1,
    },
    1080,
  );
}

/** Every sorted record of kind `k` in `[first, count]` ranges, as the placed
 *  instance it came from. */
const instancesIn = (pop: StaticChunks, k: number, ranges: number[]) => {
  const out: number[] = [];
  for (let r = 0; r < ranges.length; r += 2)
    for (let i = ranges[r]; i < ranges[r] + ranges[r + 1]; i++) out.push(pop.order[k][i]);
  return out;
};

test("records of any stride are kept whole, in chunk order, each knowing which placed instance it is", () => {
  // Placed out of chunk order: east, west, middle, and a second one west.
  const at: [number, number][] = [
    [1300, 1000],
    [1000, 1000],
    [1150, 1000],
    [1010, 1000],
  ];
  for (const stride of [5, 12, 16]) {
    const pop = createStaticChunks(placed(at, stride), 1, CHUNK_M, 4);
    expect(pop.sorted[0]).toHaveLength(at.length * stride);
    // West to east along the row; within a chunk, as placed.
    expect([...pop.order[0]]).toEqual([1, 3, 2, 0]);
    pop.order[0].forEach((from, to) => {
      const record = [...pop.sorted[0].subarray(to * stride, (to + 1) * stride)];
      expect(record).toEqual([...at[from], 0, ...Array.from({ length: stride - 3 }, () => from)]);
    });
    expect(pop.chunks.map((c) => [c.start[0], c.end[0]])).toEqual([
      [0, 2],
      [2, 3],
      [3, 4],
    ]);
  }
});

test("each kind has its own buffer, and a chunk its range in each", () => {
  const at: [number, number][] = [
    [1000, 1000],
    [1010, 1000],
    [1150, 1000],
    [1020, 1000],
  ];
  const pop = createStaticChunks(placed(at, 16, [1, 0, 1, 1]), 2, CHUNK_M, 4);
  expect([...pop.order[0]]).toEqual([1]);
  expect([...pop.order[1]]).toEqual([0, 3, 2]);
  expect(pop.chunks.map((c) => [c.start[1], c.end[1]])).toEqual([
    [0, 2],
    [2, 3],
  ]);
});

test("a chunk draws whole at the level its population gives it, a fifth one included, neighbours merged into one range", () => {
  // Four chunks along a row, all in view from 600 m.
  const at: [number, number][] = [
    [900, 1000],
    [970, 1000],
    [1030, 1000],
    [1100, 1000],
  ];
  const CARD = 4;
  const pop = createStaticChunks(placed(at, 16), 1, CHUNK_M, CARD + 1);
  // The two western chunks as cards, the next at the coarsest mesh, the last near.
  const levels = [CARD, CARD, 3, NEAR];
  selectChunks(pop, view(1000, 600), (_chunk, index) => levels[index], null);
  expect(pop.ranges[0][CARD]).toEqual([0, 2]);
  expect(instancesIn(pop, 0, pop.ranges[0][CARD])).toEqual([0, 1]);
  expect(instancesIn(pop, 0, pop.ranges[0][3])).toEqual([2]);
  expect(pop.ranges[0].slice(0, 3).every((r) => r.length === 0)).toBe(true);
  expect(pop.near.map((c) => pop.order[0][pop.chunks[c].start[0]])).toEqual([3]);
  // Nothing casts unless the population does.
  expect(pop.cast[0]).toEqual([]);
});

test("a chunk's level is asked with its nearest distance and its largest instance, and only when it is in view", () => {
  const at: [number, number][] = [
    [1000, 1000],
    [5000, 1000],
  ];
  const pop = createStaticChunks(placed(at, 16), 1, CHUNK_M, 5);
  const v = view(1000, 300);
  const asked: number[][] = [];
  selectChunks(
    pop,
    v,
    (chunk, index, distance) => {
      asked.push([index, chunk.size, distance]);
      return 4;
    },
    null,
  );
  // Only the chunk under the camera: 300 m from the eye to its centre on the
  // ground, less at most the box's reach toward the eye.
  expect(asked).toHaveLength(1);
  expect(asked[0].slice(0, 2)).toEqual([0, SIZE_M]);
  expect(asked[0][2]).toBeGreaterThan(300 - 3);
  expect(asked[0][2]).toBeLessThanOrEqual(300);
  expect(instancesIn(pop, 0, pop.ranges[0][4])).toEqual([0]);
});

test("near chunks' instances are staged by the level each one's own distance gives it", () => {
  // Two in one chunk under the camera, 40 m apart; the rule splits them by distance.
  const at: [number, number][] = [
    [1000, 1000],
    [1000, 1040],
  ];
  const pop = createStaticChunks(placed(at, 12), 1, 128, 4);
  const staged = createStagedLevels(pop);
  const v = view(1000, 100);
  selectChunks(pop, v, () => NEAR, null);
  const eyeTo = (i: number) => Math.hypot(at[i][0] - v.eye[0], at[i][1] - v.eye[1], 1 - v.eye[2]);
  const split = (eyeTo(0) + eyeTo(1)) / 2;
  stageNear(pop, staged, v, (_size, distance) => (distance < split ? 1 : 2));
  expect(staged.counts[0]).toEqual([0, 1, 1, 0]);
  const nearer = eyeTo(0) < eyeTo(1) ? 0 : 1;
  expect(staged.records[0][1][3]).toBe(nearer);
  expect(staged.records[0][2][3]).toBe(1 - nearer);
  // A later view with nothing near stages nothing.
  selectChunks(pop, v, () => 3, null);
  stageNear(pop, staged, v, () => 0);
  expect(staged.counts[0]).toEqual([0, 0, 0, 0]);
});

test("without a view every chunk is near, culled or not", () => {
  const at: [number, number][] = [
    [1000, 1000],
    [5000, 1000],
  ];
  const pop = createStaticChunks(placed(at, 16), 1, CHUNK_M, 5);
  selectChunks(pop, view(1000, 300), () => 4, null);
  selectEveryChunk(pop);
  expect(pop.near).toEqual([0, 1]);
  expect(pop.ranges[0].every((r) => r.length === 0)).toBe(true);
});
