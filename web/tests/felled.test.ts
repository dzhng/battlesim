// Felled trees (`scenery/felled.ts`): the fall's motion, the lying shape the
// rules bound, and the stump. Fixed rules, not the biome's, so a tuning edit
// never moves these.
import { expect, test } from "vitest";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import {
  fallAngle,
  felledPoint,
  felledRecords,
  FELLED_FLOATS,
  groupByKey,
  fellPose,
  lyingShare,
  restAngle,
  stumpMesh,
  stumpRecords,
  treeIndex,
} from "@packages/battle-renderer/src/scenery/felled";
import { TREE_FIELD, TREE_FLOATS } from "@packages/battle-renderer/src/scenery/placement";
import type { FelledRules } from "@packages/battle-renderer/src/terrain/biome";

const RULES: FelledRules = {
  fall_s: 2,
  settle_s: 0.5,
  settle_deg: 5,
  rest_height_m: 1,
  rest_spread: 0.6,
  stump_height_m: 0.4,
  cut: [0.4, 0.3, 0.2],
};
const QUARTER = Math.PI / 2;

test("a tree tips over gathering speed, strikes the ground at fall_s, rocks and lies", () => {
  const rest = restAngle(0.45, 10);
  expect(rest).toBeGreaterThan(QUARTER);
  expect(fallAngle(RULES, -1, rest)).toBe(0);
  expect(fallAngle(RULES, 0, rest)).toBe(0);
  let last = 0,
    lastStep = 0;
  for (let k = 1; k <= 20; k++) {
    const angle = fallAngle(RULES, (k / 20) * RULES.fall_s, rest);
    // It never slows while it falls.
    expect(angle - last).toBeGreaterThan(lastStep - 1e-12);
    lastStep = angle - last;
    last = angle;
  }
  expect(fallAngle(RULES, RULES.fall_s, rest)).toBeCloseTo(rest, 12);
  // It rocks back up no more than settle_deg, and never through the ground.
  const settle = Array.from({ length: 50 }, (_, k) =>
    fallAngle(RULES, RULES.fall_s + (k / 50) * RULES.settle_s, rest),
  );
  expect(Math.min(...settle)).toBeGreaterThan(rest - (RULES.settle_deg * Math.PI) / 180 - 1e-9);
  expect(Math.max(...settle)).toBeLessThanOrEqual(rest);
  expect(fallAngle(RULES, RULES.fall_s + RULES.settle_s, rest)).toBe(rest);
  expect(fallAngle(RULES, 1000, rest)).toBe(rest);
});

/** A tree as points about its foot: a trunk of radius `trunk` up to `top`
 *  (bark), and a crown of leaves `reach` metres about its axis from
 *  `crown` up. */
function treePoints(top: number, reach: number, trunk: number, crown: number) {
  const out: { p: [number, number, number]; foliage: boolean }[] = [];
  for (let k = 0; k < 64; k++) {
    const a = (k / 64) * Math.PI * 2;
    const [c, s] = [Math.cos(a), Math.sin(a)];
    for (let z = 0; z <= top + 1e-9; z += top / 40)
      out.push({ p: [trunk * c, trunk * s, z], foliage: false });
    for (let z = crown; z <= top + 1e-9; z += (top - crown) / 8)
      out.push({ p: [reach * c, reach * s, z], foliage: true });
  }
  return out;
}

test("a lying tree rests beside its stump, round-trunked, its crown pressed under rest_height_m", () => {
  const [top, reach, trunk, crown] = [11, 4.2, 0.45, 4];
  const toward: [number, number] = [Math.SQRT1_2, -Math.SQRT1_2];
  const pose = fellPose(RULES, toward, 1000, top, reach, trunk, crown);
  expect(pose.landed).toBe(1);
  const lying = treePoints(top, reach, trunk, crown).map(({ p, foliage }) => ({
    at: felledPoint(p, foliage, pose),
    foliage,
  }));
  for (const { at } of lying) expect(at[2]).toBeLessThanOrEqual(RULES.rest_height_m + 1e-9);
  const along = ([x, y]: readonly number[]) => x * toward[0] + y * toward[1];
  // Its foot rests on the ground a trunk's radius up, its top on the ground.
  expect(felledPoint([0, 0, RULES.stump_height_m], false, pose)[2]).toBeCloseTo(trunk, 9);
  const tip = felledPoint([0, 0, top], false, pose);
  expect(tip[2]).toBeCloseTo(0, 9);
  expect(along(tip)).toBeGreaterThan(0.99 * (top - RULES.stump_height_m));
  // It lies clear of its stump (as wide as its trunk), which stands bare.
  expect(Math.min(...lying.map(({ at }) => along(at)))).toBeGreaterThan(trunk);
  // Its trunk keeps its girth: by the foot it is a trunk across, not a plank.
  const crest = Math.max(...lying.map(({ at }) => (along(at) < 2 ? at[2] : -Infinity)));
  expect(crest).toBeGreaterThan(1.9 * trunk);
  // Across the fall its crown narrows to rest_spread of its width.
  const across = Math.max(
    ...lying.map(({ at: [x, y], foliage }) =>
      foliage ? Math.abs(x * toward[1] - y * toward[0]) : 0,
    ),
  );
  expect(across).toBeCloseTo(reach * RULES.rest_spread, 9);
  // Upright, it is the tree as it stood above its stump; below the cut the
  // stump stands in its place, so it ends at the cut from its first frame.
  const upright = fellPose(RULES, toward, 0, top, reach, trunk, crown);
  const hinge = RULES.stump_height_m;
  for (const { p, foliage } of treePoints(top, reach, trunk, crown))
    expect(felledPoint(p, foliage, upright)).toEqual(
      [p[0], p[1], Math.max(p[2], hinge)].map((v) => expect.closeTo(v, 12)),
    );
  const tipping = fellPose(RULES, toward, RULES.fall_s / 2, top, reach, trunk, crown);
  for (const { p, foliage } of treePoints(top, reach, trunk, crown))
    if (p[2] <= hinge)
      expect(felledPoint(p, foliage, tipping)[2]).toBeGreaterThanOrEqual(hinge - trunk - 1e-9);
});

test("a falling tree lands in its second half, smoothly, and lies all of it on the ground", () => {
  const rest = restAngle(0.45, 10);
  expect(lyingShare(0, rest)).toBe(0);
  expect(lyingShare(rest / 2, rest)).toBe(0);
  expect(lyingShare(rest, rest)).toBe(1);
  let last = 0;
  for (let k = 1; k <= 20; k++) {
    const share = lyingShare((k / 20) * rest, rest);
    expect(share).toBeGreaterThanOrEqual(last);
    // No jump: a twentieth of the fall lands at most a sixth of it.
    expect(share - last).toBeLessThan(1 / 6);
    last = share;
  }
});

/** Two placed trees of kinds 0 and 1, by trunk ids 40 and 7. */
function placed(): { data: Float32Array; ids: Uint32Array } {
  const data = new Float32Array(2 * TREE_FLOATS);
  const set = (i: number, x: number, kind: number) => {
    const o = i * TREE_FLOATS;
    data[o + TREE_FIELD.x] = x;
    data[o + TREE_FIELD.y] = 20;
    data[o + TREE_FIELD.z] = 3;
    data[o + TREE_FIELD.scaleXY] = 0.9;
    data[o + TREE_FIELD.scaleZ] = 1.1;
    data[o + TREE_FIELD.kind] = kind;
    data[o + TREE_FIELD.r] = data[o + TREE_FIELD.g] = data[o + TREE_FIELD.b] = 1;
  };
  set(0, 10, 0);
  set(1, 30, 1);
  return { data, ids: Uint32Array.from([40, 7]) };
}
const SIZES = [
  { height: 10, radius: 4 },
  { height: 12, radius: 3 },
];
const KINDS = [
  { trunk: 0.4, crown: 3 },
  { trunk: 0.45, crown: 4 },
];

test("a felled tree is found by its trunk; one the forest never drew is skipped", () => {
  const { data, ids } = placed();
  const index = treeIndex(ids);
  const felled = [
    { prop: 7, toward: [1, 0] as const, fellAt: 10 },
    { prop: 999, toward: [0, 1] as const, fellAt: 10 },
  ];
  const now = felledRecords(felled, data, index, SIZES, KINDS, RULES, 11);
  expect(Array.from(now.kinds)).toEqual([1]);
  expect(now.records.length).toBe(FELLED_FLOATS);
  expect([now.records[0], now.records[1], now.records[2]]).toEqual([30, 20, 3]);
  expect(now.heights[0]).toBeCloseTo(12 * 1.1, 5);
  expect(now.moving).toBe(true);
  const later = felledRecords(
    felled,
    data,
    index,
    SIZES,
    KINDS,
    RULES,
    10 + RULES.fall_s + RULES.settle_s,
  );
  expect(later.moving).toBe(false);
  // Its stump stands where it grew, as wide as it was.
  const stumps = stumpRecords(felled, data, index, RULES.stump_height_m);
  expect(Array.from(stumps.kinds)).toEqual([1]);
  expect([stumps.records[0], stumps.records[1], stumps.records[4]]).toEqual([
    30,
    20,
    expect.closeTo(0.9, 6),
  ]);
});

test("a stump is the trunk's width at its height, its bark's colour, cut at its height", () => {
  // A low-poly trunk as the tree appearances are: a ring at the foot 0.5 m
  // out, the next ring 1 m up 0.3 m out, and leaves that are no bark.
  const v: number[] = [];
  const push = (x: number, y: number, z: number, rgb: number[], foliage: number) =>
    v.push(x, y, z, 0, 0, 1, rgb[0], rgb[1], rgb[2], foliage);
  for (let k = 0; k < 6; k++) {
    const a = (k / 6) * Math.PI * 2;
    push(0.5 * Math.cos(a), 0.5 * Math.sin(a), 0, [0.2, 0.1, 0.05], 0);
    push(0.3 * Math.cos(a), 0.3 * Math.sin(a), 1, [0.2, 0.1, 0.05], 0);
    push(0.2 * Math.cos(a), 0.2 * Math.sin(a), 4, [0.9, 0.9, 0.9], 0);
  }
  push(2.5, 0, 0.2, [0, 0.5, 0], 1);
  const mesh = stumpMesh(Float32Array.from(v), 0.5, RULES.cut);
  let top = 0,
    reach = 0;
  const colours = new Set<string>();
  for (let o = 0; o < mesh.length; o += VERTEX_FLOATS) {
    top = Math.max(top, mesh[o + 2]);
    reach = Math.max(reach, Math.hypot(mesh[o], mesh[o + 1]));
    colours.add([mesh[o + 6], mesh[o + 7], mesh[o + 8]].map((c) => c.toFixed(3)).join(","));
    expect(mesh[o + 9]).toBe(0);
  }
  expect(top).toBeCloseTo(0.5, 9);
  // Half way between the rings' radii, at half the ring's height.
  expect(reach).toBeCloseTo(0.4, 6);
  expect([...colours].sort()).toEqual(["0.200,0.100,0.050", "0.400,0.300,0.200"]);
});

test("records group by key in ascending runs, each record whole", () => {
  // Three two-float records keyed 2, 0, 2.
  const { records, runs } = groupByKey(Float32Array.from([1, 1, 2, 2, 3, 3]), 2, [2, 0, 2]);
  expect(Array.from(records)).toEqual([2, 2, 1, 1, 3, 3]);
  expect(runs).toEqual([0, 0, 1, 2, 1, 2]);
});
