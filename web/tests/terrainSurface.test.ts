// @vitest-environment node
// The terrain surface against the simulation's own geometry: the drawn ground
// is WorldView's ground (heights and normals, at triangle edges and where
// props stand), and the material's road, forest and water masks are the
// simulation's surface rules.
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { readFileSync } from "node:fs";
import { afterEach, beforeAll, expect, test } from "vitest";
import { polygon2 } from "math/shapes";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
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
import {
  CUT_A,
  CUT_B,
  STROKE_CUTS,
  strokeInside,
} from "@packages/battle-renderer/src/terrain/strokes";
import { pavedKinds, SURFACE_AREA_KINDS } from "@packages/battle-renderer/src/terrain/surfaces";
import { plotAt } from "@packages/battle-renderer/src/terrain/plots.ts";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import summer from "@fixtures/biomes/summer.json";
import { loadMap } from "@web/maps/node";
import { groundHeight } from "@packages/battle-renderer/src/terrain/terrainGrid";
import { packTerrainHeights } from "@packages/battle-renderer/src/frame/terrainHeights";
import { roadLooks } from "@packages/battle-renderer/src/frame/terrainMaterial";

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

test("plots around the buildings are the settlement's meadow", () => {
  const { exports } = world(villageMap);
  const { plots } = buildTerrainSurface(exports, layout, biome);
  const settlement = biome.plots.findIndex((p) => p.name === biome.field_rules.settlement_kind);
  const houses = villageMap.buildings!.flatMap((b) => b.geometry.parts);
  for (const [x, y] of houses.map((p) => p.center))
    expect(plots.plots[plotAt(plots, x, y)!.plot].kind).toBe(settlement);
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

test("a map that names no kind but road has its roads drawn as country roads", () => {
  const street = { ...biome.roads.default, roughness: 0.5 };
  const country = { ...biome.roads.default, roughness: 0.75 };
  const rows = validateBiome({
    ...biome,
    roads: { default: biome.roads.default, road: street, country_road: country },
  });
  const drawn = (map: unknown) => {
    const { site } = buildTerrainSurface(world(map).exports, layout, rows);
    return roadLooks(rows, pavedKinds(site)).map((look) => look.core.w);
  };
  // The village's roads are `road` and it names nothing else.
  expect(drawn(villageMap).slice(0, 2)).toEqual([0.75, 0.75]);
  // Beside a road of another kind, a `road` is a street.
  const way = (kind: string, y: number) => ({
    kind,
    shape: {
      kind: "stroke",
      points: [
        [40, y],
        [200, y],
      ],
      width_m: 8,
    },
  });
  const town = { ...riverLab, surfaces: [way("road", 40), way("country_road", 80)] };
  expect(drawn(town).slice(0, 2)).toEqual([0.5, 0.75]);
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
