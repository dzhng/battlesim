// @vitest-environment node
// The terrain surface against the simulation's own geometry: the drawn ground
// is WorldView's ground (heights and normals, at triangle edges and where
// props stand), and the material's road, forest and water masks are the
// simulation's surface rules.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { polygon2 } from "math/shapes";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";
import type { WorldExports, WorldLayout } from "@packages/battle-renderer/src/worldMesh.ts";
import {
  buildTerrainSurface,
  RECT_FLOATS,
  type TerrainSurface,
} from "@packages/battle-renderer/src/terrain/terrainSurface.ts";
import { plotAt } from "@packages/battle-renderer/src/terrain/plots.ts";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import summer from "@fixtures/biomes/summer.json";
import village from "@fixtures/village.json";
import geometry from "@fixtures/geometry-lab.json";

const biome = validateBiome(summer as unknown as Biome);
let layout: WorldLayout;

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  layout = JSON.parse(world_layout()) as WorldLayout;
});

function world(map: unknown): { view: WorldView; exports: WorldExports } {
  const view = new WorldView(JSON.stringify(map));
  return {
    view,
    exports: {
      positions: view.terrain_positions(),
      indices: view.terrain_indices(),
      triangleSurfaces: view.terrain_triangle_surfaces(),
      props: view.props(),
      water: view.water(),
      forests: view.forests(),
      roads: view.roads(),
    },
  };
}

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

test("heights and normals where props stand are WorldView's", () => {
  const { view, exports } = world(village.map);
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
  expect(checked).toBe(15);
});

test("the material's road, forest and water masks are the simulation's surface rules", () => {
  for (const map of [geometry, village.map]) {
    const { view, exports } = world(map);
    const { site } = buildTerrainSurface(exports, layout, biome);
    const inRect = (rects: Float32Array, x: number, y: number) => {
      for (let r = 0; r < rects.length; r += RECT_FLOATS) {
        const [rx, ry, w, h] = rects.subarray(r, r + RECT_FLOATS);
        if (x >= rx && x <= rx + w && y >= ry && y <= ry + h) return true;
      }
      return false;
    };
    const onRoad = (x: number, y: number) => {
      for (let r = 0; r < site.roads.length; r += site.roadStride) {
        const [ax, ay, bx, by, half] = site.roads.subarray(r, r + 5);
        const [dx, dy] = [bx - ax, by - ay];
        const t = Math.min(1, Math.max(0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)));
        if (Math.hypot(x - ax - dx * t, y - ay - dy * t) <= half) return true;
      }
      return false;
    };
    const [width, depth] = map.size;
    const wrong: string[] = [];
    let roads = 0;
    for (let y = 0.37; y < depth; y += 2.3) {
      for (let x = 0.61; x < width; x += 2.3) {
        const [, , , , , kind, forest] = view.surface_at(x, y);
        const surfaceKind = layout.surfaceKinds[kind];
        // A bridge deck hides the ground kind beneath it (road or river).
        if (surfaceKind === "bridge") continue;
        const water = inRect(site.water, x, y);
        if (water !== (surfaceKind === "water")) wrong.push(`water at (${x}, ${y})`);
        if (!water && onRoad(x, y) !== (surfaceKind === "road")) wrong.push(`road at (${x}, ${y})`);
        if (inRect(site.forests, x, y) !== (forest === 1)) wrong.push(`forest at (${x}, ${y})`);
        if (surfaceKind === "road") roads++;
      }
    }
    expect(wrong).toEqual([]);
    expect(roads).toBeGreaterThan(100);
  }
});

test("roads split the patchwork: fields meet a road edge-on, never across it", () => {
  const { exports } = world(village.map);
  const { site, plots } = buildTerrainSurface(exports, layout, biome);
  for (let r = 0; r < site.roads.length; r += site.roadStride) {
    const [ax, ay, bx, by, half] = site.roads.subarray(r, r + 5);
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

test("each point lies in the plot the split walks to, and its edge distance is that plot's", () => {
  const { exports } = world(village.map);
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

test("plots around the buildings are the settlement's meadow", () => {
  const { exports } = world(village.map);
  const { plots } = buildTerrainSurface(exports, layout, biome);
  const settlement = biome.plots.findIndex((p) => p.name === biome.field_rules.settlement_kind);
  for (const [x, y] of village.map.props.map((p) => p.center))
    expect(plots.plots[plotAt(plots, x, y)!.plot].kind).toBe(settlement);
});

test("the patchwork is the same for the same seed and moves with it", () => {
  const { exports } = world(village.map);
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

test("the forest floor names a palette of litter, moss and humus, and its numbers are checked", () => {
  // Slice 19b: the floor is broken up by litter, moss, humus and roots.
  const floor = biome.forest_floor;
  expect(biome.palettes[floor.palette].length).toBeGreaterThanOrEqual(3);
  const short = { ...biome, palettes: { ...biome.palettes, [floor.palette]: [[0.3, 0.3, 0.2]] } };
  expect(() => validateBiome(short as Biome, "summer")).toThrow(/summer\.forest_floor\.palette/);
  const flecks = { ...biome, forest_floor: { ...floor, dapple: { ...floor.dapple, sun: 2 } } };
  expect(() => validateBiome(flecks, "summer")).toThrow(/summer\.forest_floor\.dapple\.sun/);
});
