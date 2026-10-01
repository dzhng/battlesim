// @vitest-environment node
// The surface field against the all-primitives distance it replaces: a lookup
// through the bucket index is the same signed distance wherever a consumer
// reads it, on the simulation's own exports and on a map dense enough that
// the index matters.
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
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
import { RIVER_FLOATS, stretchInside } from "@packages/battle-renderer/src/terrain/rivers";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import {
  buildSurfaceField,
  forestDistance,
  pavedDistance,
  surfaceListBudget,
  SURFACE_MAX_CELLS,
  surfaceCell,
  waterDistance,
  type SurfaceField,
  type SurfaceReach,
} from "@packages/battle-renderer/src/terrain/surfaceField.ts";
import { CURATED_GROUND, denseGround } from "./surfaceGrounds";
import summer from "@fixtures/biomes/summer.json";
import game from "@fixtures/game.json";
import geometry from "@fixtures/geometry-lab.json";

const biome = validateBiome(summer as unknown as Biome);
let layout: WorldLayout;
const views: WorldView[] = [];
afterEach(() => {
  for (const view of views.splice(0)) view.free();
});
beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  layout = JSON.parse(world_layout(JSON.stringify(GAME_RULES))) as WorldLayout;
});

function siteOf(map: unknown): TerrainSite {
  const view = new WorldView(JSON.stringify(map), JSON.stringify(GAME_RULES));
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
    paved = Math.max(paved, site.surfaceStrokes[o + 4] - segmentDistance(site.surfaceStrokes, o));
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
    take(shape.strokes, 6, 2);
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
  expect(expectFieldMatches(siteOf(game.map), 2000, 1).exactToM).toBe(Infinity);
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
    strokes: Float32Array.of(96, 16, 160, 80, 9, 0),
    triangles: new Float32Array(0),
    boundaries: new Float32Array(0),
  };
  const site: TerrainSite = {
    ...denseGround(256, 0, 0),
    surfaceStrokes: Float32Array.of(8, 8, 72, 72, 3, 1),
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
