// @vitest-environment node
// The placement pool: a fixed number of instance records, kept kind by kind,
// so a kind draws as one range however many chunks' instances come and go.
import { expect, test } from "vitest";
import {
  createPlacementPool,
  flushPool,
  poolAppend,
  poolRemove,
  type PlacementPool,
} from "@packages/battle-renderer/src/models/placementPool";

const STRIDE = 4;
/** A record recognisable wherever it lands: its tag in every float. */
const record = (tag: number) => new Float32Array(STRIDE).fill(tag);

/** The tags a kind draws, in the order it draws them. */
function drawn(pool: PlacementPool, kind: number): number[] {
  const out: number[] = [];
  for (let i = 0; i < pool.count[kind]; i++) {
    const at = (pool.first[kind] + i) * STRIDE;
    const tags = pool.records.subarray(at, at + STRIDE);
    expect(new Set(tags).size).toBe(1);
    out.push(tags[0]);
  }
  return out;
}

/** The pool as a GPU buffer would hold it after every flush so far. */
function mirror(pool: PlacementPool, gpu: Float32Array) {
  flushPool(pool, (first, count) =>
    gpu.set(pool.records.subarray(first * STRIDE, (first + count) * STRIDE), first * STRIDE),
  );
}

/** No two kinds' ranges overlap, and every one lies inside the pool. */
function expectDisjoint(pool: PlacementPool) {
  const ranges = Array.from(pool.count, (count, k) => [pool.first[k], pool.first[k] + count])
    .filter(([a, b]) => b > a)
    .sort((a, b) => a[0] - b[0]);
  ranges.forEach(([a, b], i) => {
    expect(a).toBeGreaterThanOrEqual(i ? ranges[i - 1][1] : 0);
    expect(b).toBeLessThanOrEqual(pool.capacity);
  });
}

test("each kind's records are one range, whatever order they arrive in", () => {
  const pool = createPlacementPool(64, STRIDE, 3);
  const tags = [10, 20, 11, 30, 21, 12, 13, 14, 15, 16, 17, 18, 19];
  tags.forEach((tag) => poolAppend(pool, Math.floor(tag / 10) - 1, record(tag), 0));
  expect(drawn(pool, 0).sort()).toEqual([10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
  expect(drawn(pool, 1).sort()).toEqual([20, 21]);
  expect(drawn(pool, 2)).toEqual([30]);
  expectDisjoint(pool);
});

test("removing a record leaves the others drawn, and its handle's place to the next", () => {
  const pool = createPlacementPool(64, STRIDE, 2);
  const handles = [1, 2, 3, 4, 5].map((tag) => poolAppend(pool, 0, record(tag), 0));
  const other = poolAppend(pool, 1, record(9), 0);
  poolRemove(pool, handles[1]);
  poolRemove(pool, handles[4]);
  expect(drawn(pool, 0).sort()).toEqual([1, 3, 4]);
  // A record moved to fill the hole is still removed by its own handle.
  poolRemove(pool, handles[3]);
  expect(drawn(pool, 0).sort()).toEqual([1, 3]);
  expect(drawn(pool, 1)).toEqual([9]);
  poolRemove(pool, other);
  poolRemove(pool, handles[0]);
  poolRemove(pool, handles[2]);
  expect(pool.used).toBe(0);
});

test("a full pool refuses a record and keeps what it holds", () => {
  const pool = createPlacementPool(32, STRIDE, 2);
  const taken: number[] = [];
  for (let tag = 0; ; tag++) {
    const handle = poolAppend(pool, tag % 2, record(tag), 0);
    if (handle < 0) break;
    taken.push(tag);
    expect(tag).toBeLessThan(32);
  }
  // It holds a useful share of its capacity before it refuses.
  expect(taken.length).toBeGreaterThanOrEqual(16);
  expect(pool.used).toBe(taken.length);
  expect([...drawn(pool, 0), ...drawn(pool, 1)].sort((a, b) => a - b)).toEqual(taken);
  expectDisjoint(pool);
});

test("records freed by one kind are room for another", () => {
  const pool = createPlacementPool(32, STRIDE, 3);
  for (let round = 0; round < 40; round++) {
    const kind = round % 3;
    const handles = Array.from({ length: 12 }, (_, i) =>
      poolAppend(pool, kind, record(round * 100 + i), 0),
    );
    expect(handles.every((h) => h >= 0)).toBe(true);
    expect(drawn(pool, kind)).toHaveLength(12);
    expectDisjoint(pool);
    for (const h of handles) poolRemove(pool, h);
  }
  expect(pool.used).toBe(0);
});

test("a flush writes exactly what changed since the last, so a buffer that takes every flush matches", () => {
  const pool = createPlacementPool(128, STRIDE, 4);
  const gpu = new Float32Array(pool.records.length);
  // A seeded churn: chunks of records arriving and leaving, across kinds.
  let seed = 7;
  const random = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff;
  const live: number[] = [];
  let tag = 1;
  for (let step = 0; step < 300; step++) {
    if (live.length && random() < 0.45) {
      poolRemove(pool, live.splice(Math.floor(random() * live.length), 1)[0]);
    } else {
      const handle = poolAppend(pool, Math.floor(random() * 4), record(tag++), 0);
      if (handle >= 0) live.push(handle);
    }
    if (step % 7 === 0) {
      mirror(pool, gpu);
      for (let k = 0; k < 4; k++) {
        const [a, b] = [pool.first[k] * STRIDE, (pool.first[k] + pool.count[k]) * STRIDE];
        expect([...gpu.subarray(a, b)]).toEqual([...pool.records.subarray(a, b)]);
      }
    }
    expect(pool.used).toBe(live.length);
    expectDisjoint(pool);
  }
  // Nothing changed: nothing is written.
  mirror(pool, gpu);
  let writes = 0;
  flushPool(pool, () => writes++);
  expect(writes).toBe(0);
});
