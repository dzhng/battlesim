// @vitest-environment node
// A map's scenery as word tables for a second client: every tree, shrub and
// piece of dressing as the scenery layer instances it, with the forest's
// trunk ids, and the tier rule's numbers.
import { expect, test } from "vitest";
import { treeInstances } from "@packages/battle-renderer/src/scenery/lod";
import {
  TREE_FLOATS,
  type DressingField,
  type KindSize,
  type SceneryPlacement,
} from "@packages/battle-renderer/src/scenery/placement";
import { NO_BODY, sceneryTables } from "@packages/battle-renderer/src/scenery/sceneryTables";
import { readRecords } from "./support/wordRecords";

const SIZES: KindSize[] = [
  { height: 12, radius: 4 },
  { height: 2, radius: 1.5 },
  { height: 0.6, radius: 0.4 },
];

/** A tree of `kind` at (x, y), turned and scaled a little by its index. */
const tree = (kind: number, x: number, y: number, i: number) => [
  x,
  y,
  1 + i,
  0.3 * i,
  1 + 0.1 * i,
  0.9 + 0.05 * i,
  kind,
  0.8,
  0.9,
  1,
];

/** A dressing field of two cells, (3, 4) with `n` pieces and (5, 4) empty. */
function field(n: number): DressingField {
  return {
    cellM: 64,
    capacity: 8,
    kinds: [2],
    topM: 0.6,
    reachM: 0.4,
    ground: [-1, 9],
    cells: Int32Array.of(3, 4, 5, 4),
    place: (i, j) =>
      Float32Array.from(
        i === 3
          ? Array.from({ length: n }, (_, p) => tree(2, i * 64 + p, j * 64 + p, p)).flat()
          : [],
      ),
  };
}

const placement: SceneryPlacement = {
  kinds: ["oak", "hazel", "fern"],
  forest: Float32Array.from([...tree(0, 10, 20, 0), ...tree(0, 30, 25, 1)]),
  forestIds: Uint32Array.of(901, 917),
  understorey: Float32Array.from(tree(1, 40, 20, 2)),
  backdrop: Float32Array.from([...tree(0, -500, 0, 3), ...tree(1, -520, 5, 4)]),
  dressing: field(3),
};

const RULES = {
  lodPx: [400, 150, 40] as const,
  chunkM: 128,
  casterCoarser: 1,
  dressing: { lodPx: [300, 100, 20] as const, fadePx: 6, slots: 2, cellsPerView: 2 },
  felled: {
    fall_s: 2.6,
    settle_s: 0.6,
    settle_deg: 4,
    rest_height_m: 1,
    rest_spread: 0.6,
    stump_height_m: 0.4,
    cut: [0.26, 0.19, 0.11] as const,
  },
};

test("every tree, shrub and piece of dressing is exported as the layer instances it", () => {
  const { tables, kinds } = sceneryTables(placement, SIZES, RULES);
  expect(kinds).toEqual(["oak", "hazel", "fern"]);
  const dressing = placement.dressing.place(3, 4);
  for (const [name, placed] of [
    ["forest", placement.forest],
    ["understorey", placement.understorey],
    ["backdrop", placement.backdrop],
    ["dressing", dressing],
  ] as const) {
    const want = treeInstances(placed, SIZES);
    const rows = readRecords(tables[name]);
    expect(rows, name).toHaveLength(want.kinds.length);
    rows.forEach((row, i) => {
      const record = [...want.records.subarray(i * 12, i * 12 + 12)];
      expect([...row.pose, ...row.shape, ...row.tint], `${name} ${i}`).toEqual(record);
      expect(row.kind).toEqual([want.kinds[i]]);
      expect(row.height).toEqual([want.heights[i]]);
      expect(row.reach).toEqual([want.reaches[i]]);
    });
  }
  // A forest tree names the simulation's trunk it stands on, so a fall finds
  // it; nothing else has a body.
  expect(readRecords(tables.forest).map((r) => r.id[0])).toEqual([901, 917]);
  for (const name of ["understorey", "backdrop", "dressing"] as const)
    expect(
      readRecords(tables[name]).every((r) => r.id[0] === NO_BODY),
      name,
    ).toBe(true);
  // The dressing is laid cell by cell, every cell the forests reach.
  expect(readRecords(tables.dressingCells)).toEqual([
    { cell: [3, 4], first: [0], count: [3] },
    { cell: [5, 4], first: [3], count: [0] },
  ]);
  expect(dressing.length / TREE_FLOATS).toBe(3);
});

test("the tier rule's numbers and how a felled tree falls travel with the tables", () => {
  const { tiers, felled } = sceneryTables(placement, SIZES, RULES);
  expect(felled).toEqual(RULES.felled);
  expect(tiers).toEqual({
    lodPx: [400, 150, 40],
    chunkM: 128,
    casterCoarser: 1,
    dressing: {
      lodPx: [300, 100, 20],
      fadePx: 6,
      gone: 0.5,
      cellM: 64,
      topM: 0.6,
      reachM: 0.4,
      ground: [-1, 9],
      slots: 2,
      cellsPerView: 2,
    },
  });
});
