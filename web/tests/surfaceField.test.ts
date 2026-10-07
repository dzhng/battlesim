// @vitest-environment node
// The surface field against the all-primitives distance it replaces: a lookup
// through the bucket index is the same signed distance wherever a consumer
// reads it, on the simulation's own exports and on a map dense enough that
// the index matters.
import { TEST_RULES } from "./catalog";
import { readFileSync } from "node:fs";
import { afterEach, beforeAll, expect, test } from "vitest";
import { vec2, type Vec2 } from "math";
import { mulberry32 } from "math/random";
import { segment2, triangle2 } from "math/shapes";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import { readWorldExports, type WorldLayout } from "@packages/battle-renderer/src/worldMesh.ts";
import {
  buildTerrainSurface,
  RECT_FLOATS,
  type TerrainSite,
} from "@packages/battle-renderer/src/terrain/terrainSurface.ts";
import { forestInside, type ForestShape } from "@packages/battle-renderer/src/terrain/forestShapes";
import {
  CUT_A,
  CUT_B,
  STROKE_CUTS,
  STROKE_FLOATS,
  strokeInside,
} from "@packages/battle-renderer/src/terrain/strokes";
import { RIVER_FLOATS, stretchInside } from "@packages/battle-renderer/src/terrain/rivers";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import {
  areaBearings,
  buildSurfaceField,
  forestDistance,
  pavedDistance,
  surfaceListBudget,
  SURFACE_FLOATS,
  SURFACE_MAX_CELLS,
  SURFACE_STROKE_ALONG,
  surfaceCell,
  waterDistance,
  type SurfaceField,
  type SurfaceReach,
} from "@packages/battle-renderer/src/terrain/surfaceField.ts";
import { CURATED_GROUND, denseGround } from "./surfaceGrounds";
import summer from "@fixtures/biomes/summer.json";
import { loadMap } from "@web/maps/node";

const geometry = loadMap("geometry").definition;
const villageMap = loadMap("village").definition;

const biome = validateBiome(summer as unknown as Biome);
let layout: WorldLayout;
const views: WorldView[] = [];
afterEach(() => {
  for (const view of views.splice(0)) view.free();
});
beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  layout = JSON.parse(world_layout(JSON.stringify(TEST_RULES))) as WorldLayout;
});

function siteOf(map: unknown): TerrainSite {
  const view = new WorldView(JSON.stringify(map), JSON.stringify(TEST_RULES));
  views.push(view);
  return buildTerrainSurface(readWorldExports(view), layout, biome).site;
}

/** How far each rule is read at a pixel `footprint` metres wide: the shape of
 *  the material's own reach (a fixed part and the pixel), not its numbers. */
const reach = (footprint: number): SurfaceReach => ({
  paved: 1.4 + footprint,
  forest: 5 + footprint,
  water: 2.5 + footprint / 2,
});

// The distances the terrain material looped over every primitive for.
const _p: Vec2 = [0, 0],
  _a: Vec2 = [0, 0],
  _b: Vec2 = [0, 0],
  _c: Vec2 = [0, 0],
  _near: Vec2 = [0, 0];
function segmentDistance(records: Float32Array, o: number): number {
  vec2.fromBuffer(_a, records, o);
  vec2.fromBuffer(_b, records, o + 2);
  segment2.closestPoint(_near, _p, _a, _b);
  return vec2.distance(_p, _near);
}
function pavedEverywhere(site: TerrainSite, x: number, y: number): number {
  vec2.set(_p, x, y);
  let paved = -1e9;
  for (let o = 0; o < site.surfaceStrokes.length; o += site.surfaceStrokeStride)
    paved = Math.max(paved, strokeInside(site.surfaceStrokes, o, x, y));
  if (site.surfaceBoundaries.length === 0) return paved;
  let inside = false,
    nearest = 1e9;
  for (let o = 0; o < site.surfaceTriangles.length; o += site.surfaceTriangleStride) {
    vec2.fromBuffer(_a, site.surfaceTriangles, o);
    vec2.fromBuffer(_b, site.surfaceTriangles, o + 2);
    vec2.fromBuffer(_c, site.surfaceTriangles, o + 4);
    inside ||= triangle2.containsPoint(_a, _b, _c, _p);
  }
  for (let o = 0; o < site.surfaceBoundaries.length; o += site.surfaceBoundaryStride)
    nearest = Math.min(nearest, segmentDistance(site.surfaceBoundaries, o));
  return Math.max(paved, inside ? nearest : -nearest);
}
/** A rect `x, y, w, h` as the GPU table holds it: its far corner rounded to f32. */
function rectsEverywhere(rects: Float32Array, x: number, y: number): number {
  let inside = -1e9;
  for (let o = 0; o < rects.length; o += RECT_FLOATS) {
    const [sx, sy, w, h] = rects.subarray(o, o + RECT_FLOATS);
    inside = Math.max(
      inside,
      Math.min(
        Math.min(x - sx, Math.fround(sx + w) - x),
        Math.min(y - sy, Math.fround(sy + h) - y),
      ),
    );
  }
  return inside;
}
function waterEverywhere(site: TerrainSite, x: number, y: number): number {
  let inside = -1e9;
  for (let o = 0; o < site.rivers.length; o += RIVER_FLOATS)
    inside = Math.max(inside, stretchInside(site.rivers, o, x, y));
  return inside;
}
function forestEverywhere(site: TerrainSite, x: number, y: number): number {
  let forest = rectsEverywhere(site.forests, x, y);
  for (const shape of site.forestShapes)
    if (shape.kind !== "rectangle") forest = Math.max(forest, forestInside(shape, x, y));
  return forest;
}

const FOOTPRINTS = [0, 0.08, 1.9, 2, 2.01, 7, 40, 300, 1e9];

/** Points over the map and past its edge, round its features, and on the
 *  field's cell boundaries. */
function probes(site: TerrainSite, field: SurfaceField, count: number, seed: number): Vec2[] {
  const random = mulberry32.create(seed);
  const next = () => mulberry32.sample(random);
  const [minX, minY, maxX, maxY] = site.map;
  const out: Vec2[] = [];
  for (let i = 0; i < count; i++)
    out.push([minX - 60 + next() * (maxX - minX + 120), minY - 60 + next() * (maxY - minY + 120)]);
  const anchors: number[] = [];
  const take = (records: Float32Array, stride: number, points: number) => {
    for (let o = 0; o < records.length; o += stride)
      for (let k = 0; k < points; k++) anchors.push(records[o + k * 2], records[o + k * 2 + 1]);
  };
  take(site.surfaceStrokes, site.surfaceStrokeStride, 2);
  take(site.surfaceBoundaries, site.surfaceBoundaryStride, 2);
  for (const shape of site.forestShapes) {
    take(shape.strokes, STROKE_FLOATS, 2);
    take(shape.boundaries, 5, 2);
  }
  take(site.rivers, RIVER_FLOATS, 2);
  for (let o = 0; o < site.forests.length; o += RECT_FLOATS)
    anchors.push(
      site.forests[o],
      site.forests[o + 1],
      site.forests[o] + site.forests[o + 2],
      site.forests[o + 1] + site.forests[o + 3],
    );
  for (let i = 0; i < count && anchors.length > 0; i++) {
    const at = Math.floor(next() * (anchors.length / 2)) * 2;
    const spread = next() < 0.5 ? 3 : 30;
    out.push([
      anchors[at] + (next() - 0.5) * 2 * spread,
      anchors[at + 1] + (next() - 0.5) * 2 * spread,
    ]);
  }
  for (let i = 0; i < count / 4; i++) {
    const cx = Math.floor(next() * field.cols),
      cy = Math.floor(next() * field.rows);
    const nudge = (next() - 0.5) * 1e-3;
    out.push([field.origin[0] + cx * field.cellM + nudge, field.origin[1] + cy * field.cellM]);
    out.push([field.origin[0] + cx * field.cellM, field.origin[1] + cy * field.cellM + nudge]);
  }
  return out;
}

/** `found` is `exact` wherever a consumer reading out to `r` can tell: equal
 *  within `r`, and on the same side beyond it. */
function agrees(found: number, exact: number, r: number): boolean {
  if (exact < -r) return found <= exact;
  if (exact > r) return found >= exact;
  return found === exact;
}

function expectFieldMatches(site: TerrainSite, count: number, seed: number) {
  const field = buildSurfaceField(site, reach);
  const wrong: unknown[] = [];
  for (const [x, y] of probes(site, field, count, seed)) {
    const exact = {
      paved: pavedEverywhere(site, x, y),
      forest: forestEverywhere(site, x, y),
      water: waterEverywhere(site, x, y),
    };
    for (const footprint of FOOTPRINTS) {
      // Past the ladder's last level a pixel reads that level's reach.
      const r = reach(Math.min(footprint, field.exactToM));
      const found = {
        paved: pavedDistance(field, x, y, footprint),
        forest: forestDistance(field, x, y, footprint),
        water: waterDistance(field, x, y, footprint),
      };
      for (const rule of ["paved", "forest", "water"] as const)
        if (!agrees(found[rule], exact[rule], r[rule]) && wrong.length < 5)
          wrong.push({ rule, x, y, footprint, found: found[rule], exact: exact[rule] });
    }
  }
  expect(wrong).toEqual([]);
  return field;
}

test("a field lookup is the all-primitives distance wherever a consumer reads it", () => {
  const site = siteOf(CURATED_GROUND);
  // The map holds what it claims to test: every primitive kind is exported.
  expect(site.surfaceStrokes.length).toBeGreaterThan(0);
  expect(site.surfaceTriangles.length).toBeGreaterThan(0);
  expect(site.surfaceBoundaries.length).toBeGreaterThan(0);
  expect(site.forests.length).toBeGreaterThan(0);
  expect(site.forestShapes.map((shape) => shape.kind).sort()).toEqual([
    "polygon",
    "polygon",
    "polygon",
    "rectangle",
    "stroke",
  ]);
  // Two rivers, one with its bend rounded into many stretches.
  expect(site.rivers.length / RIVER_FLOATS).toBeGreaterThan(15);
  // Sparse ground keeps the whole-map level: exact at any pixel width.
  expect(expectFieldMatches(site, 3000, 63).exactToM).toBe(Infinity);
});

test("the shipped maps' fields match their all-primitives distances", () => {
  expect(expectFieldMatches(siteOf(villageMap), 2000, 1).exactToM).toBe(Infinity);
  expect(expectFieldMatches(siteOf(geometry), 2000, 2).exactToM).toBe(Infinity);
});

test("on a dense map a lookup still matches, and visits what is near instead of the map", () => {
  const site = denseGround(1600, 8, 40);
  const segments = site.surfaceStrokes.length / site.surfaceStrokeStride;
  expect(segments).toBeGreaterThanOrEqual(2000);
  const field = expectFieldMatches(site, 1500, 5);
  // What the old loops visited at every pixel: every record.
  const records = field.records.length / 8;
  const random = mulberry32.create(11);
  const cell: [number, number, number, number] = [0, 0, 0, 0];
  const samples = 20000;
  /** The mean records a lookup visits at a pixel `footprint` wide. */
  const visited = (footprint: number) => {
    let sum = 0;
    for (let i = 0; i < samples; i++) {
      const x = mulberry32.sample(random) * 1600,
        y = mulberry32.sample(random) * 1600;
      surfaceCell(cell, field, x, y, footprint);
      sum += cell[3] - cell[0];
    }
    return sum / samples;
  };
  expect(records).toBeGreaterThan(3000);
  // At the play camera, a handful.
  expect(visited(0.08)).toBeLessThan(records / 1000);
  // However wide the pixel, never the map: the ladder stopped at its budget.
  // The budget is a mean over a level's cells and this a mean over points,
  // which weigh the border's part cells differently: hence the half over.
  expect(field.exactToM).toBeLessThan(Infinity);
  expect(visited(1e9)).toBeLessThan(records / 20);
  for (const [footprint, level] of [
    [8, 2],
    [1e9, field.levels - 1],
  ])
    expect(visited(footprint)).toBeLessThan(surfaceListBudget(level) * 1.5);
});

test("a stroke through its cells' corners is found in every cell it crosses", () => {
  // The diagonal of the grid: it enters and leaves each cell at a corner.
  const strip: ForestShape = {
    canopy: 12,
    trunkRange: [0, 0],
    kind: "stroke",
    // Stretches from the middle of their strokes: neither end is cut.
    strokes: Float32Array.of(96, 16, 160, 80, 9, 0, 0),
    triangles: new Float32Array(0),
    boundaries: new Float32Array(0),
  };
  const site: TerrainSite = {
    ...denseGround(256, 0, 0),
    surfaceStrokes: Float32Array.of(8, 8, 72, 72, 3, 1, 0),
    forests: new Float32Array(0),
    forestShapes: [strip],
    // A river along the other diagonal, widening as it goes.
    rivers: Float32Array.of(24, 200, 88, 136, 6, 10, 0.25, 0.25, 1, 1, -1),
  };
  const field = buildSurfaceField(site, reach);
  for (let k = 0.5; k < 64; k += 1) {
    expect(pavedDistance(field, 8 + k, 8 + k, 0.08)).toBe(3);
    expect(forestDistance(field, 96 + k, 16 + k, 0.08)).toBe(9);
    expect(waterDistance(field, 24 + k, 200 - k, 0.08)).toBeCloseTo(6 + (4 * k) / 64, 5);
  }
});

test("a stretch is cut square at the end its stroke ends at, and round at the other", () => {
  // East from (10, 20) to (50, 20), 3 m either side.
  const stretch = (cuts: number) => Float32Array.of(10, 20, 50, 20, 3, 1, cuts);
  const whole = stretch(3);
  // Along the middle: as deep as the edge is near, the end's as well as the sides'.
  expect(strokeInside(whole, 0, 30, 20)).toBe(3);
  expect(strokeInside(whole, 0, 11, 20)).toBe(1);
  expect(strokeInside(whole, 0, 49.5, 21)).toBe(0.5);
  // The end face is the edge; past it is outside by the distance to it.
  expect(strokeInside(whole, 0, 10, 20)).toBe(0);
  expect(strokeInside(whole, 0, 9, 20)).toBe(-1);
  expect(strokeInside(whole, 0, 52, 22)).toBe(-2);
  // Off a corner, by the distance to the corner.
  expect(strokeInside(whole, 0, 7, 27)).toBe(-5);
  // Beside the end, outside its width.
  expect(strokeInside(whole, 0, 10, 23.5)).toBe(-0.5);
  // A stretch from the middle of its stroke keeps its round ends: the next
  // stretch's ground.
  expect(strokeInside(stretch(0), 0, 9, 20)).toBe(2);
  expect(strokeInside(stretch(2), 0, 9, 20)).toBe(2);
  expect(strokeInside(stretch(2), 0, 51, 20)).toBe(-1);
  expect(strokeInside(stretch(1), 0, 51, 20)).toBe(2);
});

test("a paved stretch's record says how far along its stroke it starts", () => {
  // Two roads: one with a bend the simulation rounds into short stretches,
  // one straight.
  const bent = [
    [20, 40],
    [160, 40],
    [220, 110],
  ];
  const site = siteOf({
    ...CURATED_GROUND,
    forests: [],
    rivers: [],
    surfaces: [
      { kind: "road", shape: { kind: "stroke", points: bent, width_m: 8 } },
      {
        kind: "country_road",
        shape: {
          kind: "stroke",
          points: [
            [300, 300],
            [400, 300],
          ],
          width_m: 8,
        },
      },
    ],
  });
  const { records } = buildSurfaceField(site, reach);
  const stretches = site.surfaceStrokes.length / STROKE_FLOATS;
  expect(stretches).toBeGreaterThan(3);
  const ends: number[] = [];
  let length = 0;
  for (let k = 0; k < stretches; k++) {
    const o = k * SURFACE_FLOATS;
    const starts = records[o + STROKE_CUTS] & CUT_A;
    // Each stroke starts at 0 where it is cut, and runs on from there.
    if (starts) length = 0;
    expect(records[o + SURFACE_STROKE_ALONG]).toBeCloseTo(length, 3);
    length += Math.hypot(records[o + 2] - records[o], records[o + 3] - records[o + 1]);
    if (records[o + STROKE_CUTS] & CUT_B) ends.push(length);
  }
  // Each stroke ends at its own length: the bent one's is its corner's,
  // give or take the rounding.
  const span = (a: number[], b: number[]) => Math.hypot(b[0] - a[0], b[1] - a[1]);
  expect(ends).toHaveLength(2);
  expect(Math.abs(ends[0] - span(bent[0], bent[1]) - span(bent[1], bent[2]))).toBeLessThan(2);
  expect(ends[1]).toBeCloseTo(100, 3);
});

test("the field is the same bytes for the same ground", () => {
  const first = buildSurfaceField(denseGround(1600, 8, 40), reach);
  const again = buildSurfaceField(denseGround(1600, 8, 40), reach);
  expect(again.index).toEqual(first.index);
  expect(again.records).toEqual(first.records);
});

test("the field's memory is bounded by the cell budget, not the map's area", () => {
  const empty = (sizeM: number) => buildSurfaceField(denseGround(sizeM, 0, 0), reach);
  const town = empty(1600),
    large = empty(10000),
    huge = empty(40000);
  // Six times the side is 39 times the cells at one cell size: the cell grows instead.
  expect(large.cellM).toBeGreaterThan(town.cellM);
  for (const field of [town, large, huge])
    expect(field.cols * field.rows).toBeLessThanOrEqual(SURFACE_MAX_CELLS);
  // Sixteen times the area again costs under twice the index (more levels share the finest cell).
  expect(huge.index.byteLength).toBeLessThan(large.index.byteLength * 2);
});

test("every triangle of a paved area lays its slabs square to the area's longest side", () => {
  // A 40 by 10 m yard turned 30 degrees, as two triangles, and a square
  // apart from it turned 70 degrees.
  const turned = (turn: number, [w, h]: number[], [cx, cy]: number[]) =>
    [
      [0, 0],
      [w, 0],
      [w, h],
      [0, h],
    ].map(([x, y]) => [
      cx + x * Math.cos(turn) - y * Math.sin(turn),
      cy + x * Math.sin(turn) + y * Math.cos(turn),
    ]);
  const fan = (ring: number[][]) => [
    [...ring[0], ...ring[1], ...ring[2], 3],
    [...ring[0], ...ring[2], ...ring[3], 3],
  ];
  const deg = Math.PI / 180;
  const triangles = Float32Array.from(
    [
      ...fan(turned(30 * deg, [40, 10], [100, 100])),
      ...fan(turned(70 * deg, [8, 8], [300, 300])),
    ].flat(),
  );
  const bearings = Array.from(areaBearings(triangles, 7));
  // The yard's long side, 30 degrees; the square's, 70 degrees within a
  // quarter turn: 70 - 90 = -20, which is 70 again mod 90.
  expect(bearings[0]).toBeCloseTo(30 * deg, 4);
  expect(bearings[1]).toBeCloseTo(30 * deg, 4);
  expect(bearings[2]).toBeCloseTo(bearings[3], 6);
  expect(bearings[2]).toBeCloseTo(70 * deg, 4);
});

test("an area's triangles, and paving laid over it, share the area's one grid", () => {
  const deg = Math.PI / 180;
  // A fan of a pentagon court whose longest side, 50 m along 10 degrees, is
  // an outer edge of its first triangle only; the second triangle's own
  // longest outer edge runs at 100 degrees mod 90, so 10 + 25.
  const at = (bearing: number, length: number, [x, y]: number[]) => [
    x + length * Math.cos(bearing * deg),
    y + length * Math.sin(bearing * deg),
  ];
  const p0 = [0, 0];
  const p1 = at(10, 50, p0);
  const p2 = at(100, 12, p1);
  const p3 = at(160, 30, p2);
  const p4 = at(215, 12, p3);
  const court = [
    [...p0, ...p1, ...p2, 3],
    [...p0, ...p2, ...p3, 3],
    [...p0, ...p3, ...p4, 3],
  ];
  // An apron inside it, turned 25 degrees, sharing no corner with it.
  const c = [20, 10];
  const apron = [[...c, ...at(25, 8, c), ...at(25 + 90, 6, at(25, 8, c)), 3]];
  const bearings = Array.from(areaBearings(Float32Array.from([...court, ...apron].flat()), 7));
  for (const b of bearings) expect(b).toBeCloseTo(10 * deg, 4);
});
