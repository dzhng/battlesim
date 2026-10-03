// @vitest-environment node
// The terrain surface against the simulation's own geometry: the drawn ground
// is WorldView's ground (heights and normals, at triangle edges and where
// props stand), and the material's road, forest and water masks are the
// simulation's surface rules.
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { readFileSync } from "node:fs";
import { afterEach, beforeAll, expect, test } from "vitest";
import { polygon2 } from "math/shapes";
import * as generator from "@wasm/game_wasm.js";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import generated from "@fixtures/generated-battle.json";
import { generationRequest } from "@web/maps/source";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh.ts";
import {
  buildTerrainSurface,
  RECT_FLOATS,
  type TerrainSurface,
} from "@packages/battle-renderer/src/terrain/terrainSurface.ts";
import { forestInside } from "@packages/battle-renderer/src/terrain/forestShapes";
import { plotGuideEdges } from "@packages/battle-renderer/src/terrain/surfaces";
import {
  CUT_A,
  CUT_B,
  STROKE_CUTS,
  strokeInside,
} from "@packages/battle-renderer/src/terrain/strokes";
import { SURFACE_AREA_KINDS } from "@packages/battle-renderer/src/terrain/surfaces";
import { generatePlots, plotAt } from "@packages/battle-renderer/src/terrain/plots.ts";
import {
  PLOT_HUE_JITTER,
  PLOT_MIN_LSTAR,
  validateBiome,
  type Biome,
} from "@packages/battle-renderer/src/terrain/biome.ts";
import summer from "@fixtures/biomes/summer.json";
import { loadMap } from "@web/maps/node";
import { groundHeight } from "@packages/battle-renderer/src/terrain/terrainGrid";
import { packTerrainHeights } from "@packages/battle-renderer/src/frame/terrainHeights";
import {
  groundReach,
  roadLooks,
  roadOrder,
} from "@packages/battle-renderer/src/frame/terrainMaterial";

const geometry = loadMap("geometry").definition;
const riverLab = loadMap("river").definition;
const villageMap = loadMap("village").definition;

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

function world(map: unknown): { view: WorldView; exports: WorldExports } {
  const view = new WorldView(JSON.stringify(map), JSON.stringify(GAME_RULES));
  views.push(view);
  return {
    view,
    exports: readWorldExports(view),
  };
}

test("visual surroundings extend rendered fields without enlarging physical terrain or picking", () => {
  const original = world(geometry);
  const surrounded = world({ ...geometry, render_margin_m: 500 });
  const surface = buildTerrainSurface(surrounded.exports, layout, biome);
  expect(surface.plots.region).toEqual([
    -500,
    -500,
    geometry.size[0] + 500,
    geometry.size[1] + 500,
  ]);
  expect(surrounded.exports.extents).toEqual({
    playable: [0, 0, ...geometry.size],
    physical: [0, 0, ...geometry.size],
    rendered: surface.plots.region,
  });
  expect(surrounded.exports.positions).toEqual(original.exports.positions);
  expect(surrounded.exports.indices).toEqual(original.exports.indices);
  expect(surrounded.exports.terrain).toEqual(original.exports.terrain);
  expect(surrounded.exports.props).toEqual(original.exports.props);
  expect(surrounded.view.height_at(-1, 100)).toEqual(original.view.height_at(-1, 100));
  expect(surrounded.view.raycast(-1, 100, 100, 0, 0, -1, 200)).toEqual(
    original.view.raycast(-1, 100, 100, 0, 0, -1, 200),
  );
});

test("an authored arena retains its physical footprint when its last height cell is rounded", () => {
  const { view, exports } = world({
    size: [12, 10],
    height_grid_m: 4,
    fog_cell_m: 4,
    slope_cutoff_deg: 35,
  });
  const surface = buildTerrainSurface(exports, layout, {
    ...biome,
    field_rules: { ...biome.field_rules, extent_m: 8 },
  });
  expect(exports.extents).toEqual({
    playable: [0, 0, 12, 10],
    physical: [0, 0, 12, 12],
    rendered: [0, 0, 12, 12],
  });
  expect(view.height_at(0, 12)).toBe(0);
  expect(view.height_at(0, 12.01)).toBeUndefined();
  expect(surface.site.map).toEqual([0, 0, 12, 12]);
  expect(surface.plots.region).toEqual([-8, -8, 20, 20]);
});

/** Every drawn triangle containing (x, y), with its height there and its normal. */
function drawnAt(surface: TerrainSurface, x: number, y: number) {
  const m = surface.mesh;
  const out: { z: number; normal: number[] }[] = [];
  for (let t = 0; t < m.length; t += 3 * VERTEX_FLOATS) {
    const v = (k: number) => t + k * VERTEX_FLOATS;
    const [ax, ay, bx, by, cx, cy] = [
      m[v(0)],
      m[v(0) + 1],
      m[v(1)],
      m[v(1) + 1],
      m[v(2)],
      m[v(2) + 1],
    ];
    if (Math.max(ax, bx, cx) < x || Math.min(ax, bx, cx) > x) continue;
    if (Math.max(ay, by, cy) < y || Math.min(ay, by, cy) > y) continue;
    const det = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy);
    const wa = ((by - cy) * (x - cx) + (cx - bx) * (y - cy)) / det;
    const wb = ((cy - ay) * (x - cx) + (ax - cx) * (y - cy)) / det;
    const wc = 1 - wa - wb;
    if (Math.min(wa, wb, wc) < -1e-9) continue;
    out.push({
      z: wa * m[v(0) + 2] + wb * m[v(1) + 2] + wc * m[v(2) + 2],
      normal: [m[v(0) + 3], m[v(0) + 4], m[v(0) + 5]],
    });
  }
  return out;
}

function expectMatchesWorldView(view: WorldView, surface: TerrainSurface, x: number, y: number) {
  const record = view.surface_at(x, y);
  const [z, nx, ny, nz] = record;
  const drawn = drawnAt(surface, x, y);
  expect(drawn.length, `drawn triangles at (${x}, ${y})`).toBeGreaterThan(0);
  // Every triangle meeting the point agrees on its height (no seam, no
  // resampling), and one of them is the simulation's, normal and all.
  for (const d of drawn)
    expect(d.z, `height at (${x}, ${y})`).toBeCloseTo(view.height_at(x, y)!, 3);
  expect(
    drawn.some((d) => Math.hypot(d.normal[0] - nx, d.normal[1] - ny, d.normal[2] - nz) < 1e-4),
    `normal at (${x}, ${y}): ${JSON.stringify({ drawn, nx, ny, nz })}`,
  ).toBe(true);
  return z;
}

test("heights and normals at triangle edges are WorldView's", () => {
  // The geometry lab: a ridge and two mesas, so edges carry real slope breaks.
  const { view, exports } = world(geometry);
  const surface = buildTerrainSurface(exports, layout, biome);
  const spacing = geometry.height_grid_m;
  for (const [i, j] of [
    [20, 48],
    [24, 52],
    [58, 12],
    [60, 14],
    [81, 11],
    [33, 60],
  ]) {
    const [x0, y0] = [i * spacing, j * spacing];
    // A cell's south edge, west edge and its diagonal, at their midpoints.
    expectMatchesWorldView(view, surface, x0 + spacing / 2, y0);
    expectMatchesWorldView(view, surface, x0, y0 + spacing / 2);
    expectMatchesWorldView(view, surface, x0 + spacing / 2, y0 + spacing / 2);
  }
});

test("sampled pages preserve terrain across joins and partial edge pages", () => {
  const { view, exports } = world({
    size: [132, 140],
    fog_cell_m: 8,
    height_grid_m: 4,
    slope_cutoff_deg: 35,
    relief: [{ kind: "ridge", center: [64, 64], radius_m: 28, peak_m: 12 }],
    rivers: [
      {
        points: [
          { xy: [116, 100], width_m: 32, depth_m: 4 },
          { xy: [116, 140], width_m: 32, depth_m: 4 },
        ],
        surface_z: 0,
      },
    ],
  });
  const grid = exports.terrain;
  for (const x of [0, 31.3, 60, 63.9, 64, 64.1, 92, 96, 127.9, 128, 132]) {
    for (const y of [0, 33.7, 60, 63.9, 64, 64.1, 92, 127.9, 128, 140])
      expect(groundHeight(grid, x, y), `${x},${y}`).toBeCloseTo(view.height_at(x, y)!, 5);
  }
  expect(grid.minHeight).toBe(-4);
  const packed = packTerrainHeights(grid);
  const size = packed[0],
    cols = packed[1],
    count = packed[2];
  const f32 = new Float32Array(packed.buffer);
  for (let j = 0; j < grid.ny; j++)
    for (let i = 0; i < grid.nx; i++) {
      const page = packed[3 + Math.floor(j / size) * cols + Math.floor(i / size)];
      const h = page
        ? f32[3 + count + (page - 1) * size * size + (j % size) * size + (i % size)]
        : 0;
      expect(h).toBeCloseTo(view.height_at(i * grid.spacing, j * grid.spacing)!, 5);
    }
});

test("empty full-extent height uploads stay independent of area", () => {
  for (const size of [12_000, 15_000, 18_000]) {
    const grid = {
      nx: size / 4 + 1,
      ny: size / 4 + 1,
      spacing: 4,
      pageSize: 16,
      minHeight: 0,
      pageIds: new Uint32Array(0),
      heights: new Float32Array(0),
    };
    expect(packTerrainHeights(grid).byteLength).toBeLessThan(1024);
    expect(groundHeight(grid, size, size)).toBe(0);
  }
});

test("heights and normals where props stand are WorldView's", () => {
  const { view, exports } = world(villageMap);
  const surface = buildTerrainSurface(exports, layout, biome);
  const at = (f: string) => layout.propFields.indexOf(f);
  let checked = 0;
  for (let o = 0; o < exports.props.length; o += layout.propStride) {
    const kind = layout.propKinds[exports.props[o + at("kind")]];
    if (kind !== "building") continue;
    const [x, y, hx, hy] = ["x", "y", "hx", "hy"].map((f) => exports.props[o + at(f)]);
    // The building's footprint corners and centre: where it meets the ground.
    for (const [sx, sy] of [
      [-1, -1],
      [1, -1],
      [1, 1],
      [-1, 1],
      [0, 0],
    ]) {
      expectMatchesWorldView(view, surface, x + sx * hx, y + sy * hy);
      checked++;
    }
  }
  expect(checked).toBeGreaterThan(0);
});

test("the material's road, forest and water masks are the simulation's surface rules", () => {
  // How far inside the water's edge: the half width at the closest point of
  // a stretch less the distance to it, the deepest over stretches.
  const waterInside = (rivers: Float32Array, x: number, y: number) => {
    let inside = -Infinity;
    for (let r = 0; r < rivers.length; r += layout.riverStride) {
      const [ax, ay, bx, by, halfA, halfB] = rivers.subarray(r, r + 6);
      const t = Math.min(
        1,
        Math.max(
          0,
          ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / ((bx - ax) ** 2 + (by - ay) ** 2),
        ),
      );
      const half = halfA + (halfB - halfA) * t;
      inside = Math.max(inside, half - Math.hypot(x - ax - (bx - ax) * t, y - ay - (by - ay) * t));
    }
    return inside;
  };
  let wet = 0;
  // Samples round stroke ends that lie on a map (a road that runs off the
  // map's edge has none to show).
  let ends = 0;
  for (const map of [geometry, villageMap, riverLab]) {
    const { view, exports } = world(map);
    const { site } = buildTerrainSurface(exports, layout, biome);
    const inRect = (rects: Float32Array, x: number, y: number) => {
      for (let r = 0; r < rects.length; r += RECT_FLOATS) {
        const [rx, ry, w, h] = rects.subarray(r, r + RECT_FLOATS);
        if (x >= rx && x <= rx + w && y >= ry && y <= ry + h) return true;
      }
      return false;
    };
    const strokes = site.surfaceStrokes;
    // How far inside the paving: the renderer's own reading of the exported
    // stretches, which end square where their stroke does.
    const roadInside = (x: number, y: number) => {
      let inside = -Infinity;
      for (let r = 0; r < strokes.length; r += site.surfaceStrokeStride) {
        const ax = strokes[r],
          ay = strokes[r + 1];
        const dx = strokes[r + 2] - ax,
          dy = strokes[r + 3] - ay;
        // Skip a segment whose box the point is outside: rounded bends add many.
        const half = strokes[r + 4];
        if (Math.abs(x - ax - dx / 2) > Math.abs(dx) / 2 + half) continue;
        if (Math.abs(y - ay - dy / 2) > Math.abs(dy) / 2 + half) continue;
        inside = Math.max(inside, strokeInside(strokes, r, x, y));
      }
      return inside;
    };
    // The same, stopping at the first stretch that holds the point.
    const onRoad = (x: number, y: number) => {
      for (let r = 0; r < strokes.length; r += site.surfaceStrokeStride) {
        const ax = strokes[r],
          ay = strokes[r + 1];
        const dx = strokes[r + 2] - ax,
          dy = strokes[r + 3] - ay;
        const half = strokes[r + 4];
        if (Math.abs(x - ax - dx / 2) > Math.abs(dx) / 2 + half) continue;
        if (Math.abs(y - ay - dy / 2) > Math.abs(dy) / 2 + half) continue;
        if (strokeInside(strokes, r, x, y) >= 0) return true;
      }
      return false;
    };
    const [width, depth] = map.size;
    const wrong: string[] = [];
    let roads = 0;
    // Round every end of every stroke, where a round cap would differ from
    // the square end: past the end, beside it, and off its two corners.
    for (let r = 0; r < strokes.length; r += site.surfaceStrokeStride) {
      const cuts = strokes[r + STROKE_CUTS];
      for (const [bit, from, to] of [
        [CUT_A, r, r + 2],
        [CUT_B, r + 2, r],
      ]) {
        if (!(cuts & bit)) continue;
        const half = strokes[r + 4];
        const run = Math.hypot(strokes[from] - strokes[to], strokes[from + 1] - strokes[to + 1]);
        const out = [
          (strokes[from] - strokes[to]) / run,
          (strokes[from + 1] - strokes[to + 1]) / run,
        ];
        for (const past of [-0.4, 0.3, 0.6 * half, 0.95 * half])
          for (const aside of [-0.9, -0.5, 0, 0.5, 0.9, 1.1]) {
            const x = strokes[from] + out[0] * past - out[1] * aside * half,
              y = strokes[from + 1] + out[1] * past + out[0] * aside * half;
            if (x <= 0 || y <= 0 || x >= width || y >= depth) continue;
            const [, , , , , kind] = view.surface_at(x, y);
            const surfaceKind = layout.surfaceKinds[kind];
            if (surfaceKind === "bridge" || surfaceKind === "water") continue;
            // The exported stretches are f32: within a millimetre of the edge
            // a point may fall either side of the simulation's f64 edge.
            if (Math.abs(roadInside(x, y)) < 1e-3) continue;
            ends++;
            if (onRoad(x, y) !== (surfaceKind === "road"))
              wrong.push(`road end at (${x}, ${y}): ${past} past, ${aside} aside`);
          }
      }
    }
    for (let y = 0.37; y < depth; y += 2.3) {
      for (let x = 0.61; x < width; x += 2.3) {
        const [, , , , , kind, forest] = view.surface_at(x, y);
        const surfaceKind = layout.surfaceKinds[kind];
        // A bridge deck hides the ground kind beneath it (road or river).
        if (surfaceKind === "bridge") continue;
        // The exported stretches are f32: a point within a millimetre of
        // the edge may fall either side of the simulation's f64 edge.
        const inside = waterInside(site.rivers, x, y);
        const water = inside >= 0;
        if (Math.abs(inside) > 1e-3 && water !== (surfaceKind === "water"))
          wrong.push(`water at (${x}, ${y})`);
        if (water) wet++;
        if (!water && onRoad(x, y) !== (surfaceKind === "road")) wrong.push(`road at (${x}, ${y})`);
        // Water is neither road nor forest, whatever is authored over it.
        if (!water && inRect(site.forests, x, y) !== (forest === 1))
          wrong.push(`forest at (${x}, ${y})`);
        if (surfaceKind === "road") roads++;
      }
    }
    expect(wrong).toEqual([]);
    expect(roads).toBeGreaterThan(100);
  }
  expect(wet).toBeGreaterThan(2000);
  expect(ends).toBeGreaterThan(100);
});

test("each paved stretch names its own area's kind, so a track is drawn as a track", () => {
  // The river lab holds a 9 m country road and a 4 m dirt track.
  const { site } = buildTerrainSurface(world(riverLab).exports, layout, biome);
  const kindAt = layout.surfaceStrokeFields.indexOf("kind");
  const widths = new Map<string, Set<number>>();
  for (let r = 0; r < site.surfaceStrokes.length; r += site.surfaceStrokeStride) {
    const kind = SURFACE_AREA_KINDS[site.surfaceStrokes[r + kindAt]];
    widths.set(kind, (widths.get(kind) ?? new Set()).add(site.surfaceStrokes[r + 4] * 2));
  }
  expect(Object.fromEntries(widths)).toEqual({
    country_road: new Set([9]),
    dirt_track: new Set([4]),
  });
});

test("rounded strokes are the native samples, bit for bit", () => {
  const oracle = JSON.parse(
    readFileSync(
      new URL("../../fixtures/parity/ground/curve-strokes.json", import.meta.url),
      "utf8",
    ),
  ) as { map: unknown; strokes: string[]; rivers: string[] };
  const { exports } = world(oracle.map);
  const hex = (floats: Float32Array) =>
    Array.from(new Uint32Array(floats.buffer, floats.byteOffset, floats.length), (v) =>
      v.toString(16).padStart(8, "0"),
    );
  expect(hex(exports.surfaceStrokes)).toEqual(oracle.strokes);
  // A river's rounded stretches, with the width and grade at each end (C69).
  expect(oracle.rivers.length).toBeGreaterThan(40 * layout.riverStride);
  expect(hex(exports.rivers)).toEqual(oracle.rivers);
});

test("roads split the patchwork: fields meet a road edge-on, never across it", () => {
  const { exports } = world(villageMap);
  const { site, plots } = buildTerrainSurface(exports, layout, biome);
  for (let r = 0; r < site.surfaceStrokes.length; r += site.surfaceStrokeStride) {
    const [ax, ay, bx, by, half] = site.surfaceStrokes.subarray(r, r + 5);
    const len = Math.hypot(bx - ax, by - ay);
    const [nx, ny] = [-(by - ay) / len, (bx - ax) / len];
    // Along the segment, clear of its ends (where roads meet and turn).
    for (let s = 20; s < len - 20; s += 7) {
      const [px, py] = [ax + ((bx - ax) * s) / len, ay + ((by - ay) * s) / len];
      const left = plotAt(plots, px + nx * half, py + ny * half);
      const right = plotAt(plots, px - nx * half, py - ny * half);
      expect(left!.plot, `plots either side at (${px}, ${py})`).not.toBe(right!.plot);
    }
  }
});

/** How far `across` is turned from `heading` (radians), as rows read it: a
 *  quarter turn is the same grain. */
function offGrain(across: readonly number[], heading: number): number {
  const turn = Math.atan2(across[1], across[0]) - heading;
  return Math.abs(Math.asin(Math.sin(2 * turn))) / 2;
}

test("open country keeps one grain: no tract turns further than the rules say, however large the land", () => {
  // 8 km of land with no road: turns must not add up from the region down
  // to the plots, or the patchwork fans out round the map's middle.
  const bare = {
    map: [0, 0, 8000, 8000] as const,
    buildings: [],
    surfaceStrokes: new Float32Array(),
    surfaceStrokeStride: 1,
    surfaceRuns: new Float32Array(),
    surfaceRunStride: 4,
    surfaceTriangles: new Float32Array(),
    surfaceTriangleStride: 1,
    surfaceBoundaries: new Float32Array(),
    surfaceBoundaryStride: 5,
    riverRuns: new Float32Array(),
    riverRunStride: 4,
    forestShapes: [],
  };
  const rules = biome.field_rules;
  const most = ((rules.orientation_jitter_deg + rules.cut_jitter_deg) * Math.PI) / 180;
  const heading = (rules.orientation_deg * Math.PI) / 180;
  const { plots } = generatePlots(bare, biome);
  expect(plots.length).toBeGreaterThan(5000);
  const turned = plots.map((p) => offGrain(p.across, heading));
  expect(Math.max(...turned)).toBeLessThanOrEqual(most + 1e-6);
  // And tracts do turn: the land is not one ruled grid.
  expect(turned.filter((t) => t > most / 3).length).toBeGreaterThan(plots.length / 10);
});

test("a field at a road's edge lies along a road beside it: its rows run with it or square to it", () => {
  const { exports } = world(villageMap);
  const { site, plots } = buildTerrainSurface(exports, layout, biome);
  const roads = plotGuideEdges(site);
  /** Whether road `r` runs through `outline` or beside it, nearer than a plot is wide. */
  const borders = (outline: number[], r: number) => {
    const [ax, ay, bx, by] = roads.subarray(r, r + 4);
    const steps = Math.ceil(Math.hypot(bx - ax, by - ay) / 5);
    for (let s = 0; s <= steps; s++) {
      const p: [number, number] = [ax + ((bx - ax) * s) / steps, ay + ((by - ay) * s) / steps];
      const n = outline.length / 2;
      if (polygon2.containsPoint(outline, n, p)) return true;
      if (Math.abs(polygon2.signedDistance(outline, n, p)) < biome.field_rules.min_width_m)
        return true;
    }
    return false;
  };
  let beside = 0;
  for (let r = 0; r < site.surfaceStrokes.length; r += site.surfaceStrokeStride) {
    const [ax, ay, bx, by, half] = site.surfaceStrokes.subarray(r, r + 5);
    const len = Math.hypot(bx - ax, by - ay);
    const [nx, ny] = [-(by - ay) / len, (bx - ax) / len];
    // Along the stretch, clear of its ends, a metre off the paving either side.
    for (let s = 20; s < len - 20; s += 7)
      for (const side of [-half - 1, half + 1]) {
        const px = ax + ((bx - ax) * s) / len + nx * side;
        const py = ay + ((by - ay) * s) / len + ny * side;
        const plot = plots.plots[plotAt(plots, px, py)!.plot];
        let least = Infinity;
        for (let o = 0; o < roads.length; o += 4)
          if (borders(plot.outline, o))
            least = Math.min(
              least,
              offGrain(
                plot.across,
                Math.atan2(roads[o + 3] - roads[o + 1], roads[o + 2] - roads[o]),
              ),
            );
        expect(least, `the plot at (${px}, ${py})`).toBeLessThan(0.1);
        beside++;
      }
  }
  expect(beside).toBeGreaterThan(100);
});

test("each point lies in the plot the split walks to, and its edge distance is that plot's", () => {
  const { exports } = world(villageMap);
  const { plots } = buildTerrainSurface(exports, layout, biome);
  const [x0, y0, x1, y1] = plots.region;
  for (let k = 0; k < 400; k++) {
    // A fixed low-discrepancy scatter over the whole region.
    const x = x0 + (x1 - x0) * ((k * 0.618034) % 1);
    const y = y0 + (y1 - y0) * ((k * 0.754877 + 0.5) % 1);
    const found = plotAt(plots, x, y)!;
    const outline = plots.plots[found.plot].outline;
    const n = outline.length / 2;
    expect(polygon2.containsPoint(outline, n, [x, y]), `(${x}, ${y})`).toBe(true);
    const toRegion = Math.min(x - x0, x1 - x, y - y0, y1 - y);
    expect(found.edge).toBeCloseTo(
      Math.min(toRegion, Math.abs(polygon2.signedDistance(outline, n, [x, y]))),
      6,
    );
  }
});

test.each([
  {
    kind: "stroke",
    points: [
      [100, 0],
      [100, 200],
    ],
    width_m: 6,
  },
  {
    kind: "polygon",
    ring: [
      [97, 0],
      [103, 0],
      [103, 200],
      [97, 200],
    ],
  },
])(
  "a building's yard stops at a $kind road, while nearby unbuilt ground stays undrilled",
  (shape) => {
    const { exports } = world({
      size: [200, 200],
      fog_cell_m: 8,
      height_grid_m: 4,
      slope_cutoff_deg: 35,
      surfaces: [{ kind: "country_road", shape }],
    });
    const site = {
      ...buildTerrainSurface(exports, layout, biome).site,
      buildings: [[80, 100]] as [number, number][],
    };
    const local = {
      ...biome,
      field_rules: {
        ...biome.field_rules,
        extent_m: 0,
        size_m: [200, 200] as [number, number],
        yard_m: 100,
        settlement_m: 120,
      },
    };
    const plots = generatePlots(site, local);
    const kindAt = (x: number, y: number) =>
      local.plots[plots.plots[plotAt(plots, x, y)!.plot].kind];
    expect(kindAt(50, 100).name).toBe(local.field_rules.settlement_kind);
    expect(kindAt(150, 100).name).toBe(local.field_rules.surround_kind);
    expect(kindAt(150, 100).furrow_m).toBe(0);
  },
);

test("the ground round a building is the settlement's yard; the land round that is its surround, never a crop", () => {
  // Radial settlement reach, without a road separating house and plot.
  const { exports } = world({ ...villageMap, surfaces: [] });
  const { plots, site } = buildTerrainSurface(exports, layout, biome);
  const rules = biome.field_rules;
  const yard = biome.plots.findIndex((p) => p.name === rules.settlement_kind);
  const surround = biome.plots.findIndex((p) => p.name === rules.surround_kind);
  expect(biome.plots[surround].furrow_m).toBe(0);
  const seen = { yards: 0, round: 0 };
  plots.plots.forEach((plot, k) => {
    const centre = polygon2.centroid([0, 0], plot.outline, plot.outline.length / 2);
    const within = (reach: number) =>
      site.buildings.some((b) => Math.hypot(b[0] - centre[0], b[1] - centre[1]) <= reach);
    if (within(rules.yard_m)) {
      expect(plot.kind, `plot ${k} at the houses`).toBe(yard);
      seen.yards++;
    } else {
      expect(plot.kind, `plot ${k} away from the houses`).not.toBe(yard);
      if (!within(rules.settlement_m)) return;
      expect(plot.kind, `plot ${k} round the houses`).toBe(surround);
      seen.round++;
    }
  });
  expect(seen.yards).toBeGreaterThan(1);
  expect(seen.round).toBeGreaterThan(3);
});

/** A generated map with a town of streets on a country road (the ground
 *  rig's own), and where its generator says its settlements' ground is: how
 *  far inside the nearest settlement's outline a point lies, and inside its
 *  nearest block of buildings (negative outside). The map itself holds
 *  neither. */
function generatedTown() {
  const fixture = (path: string) =>
    readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");
  const documents = {
    presets: fixture("map-presets.json"),
    templates: fixture("prototype-building-templates.json"),
    rules: JSON.stringify(GAME_RULES),
  };
  const request = generationRequest(
    generator,
    { type: "mixed", size: "medium", seed: "2" },
    documents,
    generated.limits,
  );
  type Ring = [number, number][];
  const outcome = JSON.parse(
    generator.generate_map(
      JSON.stringify(request),
      documents.presets,
      documents.templates,
      documents.rules,
    ),
  ) as {
    result: {
      map: unknown;
      sites: { settlements: { outline: Ring; districts: { ring: Ring }[] }[] };
    };
  };
  const { exports } = world(outcome.result.map);
  const { settlements } = outcome.result.sites;
  const inside = (rings: Ring[]) => {
    const flat = rings.map((ring) => ring.flat());
    return (x: number, y: number) => {
      const point: [number, number] = [x, y];
      let deepest = -Infinity;
      for (const ring of flat) {
        deepest = Math.max(deepest, -polygon2.signedDistance(ring, ring.length / 2, point));
      }
      return deepest;
    };
  };
  return {
    surface: buildTerrainSurface(exports, layout, biome),
    inTown: inside(settlements.map((s) => s.outline)),
    inBlock: inside(settlements.flatMap((s) => s.districts.map((d) => d.ring))),
  };
}

test("a generated town's blocks are yards and commons, and the plain beyond keeps its fields", () => {
  const { surface, inTown, inBlock } = generatedTown();
  const { plots } = surface;
  const yard = biome.plots.findIndex((p) => p.name === biome.field_rules.settlement_kind);
  const kindAt = (x: number, y: number) => plots.plots[plotAt(plots, x, y)!.plot].kind;
  const [x0, y0, x1, y1] = surface.site.map;
  const town = { ground: 0, yards: 0, drilled: 0 };
  const plain = { ground: 0, drilled: 0 };
  for (let y = y0 + 10; y < y1; y += 20)
    for (let x = x0 + 10; x < x1; x += 20) {
      const kind = kindAt(x, y);
      const drilled = biome.plots[kind].furrow_m > 0 ? 1 : 0;
      // Inside one of the generator's blocks, clear of the street round it.
      if (inBlock(x, y) > 10) {
        town.ground++;
        town.drilled += drilled;
        if (kind === yard) town.yards++;
      } else if (inTown(x, y) < -300) {
        plain.ground++;
        plain.drilled += drilled;
      }
    }
  // A block is built ground but for an unbuilt margin here and there, which
  // the generator's rings take in and a field may reach into.
  expect(town.ground).toBeGreaterThan(3000);
  expect(town.yards / town.ground).toBeGreaterThan(0.8);
  expect(town.drilled / town.ground).toBeLessThan(0.01);
  // The biome drills about half its plots.
  expect(plain.ground).toBeGreaterThan(50000);
  expect(plain.drilled / plain.ground).toBeGreaterThan(0.4);
});

test("a country road draws a street between actual houses and keeps its paving in unbuilt country", () => {
  const descriptor = JSON.parse(
    readFileSync(
      new URL("../../fixtures/parity/templates/asymmetric.json", import.meta.url),
      "utf8",
    ),
  );
  const template_catalog_hash = JSON.parse(
    generator.template_catalogue_json(JSON.stringify([descriptor])),
  ).hash;
  // Fixed authored inputs isolate this presentation contract from release tuning.
  const local: Biome = {
    ...biome,
    seed: 1616,
    field_rules: {
      ...biome.field_rules,
      extent_m: 0,
      size_m: [20, 20],
      tract_m: 600,
      yard_m: 40,
      settlement_m: 110,
    },
    roads: {
      ...biome.roads,
      country_road: {
        ...biome.roads.country_road,
        town: { kind: "road", beside_m: 5, gap_m: 80 },
      },
    },
  };
  const authoredRoad = (houses: readonly (readonly [number, number])[]) => {
    const buildings = houses.map(([x, y], i) => ({
      owner: i * 2,
      kind: "building",
      category: descriptor.category,
      regional_family: descriptor.regional_family,
      parts: [
        { part: "main", prop: i * 2 },
        { part: "wing", prop: i * 2 + 1 },
      ],
      geometry: JSON.parse(
        generator.materialize_template(
          JSON.stringify(descriptor),
          JSON.stringify({ translation: [x, y, 0], yaw: 0 }),
        ),
      ),
    }));
    const { exports } = world({
      size: [600, 200],
      fog_cell_m: 8,
      height_grid_m: 4,
      slope_cutoff_deg: 35,
      template_catalog_hash,
      buildings,
      surfaces: [
        {
          kind: "country_road",
          shape: {
            kind: "stroke",
            points: [
              [0, 100],
              [600, 100],
            ],
            width_m: 8,
          },
        },
      ],
    });
    return buildTerrainSurface(exports, layout, local);
  };
  const pavingAt = (surface: TerrainSurface, x: number, y: number) => {
    const kinds = new Set<string>();
    for (let o = 0; o < surface.strokes.length; o += surface.site.surfaceStrokeStride)
      if (strokeInside(surface.strokes, o, x, y) >= 0)
        kinds.add(SURFACE_AREA_KINDS[surface.strokes[o + 5]]);
    return [...kinds].sort();
  };
  // Two small houses face each other across an 8 m road. No settlement outline
  // labels this fixture; actual exported buildings supply the composition input.
  const built = authoredRoad([
    [200, 75],
    [200, 125],
  ]);
  const bare = authoredRoad([]);
  const oneSided = authoredRoad([[200, 75]]);
  for (const y of [97, 100, 103]) {
    expect(pavingAt(built, 200, y), `between houses at y=${y}`).toEqual(["road"]);
    expect(pavingAt(built, 500, y), `unbuilt country at y=${y}`).toEqual(["country_road"]);
    expect(pavingAt(bare, 200, y), `same road without houses at y=${y}`).toEqual(["country_road"]);
    expect(pavingAt(oneSided, 200, y), `houses on only one side at y=${y}`).toEqual([
      "country_road",
    ]);
  }
  // Promotion changes presentation only; the authoritative road and its width
  // remain identical to the road without houses.
  expect(built.site.surfaceStrokes).toEqual(bare.site.surfaceStrokes);
  for (const x of [200, 500]) for (const y of [95, 105]) expect(pavingAt(built, x, y)).toEqual([]);
});

test("the village's roads, streets by name and country roads by look, are drawn as exported", () => {
  const { exports } = world(villageMap);
  const surface = buildTerrainSurface(exports, layout, biome);
  expect(surface.strokes).toBe(surface.site.surfaceStrokes);
});

test("the patchwork is the same for the same seed and moves with it", () => {
  const { exports } = world(villageMap);
  const a = buildTerrainSurface(exports, layout, biome).plots;
  const b = buildTerrainSurface(exports, layout, biome).plots;
  const c = buildTerrainSurface(exports, layout, { ...biome, seed: biome.seed + 1 }).plots;
  expect(Array.from(b.nodes)).toEqual(Array.from(a.nodes));
  expect(b.plots.map((p) => p.colour)).toEqual(a.plots.map((p) => p.colour));
  expect(Array.from(c.nodes)).not.toEqual(Array.from(a.nodes));
});

test("a biome that names a missing palette is refused by name", () => {
  const broken = { ...biome, plots: [{ ...biome.plots[0], palette: "nowhere" }] };
  expect(() => validateBiome(broken, "summer")).toThrow(/summer\.plots\[0\]\.palette/);
});

test("every paved kind is drawn by its own road row, or the default's", () => {
  const roads = {
    default: { ...biome.roads.default, palette: "gravel" },
    dirt_track: { ...biome.roads.default, palette: "earth", roughness: 0.5 },
  };
  const palettes = {
    ...biome.palettes,
    gravel: [
      [0.5, 0.5, 0.5],
      [0.9, 0.6, 0.3],
    ],
    earth: [
      [0.5, 0.4, 0.3],
      [0.2, 0.2, 0.2],
    ],
  } as Biome["palettes"];
  const looks = roadLooks(validateBiome({ ...biome, palettes, roads }));
  const look = (kind: (typeof SURFACE_AREA_KINDS)[number]) =>
    looks[SURFACE_AREA_KINDS.indexOf(kind)];
  expect(look("dirt_track").core.w).toBe(0.5);
  expect(look("dirt_track").core.x).toBeCloseTo(0.5 ** 2.2, 6);
  for (const kind of ["road", "country_road", "sidewalk"] as const) {
    expect(look(kind).core, kind).toEqual(look("road").core);
    expect(look(kind).shoulder, kind).toEqual(look("road").shoulder);
  }
  expect(look("road").core).not.toEqual(look("dirt_track").core);
  // A patch is a change of hue alone: as bright as the surface it lies in,
  // whichever of the two colours is the brighter in the palette.
  const luminance = (c: { x: number; y: number; z: number }) =>
    0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
  for (const kind of ["road", "dirt_track"] as const)
    expect(luminance(look(kind).worn), kind).toBeCloseTo(luminance(look(kind).core), 6);

  expect(() => validateBiome({ ...biome, roads: { dirt_track: roads.dirt_track } })).toThrow(
    /roads: needs a default/,
  );
  expect(() =>
    validateBiome({ ...biome, roads: { ...biome.roads, motorway: roads.default } }),
  ).toThrow(/roads\.motorway: names no paved kind/);
});

test("a plot kind whose ground could draw darker than the lightness floor is refused", () => {
  // Seen ground that dark, in a sun shadow, reads as unseen ground.
  const k = biome.plots.findIndex((p) => p.furrow_contrast > 0);
  const kind = biome.plots[k];
  const at = new RegExp(`summer\\.plots\\[${k}\\]\\.palette.*L\\*`);
  const withPlot = (plot: typeof kind, colours: readonly (readonly number[])[]) =>
    ({
      ...biome,
      palettes: { ...biome.palettes, [plot.palette]: colours },
      plots: biome.plots.map((p, i) => (i === k ? plot : p)),
    }) as Biome;
  // A colour whose darkest plot (the per-plot jitter at its lowest) sits on
  // the floor, and the same colour a tenth darker.
  const jitter =
    (1 - biome.field_rules.colour_jitter) * (1 - PLOT_HUE_JITTER * biome.field_rules.colour_jitter);
  const grey = (lstar: number) => {
    const v = (((lstar + 16) / 116) ** 3) ** (1 / 2.2) / jitter;
    return [[v, v, v]];
  };
  const bare = { ...kind, furrow_contrast: 0 };
  expect(() => validateBiome(withPlot(bare, grey(PLOT_MIN_LSTAR + 1)), "summer")).not.toThrow();
  expect(() => validateBiome(withPlot(bare, grey(PLOT_MIN_LSTAR - 1)), "summer")).toThrow(at);
  // Rows darken a plot too: the same passing colour under deep furrows.
  expect(() =>
    validateBiome(withPlot({ ...kind, furrow_contrast: 0.5 }, grey(PLOT_MIN_LSTAR + 1)), "summer"),
  ).toThrow(at);
});

test("wheelings are furrows laid bare: none without rows, none wider than a row", () => {
  const k = biome.plots.findIndex((p) => p.tram.rows > 0);
  const kind = biome.plots[k];
  const refused = (change: Partial<typeof kind>, field: string) =>
    expect(() =>
      validateBiome(
        { ...biome, plots: biome.plots.map((p, i) => (i === k ? { ...p, ...change } : p)) },
        "summer",
      ),
    ).toThrow(new RegExp(`summer\\.plots\\[${k}\\]\\.tram\\.${field}`));
  refused({ furrow_m: 0, furrow_contrast: 0 }, "rows");
  refused({ tram: { ...kind.tram, width_m: kind.furrow_m * 1.5 } }, "width_m");
  // A pair of wheelings needs rows between its tracks and to the next pair.
  refused({ tram: { ...kind.tram, rows: 2 } }, "rows");
});

test("a street's row says what draws its yards and its walk", () => {
  const plain = { ...biome.roads.default, area: undefined, walk: undefined };
  const street = {
    ...plain,
    area: "sidewalk",
    walk: { kind: "sidewalk", width_m: 2, slab_m: 2.5, joint: 0.2 },
  };
  const looks = roadLooks(validateBiome({ ...biome, roads: { default: plain, road: street } }));
  const tag = (kind: (typeof SURFACE_AREA_KINDS)[number]) => SURFACE_AREA_KINDS.indexOf(kind);
  // The street: its areas and its walk go to the sidewalk's row.
  const road = looks[tag("road")];
  expect([road.join.y, road.join.z, road.join.w]).toEqual([tag("sidewalk"), 2, tag("sidewalk")]);
  expect(road.slabs.x).toBeCloseTo(1 / 2.5, 6);
  // Every other kind draws its own areas and has no walk.
  for (const kind of ["country_road", "dirt_track", "sidewalk"] as const)
    expect([looks[tag(kind)].join.y, looks[tag(kind)].join.z], kind).toEqual([tag(kind), 0]);
  // Only a carriageway is carried onto the road it joins.
  expect(looks.map((look) => look.track.w)).toEqual([1, 1, 1, 0]);

  for (const [row, path] of [
    [{ ...street, area: "lawn" }, "area"],
    [{ ...street, walk: { ...street.walk, kind: "lawn" } }, "walk\\.kind"],
  ] as const)
    expect(() => validateBiome({ ...biome, roads: { default: plain, road: row } })).toThrow(
      new RegExp(`roads\\.road\\.${path}: names no paved kind "lawn"`),
    );
});

test("a curb is its row's kerbstones and a face shading never tilts past 40 degrees", () => {
  const palettes = { ...biome.palettes, stone: [[1, 1, 1]] } as Biome["palettes"];
  const curbed = (curb: unknown) =>
    validateBiome({
      ...biome,
      palettes,
      roads: { default: biome.roads.default, road: { ...biome.roads.default, curb } },
    } as Biome);
  const stones = { palette: "stone", width_m: 0.25, stone_m: 0.5, joint: 0.25, face_m: 0.125 };
  const [road, country] = roadLooks(curbed({ ...stones, tilt_deg: 45 / 2 }));
  expect([road.curb.x, road.curb.w, road.slabs.z]).toEqual([1, 0.25, 0.125]);
  expect([road.stones.x, road.stones.y]).toEqual([2, 0.25]);
  expect(road.slabs.w).toBeCloseTo(Math.SQRT2 - 1, 6);
  // A road without one has no stones and no face.
  expect([country.curb.w, country.stones.x, country.slabs.z, country.slabs.w]).toEqual([
    0, 0, 0, 0,
  ]);
  expect(() => curbed({ ...stones, tilt_deg: 41 })).toThrow(/roads\.road\.curb\.tilt_deg/);
  expect(() => curbed({ ...stones, palette: "granite", tilt_deg: 30 })).toThrow(
    /roads\.road\.curb\.palette: names no palette "granite"/,
  );
});

test("a road's markings are its row's, and the field is read as far as a crossing's bars lie", () => {
  const palettes = { ...biome.palettes, white: [[1, 1, 1]] } as Biome["palettes"];
  const markings = {
    palette: "white",
    cover: 0.75,
    wear: 0.25,
    line_m: 0.25,
    dash_m: [2, 4],
    crossing: { bar_m: 0.5, length_m: 4, gap_m: 1.5, inset_m: 0.25 },
  };
  const plain = { ...biome.roads.default, markings: undefined };
  const marked = (over: object = {}) =>
    validateBiome({
      ...biome,
      palettes,
      roads: { default: plain, road: { ...plain, markings: { ...markings, ...over } } },
    } as unknown as Biome);
  const [road, country] = roadLooks(marked());
  expect([...road.paint]).toEqual([1, 1, 1, 0.75]);
  // Half the line's width, a dash, a dash and its gap, the wear.
  expect([...road.marks]).toEqual([0.125, 2, 6, 0.25]);
  expect([...road.crossing]).toEqual([0.5, 4, 1.5, 0.25]);
  // A road without markings has no line to draw.
  expect(country.marks.x).toBe(0);
  // The bars end 5.5 m from the road that crosses, and a dash that stops
  // short of them is 2 m long: farther than any shoulder.
  const reach = (b: Biome) => groundReach(b, 0.1).paved;
  expect(reach(marked())).toBeGreaterThan(7.5);
  expect(reach(marked())).toBeGreaterThan(
    reach(marked({ crossing: { ...markings.crossing, length_m: 1 } })),
  );
  expect(() => marked({ dash_m: [2] })).toThrow(
    /roads\.road\.markings\.dash_m: must be \[dash, gap\]/,
  );
  expect(() => marked({ line_m: 3 })).toThrow(/roads\.road\.markings\.line_m/);
});

test("paving is painted in the simulation's order unless a row names its layer", () => {
  const plain = { ...biome.roads.default, layer: undefined };
  const rows = (road: object) =>
    validateBiome({ ...biome, roads: { default: plain, road: { ...plain, ...road } } } as Biome);
  const names = (order: number[]) => order.map((tag) => SURFACE_AREA_KINDS[tag]);
  // Lowest first: the earlier kind is on top.
  expect(names(roadOrder(rows({})))).toEqual(["sidewalk", "dirt_track", "country_road", "road"]);
  // A street under the country road it meets, still over a track.
  expect(names(roadOrder(rows({ layer: 2.5 })))).toEqual([
    "sidewalk",
    "dirt_track",
    "road",
    "country_road",
  ]);
});

test("a paved area's authored kind selects its appearance even when every road has that kind", () => {
  const street = { ...biome.roads.default, roughness: 0.5 };
  const country = { ...biome.roads.default, roughness: 0.75 };
  const rows = validateBiome({
    ...biome,
    roads: { default: biome.roads.default, road: street, country_road: country },
  });
  const drawn = (kind: "road" | "country_road") => {
    const { site } = buildTerrainSurface(
      world({
        ...riverLab,
        surfaces: [
          {
            kind,
            shape: {
              kind: "stroke",
              points: [
                [40, 40],
                [200, 40],
              ],
              width_m: 8,
            },
          },
        ],
      }).exports,
      layout,
      rows,
    );
    const tag = site.surfaceStrokes[5];
    return roadLooks(rows)[tag].core.w;
  };
  expect(drawn("road")).toBe(0.5);
  expect(drawn("country_road")).toBe(0.75);
});

test("the forest floor names a palette of litter, moss and humus, and its numbers are checked", () => {
  // The floor is broken up by litter, moss, humus and roots.
  const floor = biome.forest_floor;
  expect(biome.palettes[floor.palette].length).toBeGreaterThanOrEqual(3);
  const short = { ...biome, palettes: { ...biome.palettes, [floor.palette]: [[0.3, 0.3, 0.2]] } };
  expect(() => validateBiome(short as Biome, "summer")).toThrow(/summer\.forest_floor\.palette/);
  const flecks = { ...biome, forest_floor: { ...floor, dapple: { ...floor.dapple, sun: 2 } } };
  expect(() => validateBiome(flecks, "summer")).toThrow(/summer\.forest_floor\.dapple\.sun/);
});

test("a shore that could draw dark, over its own wet bank or steeper than its bank is refused", () => {
  const shore = biome.shore;
  const refused = (change: Partial<typeof shore>, field: string) =>
    expect(() => validateBiome({ ...biome, shore: { ...shore, ...change } }, "summer")).toThrow(
      new RegExp(`summer\\.shore\\.${field}`),
    );
  // Darker than the ground it covers: it would read as a shadow on the field.
  refused({ lift: 0.9 }, "lift");
  // The earth's line wandering in past the wet bank's end.
  refused({ wander: 0.5, mud_m: shore.wet_m * 1.5 }, "mud_m");
  // Shading a bank steeper than it was cut.
  refused({ relief: 1.2 }, "relief");
  const silt = { ...biome.palettes, [shore.palette]: biome.palettes[shore.palette].slice(0, 1) };
  expect(() => validateBiome({ ...biome, palettes: silt }, "summer")).toThrow(
    /summer\.shore\.palette/,
  );
});

test("mixed forest exports retain authored IDs and real concave and square-ended strip membership", () => {
  const forest = (shape: unknown) => ({ shape });
  const { view, exports } = world({
    size: [256, 128],
    fog_cell_m: 8,
    height_grid_m: 4,
    slope_cutoff_deg: 35,
    forests: [
      forest({
        kind: "polygon",
        ring: [
          [0, 0],
          [90, 0],
          [90, 27],
          [27, 27],
          [27, 90],
          [0, 90],
        ],
      }),
      forest({
        kind: "polygon",
        ring: [
          [180, 0],
          [240, 0],
          [240, 60],
          [180, 60],
        ],
      }),
      forest({
        kind: "stroke",
        points: [
          [100, 20],
          [160, 80],
        ],
        width_m: 18,
      }),
    ],
  });
  const surface = buildTerrainSurface(exports, layout, biome);
  const shapes = surface.site.forestShapes;
  expect(Array.from(exports.forestRectIds)).toEqual([1]);
  expect(Array.from(exports.forestMetadata)).toEqual([0, 12, 2, 1, 12, 0, 2, 12, 1]);
  expect(shapes.map((shape) => [shape.canopy, shape.kind])).toEqual([
    [12, "polygon"],
    [12, "rectangle"],
    [12, "stroke"],
  ]);
  for (const [id, x, y, distance] of [
    [0, 10, 50, 10],
    [0, 50, 50, -23],
    [0, 27, 70, 0],
    [1, 210, 30, 30],
    [2, 130, 50, 9],
    [2, 100, 80, -Math.SQRT2 * 30 + 9],
    // The strip ends square across its first point (100, 20): behind it, just
    // inside it, and off its corner at (106.36, 13.64): 3.73 m aside of the
    // strip's width and 1.41 m past its end.
    [2, 100, 10, -Math.hypot(5, 5)],
    [2, 101, 21, Math.SQRT2],
    [2, 108, 10, -Math.hypot(18 / Math.SQRT2 - 9, Math.SQRT2)],
  ]) {
    expect(forestInside(shapes[id], x, y)).toBeCloseTo(distance, 10);
    expect(shapes.some((shape) => forestInside(shape, x, y) >= 0)).toBe(
      view.surface_at(x, y)[6] === 1,
    );
  }
});

test("a rotated prop ray preserves the portable world normal bits", () => {
  const view = new WorldView(
    JSON.stringify({
      size: [200, 200],
      fog_cell_m: 8,
      height_grid_m: 4,
      slope_cutoff_deg: 35,
      props: [{ kind: "wall", center: [50, 50], yaw: -1.785965, half_extents: [3, 5, 4.225] }],
    }),
    JSON.stringify(GAME_RULES),
  );
  try {
    const hit = view.raycast(50, 30, 1, 0, 1, 0, 100);
    expect(hit[7]).toBe(0);
    const normalBits = new BigUint64Array(new Float64Array(hit.slice(4, 7)).buffer);
    expect(Array.from(normalBits)).toEqual([0xbfcb545e465efc4en, 0xbfef431880772275n, 0n]);
  } finally {
    view.free();
  }
});
