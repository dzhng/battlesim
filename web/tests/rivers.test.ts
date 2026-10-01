// @vitest-environment node
// Rivers at the renderer's seam: the export follows its published layout, the
// water surface covers the water once, and fields are cut along the authored
// runs.
import { VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import { readFileSync } from "node:fs";
import { afterEach, beforeAll, expect, test } from "vitest";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";
import {
  buildWorldLayers,
  readWorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh.ts";
import {
  RIVER_FIELDS,
  RIVER_FLOATS,
  stretchInside,
} from "@packages/battle-renderer/src/terrain/rivers";
import { plotAt } from "@packages/battle-renderer/src/terrain/plots.ts";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import summer from "@fixtures/biomes/summer.json";
import { loadMap } from "@web/maps/node";

const riverLab = loadMap("river").definition;
const river = riverLab.rivers![0];
const villageMap = loadMap("village").definition;

const biome = validateBiome(summer as unknown as Biome);
/** How near the map's edge a field may still span a river that ends there. */
const EDGE_M = 80;
let layout: WorldLayout;
const views: WorldView[] = [];
afterEach(() => {
  for (const view of views.splice(0)) view.free();
});
beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  layout = JSON.parse(world_layout(JSON.stringify(VILLAGE_RULES))) as WorldLayout;
});

function exportsOf(map: unknown) {
  const view = new WorldView(JSON.stringify(map), JSON.stringify(VILLAGE_RULES));
  views.push(view);
  return readWorldExports(view);
}

test("the river export follows its published layout", () => {
  expect(layout.riverFields).toEqual([...RIVER_FIELDS]);
  expect(layout.riverStride).toBe(RIVER_FLOATS);
  expect(layout.riverRunFields).toEqual(["ax", "ay", "bx", "by"]);
  const exports = exportsOf(riverLab);
  const authored = river.points;
  const { rivers } = exports;
  expect(rivers.length % RIVER_FLOATS).toBe(0);
  // Each authored corner is rounded: more stretches than authored runs.
  expect(rivers.length / RIVER_FLOATS).toBeGreaterThan(authored.length * 2);
  // The line starts and ends on the authored ends, at their width and grade.
  const last = rivers.length - RIVER_FLOATS;
  const [first, final] = [authored[0], authored[authored.length - 1]];
  expect([rivers[0], rivers[1], rivers[4], rivers[6]]).toEqual(
    [...first.xy, first.width_m / 2, first.depth_m / (first.width_m / 2)].map(Math.fround),
  );
  expect([rivers[last + 2], rivers[last + 3], rivers[last + 5], rivers[last + 7]]).toEqual(
    [...final.xy, final.width_m / 2, final.depth_m / (final.width_m / 2)].map(Math.fround),
  );
  // Consecutive stretches join, and every one carries the river's surface.
  for (let o = 0; o < rivers.length; o += RIVER_FLOATS) {
    // The lab's land is flat at zero: every bank stands the water's depth
    // below it, above the water.
    expect(rivers[o + 8]).toBe(Math.fround(-river.surface_z));
    expect(rivers[o + 9]).toBe(Math.fround(-river.surface_z));
    expect(rivers[o + 10]).toBe(Math.fround(river.surface_z));
    if (o > 0) {
      expect([rivers[o], rivers[o + 1], rivers[o + 4]]).toEqual([
        rivers[o - RIVER_FLOATS + 2],
        rivers[o - RIVER_FLOATS + 3],
        rivers[o - RIVER_FLOATS + 5],
      ]);
    }
  }
  // A map without water exports none.
  const dry = exportsOf(villageMap);
  expect(dry.rivers.length + dry.riverRuns.length).toBe(0);
});

test("fields are cut along a river's long runs, which leave out the points that barely turn it", () => {
  // A river authored every 40 m that wanders 3 m either side of a line, then
  // turns north: 12 m of water, so a run may pass 6 m from a point it skips.
  const authored: [number, number][] = [
    [0, 100],
    [40, 103],
    [80, 97],
    [120, 102],
    [160, 98],
    [200, 100],
    [240, 150],
    [250, 220],
  ];
  const { riverRuns } = exportsOf({
    size: [300, 260],
    fog_cell_m: 8,
    height_grid_m: 4,
    slope_cutoff_deg: 35,
    rivers: [
      { points: authored.map((xy) => ({ xy, width_m: 12, depth_m: 1.5 })), surface_z: -0.5 },
    ],
  });
  const runs: number[][] = [];
  for (let o = 0; o < riverRuns.length; o += layout.riverRunStride)
    runs.push(Array.from(riverRuns.subarray(o, o + 4)));
  // The wandering stretch is one run; the turn keeps its points.
  expect(runs).toEqual([
    [0, 100, 200, 100],
    [200, 100, 240, 150],
    [240, 150, 250, 220],
  ]);
  // The lab's meander is authored every 5 m: its runs join authored points
  // end to end, far fewer than were authored, and pass within the narrowest
  // water's half width (6 m) of every point they leave out.
  const lab = river.points;
  const labRuns = exportsOf(riverLab).riverRuns;
  expect(labRuns.length / layout.riverRunStride).toBeLessThan(lab.length / 4);
  let at = 0;
  for (let o = 0; o < labRuns.length; o += layout.riverRunStride) {
    const [ax, ay, bx, by] = labRuns.subarray(o, o + 4);
    expect(lab[at].xy.map(Math.fround)).toEqual([ax, ay]);
    const length = Math.hypot(bx - ax, by - ay);
    while (Math.fround(lab[at].xy[0]) !== bx || Math.fround(lab[at].xy[1]) !== by) {
      const [px, py] = lab[at].xy;
      const off = Math.abs((px - ax) * (by - ay) - (py - ay) * (bx - ax)) / length;
      expect(off, `authored point ${at} from its run`).toBeLessThanOrEqual(6);
      at++;
    }
  }
  expect(at).toBe(lab.length - 1);
});

test("the water surface covers the water once, at its surface height", () => {
  const exports = exportsOf(riverLab);
  const { water, terrain } = buildWorldLayers(exports, layout, biome, "surface");
  const rivers = terrain.site.rivers;
  // Each cell is two triangles; index them by their low corner.
  const cells = new Map<string, number>();
  let cell = 0;
  for (let v = 0; v < water.length; v += 6 * VERTEX_FLOATS) {
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity;
    for (let k = 0; k < 6; k++) {
      const at = v + k * VERTEX_FLOATS;
      minX = Math.min(minX, water[at]);
      maxX = Math.max(maxX, water[at]);
      minY = Math.min(minY, water[at + 1]);
      expect(water[at + 2]).toBe(Math.fround(river.surface_z));
    }
    cell = maxX - minX;
    const key = `${minX},${minY}`;
    cells.set(key, (cells.get(key) ?? 0) + 1);
  }
  expect(cell).toBeGreaterThan(0);
  expect([...cells.values()].every((count) => count === 1)).toBe(true);
  // Every point of the water, and a pixel's feather past its edge, is on a cell.
  let wet = 0;
  for (let y = -20; y < 500; y += 1.7)
    for (let x = -20; x < 660; x += 1.3) {
      let inside = -Infinity;
      for (let o = 0; o < rivers.length; o += RIVER_FLOATS)
        inside = Math.max(inside, stretchInside(rivers, o, x, y));
      if (inside < -2) continue;
      wet++;
      const key = `${Math.floor(x / cell) * cell},${Math.floor(y / cell) * cell}`;
      expect(cells.has(key), `water at (${x}, ${y}) has no surface`).toBe(true);
    }
  expect(wet).toBeGreaterThan(5000);
  // The surface follows the river: far fewer cells than its bounding box.
  expect(cells.size * cell * cell).toBeLessThan(640 * 480 * 0.2);
  // The traversal view shows what blocks, with no water over it.
  expect(buildWorldLayers(exports, layout, biome, "traversal").water.length).toBe(0);
});

test("fields are cut along the river's long runs: no plot spans both banks", () => {
  const exports = exportsOf(riverLab);
  const { plots } = buildWorldLayers(exports, layout, biome, "surface").terrain;
  const runs = exports.riverRuns;
  let checked = 0;
  for (let o = 0; o < runs.length; o += layout.riverRunStride) {
    const [ax, ay, bx, by] = runs.subarray(o, o + 4);
    const length = Math.hypot(bx - ax, by - ay);
    if (length < 45) continue;
    const [nx, ny] = [-(by - ay) / length, (bx - ax) / length];
    // Along the run's middle, clear of its bends, and of the map's edge: a
    // run that ends there cuts only as far as it runs, like a road's.
    for (let s = 20; s < length - 20; s += 3) {
      const [px, py] = [ax + ((bx - ax) * s) / length, ay + ((by - ay) * s) / length];
      if (px < EDGE_M || px > riverLab.size[0] - EDGE_M) continue;
      const left = plotAt(plots, px + nx * 3, py + ny * 3);
      const right = plotAt(plots, px - nx * 3, py - ny * 3);
      expect(left!.plot, `plots either side at (${px}, ${py})`).not.toBe(right!.plot);
      checked++;
    }
  }
  // The lab's two long straight runs: by the bridge and along the 30 m stretch.
  expect(checked).toBeGreaterThan(10);
});
