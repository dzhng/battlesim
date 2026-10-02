// @vitest-environment node
// The forest floor's dressing as the scenery layer keeps it: only the cells a
// camera can see pieces in are laid, nearest first and a few a view, into a
// pool that never grows.
import { expect, test } from "vitest";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView.ts";
import {
  createDressingCache,
  dressingCounts,
  dressingReach,
  selectDressing,
  setDressingCleared,
} from "@packages/battle-renderer/src/scenery/dressing.ts";
import { INSTANCE_FLOATS, type TierView } from "@packages/battle-renderer/src/scenery/lod.ts";
import {
  TREE_FLOATS,
  type DressingField,
  type KindSize,
} from "@packages/battle-renderer/src/scenery/placement.ts";

const CELL_M = 64;
const PER_CELL = 6;
const SIZES: KindSize[] = [
  { height: 11, radius: 5 },
  { height: 0.8, radius: 0.5 },
  { height: 0.4, radius: 0.6 },
];

/** A square wood of `side` cells a side, every cell holding `PER_CELL` pieces
 *  of two kinds; `laid` counts each cell's layings. */
function wood(side: number) {
  const laid = new Map<string, number>();
  const cells: number[] = [];
  for (let j = 0; j < side; j++) for (let i = 0; i < side; i++) cells.push(i, j);
  const field: DressingField = {
    cellM: CELL_M,
    capacity: PER_CELL,
    kinds: [1, 2],
    topM: 0.8,
    reachM: 0.6,
    ground: [0, 0],
    cells: Int32Array.from(cells),
    place(i, j) {
      laid.set(`${i},${j}`, (laid.get(`${i},${j}`) ?? 0) + 1);
      const out = new Float32Array(PER_CELL * TREE_FLOATS);
      for (let p = 0; p < PER_CELL; p++)
        out.set(
          [
            (i + (p + 0.5) / PER_CELL) * CELL_M,
            (j + 0.5) * CELL_M,
            0,
            0,
            1,
            1,
            1 + (p % 2),
            1,
            1,
            1,
          ],
          p * TREE_FLOATS,
        );
      return out;
    },
  };
  return { field, laid };
}

const FADE_PX = 6;
/** The play camera over (x, y) from `distance` metres (at `pitch`: π/2 looks
 *  straight down), on a 1080 px viewport. */
function viewOver(x: number, y: number, distance: number, pitch = 0.85): TierView {
  const view: TierView = {
    ...createDetailView(),
    lodPx: [40, 14, 6],
    shadow: { fall: [0, 0], reach: 0 },
  };
  setDetailView(
    view,
    {
      target: [x, y, 0],
      distance,
      yaw: -Math.PI / 2,
      pitch,
      fovY: 0.8,
      aspect: 16 / 9,
      near: 1,
    },
    1080,
  );
  return view;
}

/** Select until nothing is pending; returns the selections it took and what
 *  was written, slot by slot. */
function settle(cache: ReturnType<typeof createDressingCache>, view: TierView, budget = 2) {
  const written = new Map<number, Float32Array>();
  let selections = 0;
  do {
    selectDressing(cache, view, budget, (slot, records) => written.set(slot, records));
    selections++;
  } while (cache.pending && selections < 1000);
  return { selections, written };
}

const drawn = (cache: ReturnType<typeof createDressingCache>) => cache.drawn.flat();

test("only the cells the camera sees pieces in are laid, and a camera too far to see any lays none", () => {
  const { field, laid } = wood(40);
  const cache = createDressingCache(field, SIZES, FADE_PX, 128);
  const near = viewOver(1280, 1280, 65);
  settle(cache, near);
  expect(drawn(cache).length).toBeGreaterThan(0);
  // Far fewer than the wood's 1600 cells, and none past where the tallest
  // piece has shrunk to nothing.
  expect(laid.size).toBeLessThan(60);
  const reach = dressingReach(cache, near);
  for (const cell of drawn(cache)) expect(cell.distance).toBeLessThan(reach);
  // Looking straight down from past that distance every piece in view has
  // shrunk to nothing: no cell is drawn or laid for it.
  const before = laid.size;
  settle(cache, viewOver(1280, 1280, reach * 1.1, Math.PI / 2));
  expect(drawn(cache)).toEqual([]);
  expect(laid.size).toBe(before);
  // Just inside it, the cells under the camera are.
  settle(cache, viewOver(1280, 1280, reach * 0.9, Math.PI / 2));
  expect(drawn(cache).length).toBeGreaterThan(0);
});

test("a cut's cells are laid a few a view, nearest first, and a settled view lays nothing more", () => {
  const { field, laid } = wood(40);
  const cache = createDressingCache(field, SIZES, FADE_PX, 128);
  const view = viewOver(1280, 1280, 120);
  selectDressing(cache, view, 2, () => {});
  expect(laid.size).toBe(2);
  expect(cache.pending).toBe(true);
  const first = drawn(cache).map((cell) => cell.distance);
  const { selections } = settle(cache, view);
  expect(selections).toBeGreaterThan(2);
  // The first two laid were the nearest of all that the view wants.
  const all = drawn(cache)
    .map((cell) => cell.distance)
    .sort((a, b) => a - b);
  expect(first.sort((a, b) => a - b)).toEqual(all.slice(0, 2));
  const total = [...laid.values()].reduce((sum, n) => sum + n, 0);
  selectDressing(cache, view, 2, () => {});
  expect(cache.pending).toBe(false);
  expect([...laid.values()].reduce((sum, n) => sum + n, 0)).toBe(total);
});

test("a cell's records are written kind by kind into its own slot, each piece's last float its height over the fade", () => {
  const { field } = wood(4);
  const cache = createDressingCache(field, SIZES, FADE_PX, 8);
  const { written } = settle(cache, viewOver(128, 128, 65));
  expect(written.size).toBeGreaterThan(0);
  for (const cell of drawn(cache)) {
    const records = written.get(cell.slot)!;
    expect(records.length).toBe(PER_CELL * INSTANCE_FLOATS);
    expect(cell.starts).toEqual([0, PER_CELL / 2, PER_CELL]);
    for (let p = 0; p < PER_CELL; p++) {
      const size = SIZES[p < PER_CELL / 2 ? 1 : 2];
      expect(records[p * INSTANCE_FLOATS + INSTANCE_FLOATS - 1]).toBeCloseTo(
        size.height / FADE_PX,
        6,
      );
      // Inside its own cell.
      expect(Math.floor(records[p * INSTANCE_FLOATS] / CELL_M)).toBe(cell.i);
    }
  }
  // No two drawn cells share a slot.
  expect(new Set(drawn(cache).map((cell) => cell.slot)).size).toBe(drawn(cache).length);
});

test("the pool never grows: the cell wanted longest ago gives up its slot, and a view wider than the pool draws its nearest", () => {
  const { field, laid } = wood(40);
  const slots = 12;
  const cache = createDressingCache(field, SIZES, FADE_PX, slots);
  // Pan across the wood: every view is drawn in full though the pool holds
  // only a dozen cells.
  for (let x = 300; x < 2300; x += 150) {
    settle(cache, viewOver(x, 1280, 40));
    expect(drawn(cache).length).toBeGreaterThan(0);
    expect(drawn(cache).length).toBeLessThanOrEqual(slots);
    expect(cache.held.filter((cell) => cell !== null).length).toBeLessThanOrEqual(slots);
  }
  // Coming back lays the first cells again: they were given up.
  settle(cache, viewOver(300, 1280, 40));
  expect(Math.max(...laid.values())).toBe(2);
  // A view that wants more cells than there are slots settles, drawing the
  // nearest.
  const wide = viewOver(1280, 1280, 200);
  const { selections } = settle(cache, wide);
  expect(selections).toBeLessThan(100);
  expect(drawn(cache).length).toBe(slots);
  expect(dressingCounts(cache).held).toBe(slots * PER_CELL);
});

test("dressing on ground the side has seen cleared is laid with no size, once the cleared ground changes", () => {
  const { field } = wood(4);
  const cache = createDressingCache(field, SIZES, FADE_PX, 8);
  const view = viewOver(128, 128, 65);
  settle(cache, view);
  setDressingCleared(cache, (x) => x < CELL_M * 2);
  const { written } = settle(cache, view);
  expect(written.size).toBeGreaterThan(0);
  for (const cell of drawn(cache)) {
    const records = written.get(cell.slot)!;
    for (let o = 0; o < records.length; o += INSTANCE_FLOATS) {
      const gone = records[o] < CELL_M * 2;
      expect([records[o + 4], records[o + 5], records[o + 6]]).toEqual(
        gone ? [0, 0, 0] : [1, 1, 1],
      );
    }
  }
});
