// @vitest-environment node
// Scenery placement against the simulation's own geometry: the drawn forest
// is the simulation's forest volume (every crown within the canopy's radius
// of its trunk and under its canopy over the simulation's ground, the
// simulation's trunks each a drawn tree), and scenery past the map stays off
// it.
import { TEST_RULES } from "./catalog";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as generator from "@wasm/game_wasm.js";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import generated from "@fixtures/generated-battle.json";
import { generationRequest } from "@web/maps/source";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh.ts";
import { buildTerrainSurface } from "@packages/battle-renderer/src/terrain/terrainSurface.ts";
import {
  FOREST_STROKE_FLOATS,
  forestInside,
} from "@packages/battle-renderer/src/terrain/forestShapes.ts";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import {
  placeScenery,
  scenerySite,
  TREE_FIELD,
  TREE_FLOATS,
  type KindSize,
  type SceneryPlacement,
} from "@packages/battle-renderer/src/scenery/placement.ts";
import summer from "@fixtures/biomes/summer.json";
import game from "@fixtures/game.json";
import { loadMap } from "@web/maps/node";
import { WHOLE_MAP_MS } from "./support/wholeMap";

const villageMap = loadMap("village").definition;

const biome = validateBiome(summer as unknown as Biome);
/** Unscaled appearance sizes: the loader reads them from the bundles. */
const SIZES = new Map<string, KindSize>([
  ["tree_broadleaf", { height: 11, radius: 5.6 }],
  ["tree_spreading", { height: 10.5, radius: 6.1 }],
  ["tree_tall", { height: 11.5, radius: 3.8 }],
  ["tree_spruce", { height: 11.2, radius: 3.9 }],
  ["tree_pine", { height: 11, radius: 5 }],
  ["tree_birch", { height: 11.3, radius: 4.1 }],
  ["tree_snag", { height: 10.7, radius: 4.9 }],
  ["hedge_shrub", { height: 2.8, radius: 3.2 }],
]);

/** The forest floor's dressing, as built (`forest_floor.py`). */
const FLOOR_SIZES = new Map<string, KindSize>([
  ["floor_fern", { height: 0.44, radius: 0.64 }],
  ["floor_bush", { height: 0.54, radius: 0.74 }],
  ["floor_sapling", { height: 0.86, radius: 0.33 }],
  ["floor_rock", { height: 0.33, radius: 0.5 }],
  ["floor_litter", { height: 0.19, radius: 0.94 }],
]);
/** Every appearance the biome places. */
const PLACED = new Map([...SIZES, ...FLOOR_SIZES]);

let view: WorldView;
let layout: WorldLayout;
let exports: WorldExports;
let placement: SceneryPlacement;

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  layout = JSON.parse(world_layout(JSON.stringify(TEST_RULES))) as WorldLayout;
  view = new WorldView(JSON.stringify(villageMap), JSON.stringify(TEST_RULES));
  exports = readWorldExports(view);
  const site = scenerySite(exports, layout, buildTerrainSurface(exports, layout, biome));
  placement = placeScenery(site, biome, PLACED);
});

interface Tree {
  x: number;
  y: number;
  z: number;
  radius: number;
  top: number;
  /** The drawn crown's width over its appearance's own. */
  girth: number;
}
function trees(data: Float32Array): Tree[] {
  const out: Tree[] = [];
  for (let o = 0; o < data.length; o += TREE_FLOATS) {
    const size = SIZES.get(placement.kinds[data[o + TREE_FIELD.kind]])!;
    out.push({
      x: data[o + TREE_FIELD.x],
      y: data[o + TREE_FIELD.y],
      z: data[o + TREE_FIELD.z],
      radius: size.radius * data[o + TREE_FIELD.scaleXY],
      top: data[o + TREE_FIELD.z] + size.height * data[o + TREE_FIELD.scaleZ],
      girth: data[o + TREE_FIELD.scaleXY],
    });
  }
  return out;
}
const ground = (x: number, y: number) => view.surface_at(x, y)[0];
const forests = () =>
  villageMap.forests.map(({ shape }) => {
    if (shape.kind !== "polygon") throw new Error("the village's forests are rings");
    const { ring } = shape;
    return {
      rect: [ring[0][0], ring[0][1], ring[1][0] - ring[0][0], ring[2][1] - ring[1][1]] as [
        number,
        number,
        number,
        number,
      ],
      canopy: game.forests.rule.canopy_height_m,
    };
  });

test("every forest tree's crown reaches no farther than the simulation's canopy radius, under its canopy over the simulation's ground", () => {
  const drawn = trees(placement.forest);
  expect(drawn.length).toBeGreaterThan(400);
  // The simulation's foliage lies within the canopy radius of a trunk, past
  // the forest's own edge too: a crown is as wide at the edge as inside.
  const canopyRadius = game.forests.rule.canopy_radius_m;
  expect(Math.max(...[...SIZES.values()].map((s) => s.radius))).toBeLessThanOrEqual(canopyRadius);
  for (const t of drawn) {
    expect(t.radius, JSON.stringify(t)).toBeLessThanOrEqual(canopyRadius + 1e-3);
    const forest = forests().find(
      ({ rect: [x, y, w, h] }) => t.x >= x && t.x <= x + w && t.y >= y && t.y <= y + h,
    );
    expect(forest, JSON.stringify(t)).toBeDefined();
    // The simulation's foliage is below ground + canopy at each point: sample
    // the crown's disc, where its top stands.
    for (let k = 0; k < 17; k++) {
      const r = k === 0 ? 0 : k <= 8 ? t.radius / 2 : t.radius;
      const a = (k * Math.PI) / 4;
      const g = ground(t.x + r * Math.cos(a), t.y + r * Math.sin(a));
      expect(t.top, JSON.stringify(t)).toBeLessThanOrEqual(g + forest!.canopy + 1e-3);
    }
  }
});

test("a tree at its forest's edge is as wide as one deep inside", () => {
  const [lo, hi] = biome.trees.forest.girth;
  const [[x, y, w, h]] = forests().map((f) => f.rect);
  const west = trees(placement.forest).filter(
    (t) => t.x >= x && t.x <= x + w && t.y >= y && t.y <= y + h,
  );
  const edge = west.filter((t) => Math.min(t.x - x, x + w - t.x, t.y - y, y + h - t.y) < 3);
  expect(edge.length).toBeGreaterThan(10);
  for (const t of west) {
    expect(t.girth, JSON.stringify(t)).toBeGreaterThanOrEqual(lo - 1e-6);
    expect(t.girth, JSON.stringify(t)).toBeLessThanOrEqual(hi + 1e-6);
  }
});

test("a biome may not draw a tree wider than its appearance", () => {
  // The appearance is held inside the simulation's canopy radius; a wider
  // drawing of it would not be.
  const wide = {
    ...biome,
    trees: { ...biome.trees, forest: { ...biome.trees.forest, girth: [0.9, 1.2] } },
  };
  expect(() => validateBiome(wide as unknown as Biome, "summer")).toThrow(
    /summer\.trees\.forest\.girth/,
  );
});

test("trees stand on the simulation's ground", () => {
  for (const t of [...trees(placement.forest), ...trees(placement.backdrop)].slice(0, 4000)) {
    const g = t.x >= 0 && t.y >= 0 && t.x <= 1600 && t.y <= 1600 ? ground(t.x, t.y) : null;
    if (g !== null) {
      expect(t.z).toBeLessThanOrEqual(g);
      expect(t.z).toBeGreaterThan(g - 0.3);
    }
  }
});

test("the forest draws exactly the simulation's trunks, one tree each", () => {
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const drawn = trees(placement.forest);
  let trunks = 0;
  for (let o = 0; o < exports.props.length; o += layout.propStride) {
    if (layout.propKinds[exports.props[o + at.kind]] !== "trunk") continue;
    trunks++;
    const [x, y] = [exports.props[o + at.x], exports.props[o + at.y]];
    expect(drawn.some((t) => Math.hypot(t.x - x, t.y - y) < 1e-3)).toBe(true);
  }
  expect(trunks).toBeGreaterThan(50);
  expect(drawn.length).toBe(trunks);
});

test("each forest tree names the trunk it stands on, so a published fall finds it", () => {
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const trunkSpot = new Map<number, [number, number]>();
  for (let o = 0; o < exports.props.length; o += layout.propStride) {
    if (layout.propKinds[exports.props[o + at.kind]] !== "trunk") continue;
    const id = exports.props[o + at.idLo] + exports.props[o + at.idHi] * 2 ** layout.limbBits;
    trunkSpot.set(id, [exports.props[o + at.x], exports.props[o + at.y]]);
  }
  const drawn = trees(placement.forest);
  expect(placement.forestIds.length).toBe(drawn.length);
  expect(new Set(placement.forestIds).size).toBe(trunkSpot.size);
  drawn.forEach((t, i) => {
    const [x, y] = trunkSpot.get(placement.forestIds[i])!;
    expect(Math.hypot(t.x - x, t.y - y), `tree ${i}`).toBeLessThan(1e-3);
  });
});

/** Whether (x, y) is on a road's surface. */
function onRoad(x: number, y: number): boolean {
  const r = exports.surfaceStrokes;
  for (let o = 0; o < r.length; o += layout.surfaceStrokeStride) {
    const [ax, ay, bx, by, half] = [r[o], r[o + 1], r[o + 2], r[o + 3], r[o + 4]];
    const len2 = (bx - ax) ** 2 + (by - ay) ** 2;
    const s = Math.max(0, Math.min(1, ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / len2));
    if (Math.hypot(x - ax - s * (bx - ax), y - ay - s * (by - ay)) <= half) return true;
  }
  return false;
}

test("no drawn trunk stands on a road", () => {
  for (const t of trees(placement.forest)) expect(onRoad(t.x, t.y), JSON.stringify(t)).toBe(false);
});

test("scenery past the map stays clear of it and within reach, as hedgerows and copses", () => {
  const drawn = trees(placement.backdrop);
  expect(drawn.length).toBeGreaterThan(2000);
  const { clear_m, reach_m } = biome.trees.backdrop;
  for (const t of drawn) {
    const dx = Math.max(-t.x, t.x - 1600, 0);
    const dy = Math.max(-t.y, t.y - 1600, 0);
    const outside = Math.hypot(dx, dy);
    expect(outside - t.radius, JSON.stringify(t)).toBeGreaterThanOrEqual(clear_m - 1e-3);
    expect(outside).toBeLessThanOrEqual(reach_m + 1e-3);
  }
  // Every kind but those that stand only deep inside a forest.
  const kinds = new Set<string>();
  for (let o = 0; o < placement.backdrop.length; o += TREE_FLOATS)
    kinds.add(placement.kinds[placement.backdrop[o + TREE_FIELD.kind]]);
  const deep = biome.trees.species.filter((s) => s.interior_m !== undefined);
  expect(deep.length).toBeGreaterThan(0);
  expect([...kinds].sort()).toEqual(
    [...SIZES.keys()].filter((k) => !deep.some((s) => s.appearance === k)).sort(),
  );
});

test("map-owned surrounding scenery keeps each complete crown inside the rendered extent", () => {
  const surrounded = new WorldView(
    JSON.stringify({ ...villageMap, render_margin_m: 200 }),
    JSON.stringify(TEST_RULES),
  );
  try {
    const exported = readWorldExports(surrounded);
    const surface = buildTerrainSurface(exported, layout, biome);
    const placed = placeScenery(scenerySite(exported, layout, surface), biome, PLACED);
    expect(placed.backdrop.length).toBeGreaterThan(0);
    const b = exported.extents.rendered;
    for (let o = 0; o < placed.backdrop.length; o += TREE_FLOATS) {
      const t = placed.backdrop;
      const kind = placed.kinds[t[o + TREE_FIELD.kind]];
      const radius = SIZES.get(kind)!.radius * t[o + TREE_FIELD.scaleXY];
      const [x, y] = [t[o + TREE_FIELD.x], t[o + TREE_FIELD.y]];
      // Float32 placement packing narrows coordinates at the export seam.
      expect(x - radius).toBeGreaterThanOrEqual(b[0] - 1e-3);
      expect(y - radius).toBeGreaterThanOrEqual(b[1] - 1e-3);
      expect(x + radius).toBeLessThanOrEqual(b[2] + 1e-3);
      expect(y + radius).toBeLessThanOrEqual(b[3] + 1e-3);
    }
  } finally {
    surrounded.free();
  }
});

/** The appearance of each placed tree, with its place. */
function named(placed: SceneryPlacement, data: Float32Array) {
  const out: { x: number; y: number; appearance: string }[] = [];
  for (let o = 0; o < data.length; o += TREE_FLOATS)
    out.push({
      x: data[o + TREE_FIELD.x],
      y: data[o + TREE_FIELD.y],
      appearance: placed.kinds[data[o + TREE_FIELD.kind]],
    });
  return out;
}

test("a wood is stands, each mostly one family's species, with the odd tree of no family", () => {
  // Stands small enough that the village's two woods hold many.
  const stands = { size_m: 50, purity: 0.9 };
  const small = { ...biome, trees: { ...biome.trees, stands } };
  const site = scenerySite(exports, layout, buildTerrainSurface(exports, layout, biome));
  const placed = placeScenery(site, small, PLACED);
  const family = new Map(biome.trees.species.map((s) => [s.appearance, s.family]));
  const drawn = named(placed, placed.forest).map((t) => ({
    ...t,
    family: family.get(t.appearance),
  }));
  const inFamily = drawn.filter((t) => t.family !== undefined);
  const families = [...new Set(inFamily.map((t) => t.family))];
  expect(families.length).toBeGreaterThan(1);
  // Each family has stands of its own somewhere in the two woods.
  for (const f of families)
    expect(inFamily.filter((t) => t.family === f).length / inFamily.length).toBeGreaterThan(0.1);
  // A tree's nearest neighbour is of its family far more often than chance
  // (the families' own shares) would have it.
  let same = 0;
  for (const t of inFamily) {
    let nearest = inFamily[0],
      best = Infinity;
    for (const u of inFamily) {
      const d = Math.hypot(u.x - t.x, u.y - t.y);
      if (u !== t && d < best) [nearest, best] = [u, d];
    }
    if (nearest.family === t.family) same++;
  }
  const chance = families
    .map((f) => inFamily.filter((t) => t.family === f).length / inFamily.length)
    .reduce((sum, share) => sum + share * share, 0);
  expect(same / inFamily.length).toBeGreaterThan(chance + 0.2);
  // Trees of no family stand among them, about as often as their weight says.
  const total = biome.trees.species.reduce((sum, s) => sum + s.weight, 0);
  const odd = biome.trees.species.filter(
    (s) => s.family === undefined && s.interior_m === undefined,
  );
  expect(odd.length).toBeGreaterThan(0);
  for (const s of odd) {
    const share = drawn.filter((t) => t.appearance === s.appearance).length / drawn.length;
    expect(share).toBeGreaterThan((0.4 * s.weight) / total);
    expect(share).toBeLessThan((2 * s.weight) / total);
  }
});

/** A map of one forest of `shape`, its site and the forest's drawn trees
 *  under `trees`. */
function wood(shape: unknown, trees: Biome["trees"]) {
  const view = new WorldView(
    JSON.stringify({
      size: [400, 400],
      height_grid_m: 4,
      fog_cell_m: 8,
      slope_cutoff_deg: 35,
      forests: [{ shape }],
    }),
    JSON.stringify(TEST_RULES),
  );
  try {
    const exported = readWorldExports(view);
    const site = scenerySite(exported, layout, buildTerrainSurface(exported, layout, biome));
    const placed = placeScenery(site, { ...biome, trees }, PLACED);
    return { site, drawn: named(placed, placed.forest) };
  } finally {
    view.free();
  }
}

test("a species kept to the interior stands that deep inside its forest, and in no strip", () => {
  // Whatever its weight: here every tree would be one if it could.
  const [deep] = biome.trees.species.filter((s) => s.interior_m !== undefined);
  const keen = {
    ...biome.trees,
    species: biome.trees.species.map((s) => (s === deep ? { ...s, weight: 1000 } : s)),
  };
  const ring = [
    [40, 40],
    [360, 40],
    [360, 360],
    [40, 360],
  ];
  const polygon = wood({ kind: "polygon", ring }, keen);
  const inside = polygon.drawn.filter((t) => t.appearance === deep.appearance);
  expect(inside.length).toBeGreaterThan(100);
  for (const t of inside)
    expect(forestInside(polygon.site.forests[0], t.x, t.y)).toBeGreaterThanOrEqual(
      deep.interior_m!,
    );
  // The outer ring is drawn all the same, as other species.
  const edge = polygon.drawn.filter(
    (t) => forestInside(polygon.site.forests[0], t.x, t.y) < deep.interior_m!,
  );
  expect(edge.length).toBeGreaterThan(20);
  const strip = wood(
    {
      kind: "stroke",
      points: [
        [40, 200],
        [360, 200],
      ],
      width_m: 60,
    },
    keen,
  );
  expect(strip.drawn.length).toBeGreaterThan(50);
  expect(strip.drawn.filter((t) => t.appearance === deep.appearance)).toEqual([]);
});

test("the summer woods' interior-only trees (snags) are at most one in twenty", () => {
  const deep = biome.trees.species
    .filter((s) => s.interior_m !== undefined)
    .map((s) => s.appearance);
  const drawn = named(placement, placement.forest);
  const share = drawn.filter((t) => deep.includes(t.appearance)).length / drawn.length;
  expect(share).toBeGreaterThan(0);
  expect(share).toBeLessThanOrEqual(0.05);
});

test("placement is deterministic", () => {
  const site = scenerySite(exports, layout, buildTerrainSurface(exports, layout, biome));
  const again = placeScenery(site, biome, PLACED);
  expect(again.kinds).toEqual(placement.kinds);
  expect(Array.from(again.forest)).toEqual(Array.from(placement.forest));
  expect(Array.from(again.backdrop)).toEqual(Array.from(placement.backdrop));
});

test("placement refuses a species the catalog lacks, by name", () => {
  const site = scenerySite(exports, layout, buildTerrainSurface(exports, layout, biome));
  const sizes = new Map(SIZES);
  sizes.delete("tree_tall");
  expect(() => placeScenery(site, biome, sizes)).toThrow(/tree_tall/);
});

test("overlapping polygon and strip draw each original native-owned trunk exactly once", () => {
  const forest = (shape: unknown) => ({ shape });
  const overlap = new WorldView(
    JSON.stringify({
      size: [100, 100],
      height_grid_m: 4,
      fog_cell_m: 8,
      slope_cutoff_deg: 35,
      props: [{ kind: "trunk", center: [9, 9], yaw: 0, half_extents: [0.35, 0.35, 5] }],
      bridges: [
        {
          deck: "bridge_deck",
          center: [90, 10],
          half_extents: [5, 5],
          yaw: 0,
          deck_z: 0.2,
          thickness_m: 0.8,
        },
      ],
      forests: [
        forest({
          kind: "polygon",
          ring: [
            [0, 0],
            [72, 0],
            [72, 36],
            [36, 36],
            [36, 72],
            [0, 72],
          ],
        }),
        forest({
          kind: "stroke",
          points: [
            [18, 18],
            [54, 54],
          ],
          width_m: 36,
        }),
        forest({
          kind: "polygon",
          ring: [
            [90, 90],
            [91, 90],
            [91, 91],
            [90, 91],
          ],
        }),
      ],
    }),
    JSON.stringify(TEST_RULES),
  );
  try {
    const exported = readWorldExports(overlap);
    const ranges = exported.forestTrunkRanges;
    expect(Array.from(ranges.slice(0, 1))).toEqual([2]);
    expect(ranges[4]).toBe(ranges[5]);
    const at = Object.fromEntries(layout.propFields.map((field, i) => [field, i]));
    const expected: number[][] = [];
    for (let o = 0; o < exported.props.length; o += layout.propStride) {
      const id = exported.props[o + at.idLo] + exported.props[o + at.idHi] * 2 ** layout.limbBits;
      if (id >= ranges[0] && id < ranges[3])
        expected.push([exported.props[o + at.x], exported.props[o + at.y]]);
    }
    expect(expected.length).toBeGreaterThan(30);
    const site = scenerySite(exported, layout, buildTerrainSurface(exported, layout, biome));
    const placed = placeScenery(site, biome, PLACED);
    // Then the trunk the map authored, which no forest generated: a tree of
    // its own.
    expect(trees(placed.forest).map((tree) => [tree.x, tree.y])).toEqual([...expected, [9, 9]]);
  } finally {
    overlap.free();
  }
});

// ---------------------------------------------------- the forest floor's dressing

const DRESSING = biome.forest_floor.dressing;

/** Every piece of dressing the field lays, cell by cell: its appearance, its
 *  foot and its scale. */
function pieces(placed: SceneryPlacement) {
  const out: { x: number; y: number; z: number; appearance: string; scale: number }[] = [];
  const field = placed.dressing;
  for (let c = 0; c < field.cells.length; c += 2) {
    const data = field.place(field.cells[c], field.cells[c + 1]);
    for (let o = 0; o < data.length; o += TREE_FLOATS)
      out.push({
        x: data[o + TREE_FIELD.x],
        y: data[o + TREE_FIELD.y],
        z: data[o + TREE_FIELD.z],
        appearance: placed.kinds[data[o + TREE_FIELD.kind]],
        scale: Math.max(data[o + TREE_FIELD.scaleXY], data[o + TREE_FIELD.scaleZ]),
      });
  }
  return out;
}

/** The simulation's own word on whether (x, y) is forest ground. */
const forestGround = (world: WorldView, x: number, y: number) => world.surface_at(x, y)[6] === 1;

/** The scenery site of `world`. */
function siteOf(world: WorldView) {
  const worldExports = readWorldExports(world);
  return {
    worldExports,
    site: scenerySite(worldExports, layout, buildTerrainSurface(worldExports, layout, biome)),
  };
}

test("dressing lies only on the simulation's forest ground, on the ground, off its roads", () => {
  const dressed = pieces(placement);
  // Both woods are dressed, by every kind.
  expect(dressed.length).toBeGreaterThan(1000);
  expect(new Set(dressed.map((p) => p.appearance))).toEqual(
    new Set(DRESSING.kinds.map((k) => k.appearance)),
  );
  const woods = forests().map((f) => f.rect);
  for (const p of dressed) {
    expect(forestGround(view, p.x, p.y), JSON.stringify(p)).toBe(true);
    // Inside the edge the floor's verge wanders about: on the drawn floor.
    const inside = Math.max(
      ...woods.map(([x, y, w, h]) => Math.min(p.x - x, x + w - p.x, p.y - y, y + h - p.y)),
    );
    expect(inside, JSON.stringify(p)).toBeGreaterThanOrEqual(DRESSING.edge_m - 1e-3);
    expect(onRoad(p.x, p.y), JSON.stringify(p)).toBe(false);
    const g = ground(p.x, p.y);
    expect(p.z, JSON.stringify(p)).toBeLessThanOrEqual(g);
    expect(p.z, JSON.stringify(p)).toBeGreaterThan(g - 0.1);
  }
  // The road through the east wood (12 m wide, along x = 1150) is left
  // bare, and the clearance beside it.
  const beside = dressed.filter(
    (p) => Math.abs(p.x - 1150) < 6 + DRESSING.clear_m - 1e-3 && p.y > 600 && p.y < 780,
  );
  expect(beside).toEqual([]);
});

test("dressing keeps clear of every trunk and of every body on the floor", () => {
  // The floor's own cover (logs and boulders) at the densities its systems
  // tests use: the village's default may hold none.
  const rules = structuredClone(TEST_RULES) as typeof TEST_RULES;
  rules.forests.rule.logs_per_ha = 5;
  rules.forests.rule.boulders_per_ha = 3;
  const world = new WorldView(JSON.stringify(villageMap), JSON.stringify(rules));
  try {
    const { worldExports, site } = siteOf(world);
    const dressed = pieces(placeScenery(site, biome, PLACED));
    const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
    const p = worldExports.props;
    let trunks = 0,
      bodies = 0;
    const tooNear: string[] = [];
    for (let o = 0; o < p.length; o += layout.propStride) {
      const kind = layout.propKinds[p[o + at.kind]];
      if (kind !== "trunk" && kind !== "log" && kind !== "boulder") continue;
      const [x, y, yaw] = [p[o + at.x], p[o + at.y], p[o + at.yaw]];
      const [hx, hy] = [p[o + at.hx], p[o + at.hy]];
      const [c, s] = [Math.cos(yaw), Math.sin(yaw)];
      if (kind === "trunk") trunks++;
      else bodies++;
      for (const piece of dressed) {
        const [dx, dy] = [piece.x - x, piece.y - y];
        const near =
          kind === "trunk"
            ? Math.hypot(dx, dy) < DRESSING.trunk_clear_m - 1e-3
            : // Inside the body's own box, turned as it lies.
              Math.abs(dx * c + dy * s) < hx && Math.abs(dy * c - dx * s) < hy;
        if (near) tooNear.push(`${kind} at ${x}, ${y}: ${JSON.stringify(piece)}`);
      }
    }
    expect(tooNear).toEqual([]);
    expect(trunks).toBeGreaterThan(50);
    expect(bodies).toBeGreaterThan(3);
  } finally {
    world.free();
  }
});

test("a strip of forest and a concave wood are dressed inside their own shapes, about as thickly", () => {
  const strip = {
    points: [
      [100, 20],
      [160, 80],
    ],
    width_m: 18,
  };
  const ring = [
    [0, 0],
    [90, 0],
    [90, 27],
    [27, 27],
    [27, 90],
    [0, 90],
  ];
  const map = {
    size: [256, 128],
    height_grid_m: 4,
    fog_cell_m: 8,
    slope_cutoff_deg: 35,
    forests: [{ shape: { kind: "polygon", ring } }, { shape: { kind: "stroke", ...strip } }],
  };
  const world = new WorldView(JSON.stringify(map), JSON.stringify(TEST_RULES));
  try {
    const { site } = siteOf(world);
    // Evenly and out to the edge, so a count is an area: no kind gathers in
    // drifts, and none keeps back from the forest's edge.
    const kinds = DRESSING.kinds.map((k) => ({ ...k, drift: 0 }));
    const even = {
      ...biome,
      forest_floor: { ...biome.forest_floor, dressing: { ...DRESSING, kinds, edge_m: 0 } },
    };
    const dressed = pieces(placeScenery(site, even, PLACED));
    for (const p of dressed) expect(forestGround(world, p.x, p.y), JSON.stringify(p)).toBe(true);
    const inStrip = dressed.filter((p) => p.x > 95);
    const perHa = (count: number, areaM2: number) => (count / areaM2) * 10000;
    const stripArea = Math.hypot(60, 60) * strip.width_m;
    const woodArea = 90 * 27 + 27 * 63;
    // Trunks take their clearance out of both.
    for (const density of [
      perHa(inStrip.length, stripArea),
      perHa(dressed.length - inStrip.length, woodArea),
    ]) {
      expect(density).toBeGreaterThan(DRESSING.per_ha * 0.7);
      expect(density).toBeLessThanOrEqual(DRESSING.per_ha * 1.15);
    }
    // The same seed dresses the same floor.
    expect(pieces(placeScenery(site, even, PLACED))).toEqual(dressed);
  } finally {
    world.free();
  }
});

test("the dressing is laid a cell of ground at a time: a cell's own pieces, the same whenever it is laid", () => {
  const field = placement.dressing;
  expect(field.cells.length / 2).toBeGreaterThan(10);
  let laid = 0;
  for (let c = 0; c < field.cells.length; c += 2) {
    const [i, j] = [field.cells[c], field.cells[c + 1]];
    const data = field.place(i, j);
    expect(data.length / TREE_FLOATS).toBeLessThanOrEqual(field.capacity);
    for (let o = 0; o < data.length; o += TREE_FLOATS) {
      expect(Math.floor(data[o + TREE_FIELD.x] / field.cellM)).toBe(i);
      expect(Math.floor(data[o + TREE_FIELD.y] / field.cellM)).toBe(j);
    }
    expect(field.place(i, j)).toEqual(data);
    laid += data.length / TREE_FLOATS;
  }
  expect(laid).toBeGreaterThan(1000);
  // Its tallest piece is a kind's own height at most: nothing is scaled up.
  expect(field.topM).toBeLessThanOrEqual(
    Math.max(...[...FLOOR_SIZES.values()].map((s) => s.height)),
  );
  // A cell no forest reaches is never offered, and holds nothing if asked.
  expect(field.place(0, 0).length).toBe(0);
});

test("a kind of dressing gathers in drifts: thick in places, thin between", () => {
  // The west wood in 20 m squares: how unevenly the ferns fall among them,
  // as the variance of a square's count over its mean (1 for an even
  // scatter, whatever its density).
  const spread = (placed: SceneryPlacement) => {
    const squares = new Map<string, number>();
    const square = (x: number, y: number) =>
      `${Math.floor((x - 700) / 20)},${Math.floor((y - 880) / 20)}`;
    for (let x = 700; x < 880; x += 20)
      for (let y = 880; y < 1040; y += 20) squares.set(square(x, y), 0);
    for (const p of pieces(placed)) {
      const key = square(p.x, p.y);
      if (p.appearance === "floor_fern" && squares.has(key))
        squares.set(key, squares.get(key)! + 1);
    }
    const counts = [...squares.values()];
    const mean = counts.reduce((sum, n) => sum + n, 0) / counts.length;
    return counts.reduce((sum, n) => sum + (n - mean) ** 2, 0) / counts.length / mean;
  };
  const kinds = DRESSING.kinds.map((k) => ({ ...k, drift: 0 }));
  const even = {
    ...biome,
    forest_floor: { ...biome.forest_floor, dressing: { ...DRESSING, kinds } },
  };
  const { site } = siteOf(view);
  expect(spread(placement)).toBeGreaterThan(2 * spread(placeScenery(site, even, PLACED)));
});

test("no piece of dressing is drawn larger than its appearance, and a biome may not ask for it", () => {
  // The appearance is what the asset validator holds under a man's waist.
  const dressed = pieces(placement);
  for (const p of dressed) expect(p.scale, JSON.stringify(p)).toBeLessThanOrEqual(1);
  // Pieces of a kind differ in size: no two clones side by side by rule.
  const ferns = dressed.filter((p) => p.appearance === "floor_fern").map((p) => p.scale);
  expect(Math.max(...ferns) - Math.min(...ferns)).toBeGreaterThan(0.3);
  const tall = structuredClone(summer) as unknown as Biome;
  (tall.forest_floor.dressing.kinds[0] as { scale: readonly number[] }).scale = [0.8, 1.3];
  expect(() => validateBiome(tall, "summer")).toThrow(
    /summer\.forest_floor\.dressing\.kinds\[0\]\.scale/,
  );
});

// ------------------------------------------------------------------ tree lines

test("a tree line's crowns are as wide as a wood's, and lie within a fog cell of the simulation's foliage", () => {
  // A strip narrower than one crown, bent once: a hedgerow between fields.
  const strip = {
    kind: "stroke",
    points: [
      [30, 40],
      [140, 40],
      [220, 100],
    ],
    width_m: 10,
  };
  const map = {
    size: [256, 128],
    height_grid_m: 4,
    fog_cell_m: 8,
    slope_cutoff_deg: 35,
    forests: [{ shape: strip }],
  };
  const world = new WorldView(JSON.stringify(map), JSON.stringify(TEST_RULES));
  try {
    const { worldExports, site } = siteOf(world);
    const placed = placeScenery(site, biome, PLACED);
    const line = trees(placed.forest);
    expect(line.length).toBeGreaterThan(15);
    // The simulation's foliage: the fog cells some trunk's canopy covers.
    const [nx, ny, cellM] = worldExports.foliage;
    const foliage = new Set<number>();
    for (let o = 3; o < worldExports.foliage.length; o += 4)
      foliage.add(worldExports.foliage[o + 1] * nx + worldExports.foliage[o]);
    expect(foliage.size).toBeGreaterThan(20);
    const near = (x: number, y: number) => {
      const [i, j] = [Math.floor(x / cellM), Math.floor(y / cellM)];
      for (let dj = -1; dj <= 1; dj++)
        for (let di = -1; di <= 1; di++) {
          const [ci, cj] = [i + di, j + dj];
          if (ci >= 0 && cj >= 0 && ci < nx && cj < ny && foliage.has(cj * nx + ci)) return true;
        }
      return false;
    };
    const [lo, hi] = biome.trees.forest.girth;
    let overhang = 0;
    for (const t of line) {
      // No tree is narrowed to the strip: each is as wide as in a wood.
      expect(t.girth, JSON.stringify(t)).toBeGreaterThanOrEqual(lo - 1e-6);
      expect(t.girth, JSON.stringify(t)).toBeLessThanOrEqual(hi + 1e-6);
      for (let k = 0; k < 8; k++) {
        const a = (k * Math.PI) / 4;
        const [x, y] = [t.x + t.radius * Math.cos(a), t.y + t.radius * Math.sin(a)];
        expect(near(x, y), JSON.stringify(t)).toBe(true);
        if (!forestGround(world, x, y)) overhang++;
      }
    }
    // The crowns hang out over the fields beside the strip, as the foliage
    // does.
    expect(overhang).toBeGreaterThan(line.length);
    // And no foliage is left undrawn: every foliage cell has a drawn tree
    // within the canopy's radius of its centre.
    const canopyRadius = game.forests.rule.canopy_radius_m;
    for (const cell of foliage) {
      const [cx, cy] = [((cell % nx) + 0.5) * cellM, (Math.floor(cell / nx) + 0.5) * cellM];
      expect(
        line.some((t) => Math.hypot(t.x - cx, t.y - cy) <= canopyRadius + 1e-3),
        `${cx}, ${cy}`,
      ).toBe(true);
    }
  } finally {
    world.free();
  }
});

const UNDERSTOREY = biome.trees.understorey;

/** Every shrub of a placement's understorey: its foot, its reach from it and
 *  its top above it. */
function shrubs(placed: SceneryPlacement) {
  const data = placed.understorey;
  const size = PLACED.get(UNDERSTOREY.appearance)!;
  const out: { x: number; y: number; reach: number; top: number; appearance: string }[] = [];
  for (let o = 0; o < data.length; o += TREE_FLOATS)
    out.push({
      x: data[o + TREE_FIELD.x],
      y: data[o + TREE_FIELD.y],
      reach: size.radius * data[o + TREE_FIELD.scaleXY],
      top: size.height * data[o + TREE_FIELD.scaleZ],
      appearance: placed.kinds[data[o + TREE_FIELD.kind]],
    });
  return out;
}

/** Whether the simulation finds open ground at (x, y): no paving, no water. */
const onGround = (world: WorldView, x: number, y: number) =>
  layout.surfaceKinds[world.surface_at(x, y)[5]] === "ground";

test("a tree line carries shrubs along its whole length, inside the strip and its foliage, and a wood carries none", () => {
  // A strip between fields, bent once, crossed by a road; and a wood.
  const strip = {
    kind: "stroke",
    points: [
      [30, 40],
      [140, 40],
      [220, 100],
    ],
    width_m: 12,
  };
  const ring = [
    [20, 150],
    [120, 150],
    [120, 190],
    [70, 190],
    [70, 230],
    [20, 230],
  ];
  const road = {
    kind: "road",
    shape: {
      kind: "stroke",
      points: [
        [80, 0],
        [80, 120],
      ],
      width_m: 8,
    },
  };
  const map = {
    size: [256, 256],
    height_grid_m: 4,
    fog_cell_m: 8,
    slope_cutoff_deg: 35,
    surfaces: [road],
    forests: [{ shape: strip }, { shape: { kind: "polygon", ring } }],
  };
  const world = new WorldView(JSON.stringify(map), JSON.stringify(TEST_RULES));
  try {
    const { worldExports, site } = siteOf(world);
    const placed = placeScenery(site, biome, PLACED);
    const line = shrubs(placed);
    expect(line.length).toBeGreaterThan(40);
    const [nx, , cellM] = worldExports.foliage;
    const foliage = new Set<number>();
    for (let o = 3; o < worldExports.foliage.length; o += 4)
      foliage.add(worldExports.foliage[o + 1] * nx + worldExports.foliage[o]);
    // As a crown is held: in a fog cell with foliage, or beside one.
    const underFoliage = (x: number, y: number) => {
      const [i, j] = [Math.floor(x / cellM), Math.floor(y / cellM)];
      for (let dj = -1; dj <= 1; dj++)
        for (let di = -1; di <= 1; di++)
          if (i + di >= 0 && i + di < nx && foliage.has((j + dj) * nx + i + di)) return true;
      return false;
    };
    const [stroke, wood] = site.forests;
    expect([stroke.kind, wood.kind]).toEqual(["stroke", "polygon"]);
    const tallest = PLACED.get(UNDERSTOREY.appearance)!.height * UNDERSTOREY.height[1];
    for (const s of line) {
      const where = JSON.stringify(s);
      expect(s.appearance, where).toBe(UNDERSTOREY.appearance);
      // All of it stands on the strip's own ground: nothing is drawn wider
      // than the forest the simulation blocks sight with, and none in the wood.
      expect(forestInside(stroke, s.x, s.y), where).toBeGreaterThanOrEqual(s.reach - 1e-3);
      expect(forestInside(wood, s.x, s.y), where).toBeLessThan(0);
      expect(underFoliage(s.x, s.y), where).toBe(true);
      // A hedge's height, by the biome's row: well under the crowns.
      expect(s.top, where).toBeLessThanOrEqual(tallest + 1e-3);
      expect(s.top, where).toBeLessThan(game.forests.rule.canopy_height_m / 2);
      // Off the road and under the foliage, by its whole reach.
      for (let k = 0; k < 8; k++) {
        const a = (k * Math.PI) / 4;
        const [x, y] = [s.x + s.reach * Math.cos(a), s.y + s.reach * Math.sin(a)];
        expect(onGround(world, x, y), where).toBe(true);
        expect(underFoliage(x, y), where).toBe(true);
      }
    }
    // Along each stretch longer than the strip is wide, clear of its ends
    // and of the road, a shrub is never farther than two spacings away.
    const runs = [
      [strip.points[0], strip.points[1]],
      [strip.points[1], strip.points[2]],
    ];
    let walked = 0;
    for (const [[ax, ay], [bx, by]] of runs) {
      const length = Math.hypot(bx - ax, by - ay);
      expect(length).toBeGreaterThan(strip.width_m);
      for (let t = strip.width_m; t <= length - strip.width_m; t += 2) {
        const [x, y] = [ax + ((bx - ax) * t) / length, ay + ((by - ay) * t) / length];
        if (Math.abs(x - 80) < road.shape.width_m / 2 + 2 * UNDERSTOREY.spacing_m) continue;
        walked++;
        expect(
          line.some((s) => Math.hypot(s.x - x, s.y - y) <= 2 * UNDERSTOREY.spacing_m),
          `${x}, ${y}`,
        ).toBe(true);
      }
    }
    expect(walked).toBeGreaterThan(50);
    // The same seed plants the same shrubs.
    expect(shrubs(placeScenery(site, biome, PLACED))).toEqual(line);
  } finally {
    world.free();
  }
});

test("the village, which has no tree line, has no understorey", () => {
  expect(placement.understorey.length).toBe(0);
});

// ------------------------------------------------------------------ lone trees

test("a tree body outside every forest is drawn as a tree of its own height, of a species the biome lets stand alone", () => {
  const lone = biome.trees.lone;
  const body = { yaw: 0, half_extents: [0.35, 0.35, 5] };
  const map = {
    size: [200, 200],
    height_grid_m: 4,
    fog_cell_m: 8,
    slope_cutoff_deg: 35,
    props: [40, 64, 88, 112].map((x) => ({ kind: "street_tree", center: [x, 40], ...body })),
    forests: [
      {
        shape: {
          kind: "polygon",
          ring: [
            [20, 100],
            [180, 100],
            [180, 180],
            [20, 180],
          ],
        },
      },
    ],
  };
  const world = new WorldView(JSON.stringify(map), JSON.stringify(TEST_RULES));
  try {
    const { site } = siteOf(world);
    const placed = placeScenery(site, biome, PLACED);
    const drawn = trees(placed.forest).map((t, i) => ({
      ...t,
      appearance: placed.kinds[placed.forest[i * TREE_FLOATS + TREE_FIELD.kind]],
    }));
    const street = drawn.filter((t) => t.y < 100);
    expect(street.map((t) => [t.x, t.y])).toEqual(map.props.map((p) => p.center));
    for (const t of street) {
      const where = JSON.stringify(t);
      expect(lone.species, where).toContain(t.appearance);
      // No taller than its body, and no wider than the biome's row.
      const height = t.top - t.z;
      expect(height, where).toBeLessThanOrEqual(2 * body.half_extents[2] + 0.06);
      expect(height, where).toBeGreaterThanOrEqual(2 * body.half_extents[2] * lone.top[0]);
      expect(t.girth, where).toBeGreaterThanOrEqual(lone.girth[0] - 1e-6);
      expect(t.girth, where).toBeLessThanOrEqual(lone.girth[1] + 1e-6);
    }
    // The wood's trees are the forest rule's, as before.
    expect(drawn.length - street.length).toBeGreaterThan(30);
    // A species the biome's own list lacks is refused by name.
    const palm = { ...biome, trees: { ...biome.trees, lone: { ...lone, species: ["tree_palm"] } } };
    expect(() => validateBiome(structuredClone(palm), "summer")).toThrow(
      /summer\.trees\.lone\.species\[0\].*tree_palm/,
    );
  } finally {
    world.free();
  }
});

// ------------------------------------------------------------ a generated map

test(
  "on a generated map every tree line carries its shrubs and every street tree is a drawn tree",
  () => {
    const fixture = (path: string) =>
      readFileSync(new URL(`../../fixtures/${path}`, import.meta.url), "utf8");
    const documents = {
      presets: fixture("map-presets.json"),
      templates: fixture("prototype-building-templates.json"),
      rules: JSON.stringify(TEST_RULES),
    };
    const request = generationRequest(
      generator,
      { type: "mixed", size: "medium", seed: "2" },
      documents,
      generated.limits,
    );
    const { map } = JSON.parse(
      generator.generate_map(
        JSON.stringify(request),
        documents.presets,
        documents.templates,
        documents.rules,
      ),
    ).result as { map: { props: { kind: string }[] } };
    const world = new WorldView(JSON.stringify(map), JSON.stringify(TEST_RULES));
    try {
      const { site } = siteOf(world);
      const placed = placeScenery(site, biome, PLACED);
      const lines = site.forests.filter((f) => f.kind === "stroke");
      expect(lines.length).toBeGreaterThan(5);
      const hedge = shrubs(placed);
      for (const line of lines) {
        let length = 0;
        for (let o = 0; o < line.strokes.length; o += FOREST_STROKE_FLOATS)
          length += Math.hypot(
            line.strokes[o + 2] - line.strokes[o],
            line.strokes[o + 3] - line.strokes[o + 1],
          );
        // Two rows of them, less the gaps and the ends: one a row every two
        // spacings at the least.
        const along = hedge.filter((s) => forestInside(line, s.x, s.y) >= 0).length;
        expect(along, `${length} m`).toBeGreaterThan(length / (2 * UNDERSTOREY.spacing_m));
      }
      // Every tree body is one drawn tree: the forests' trunks and the streets'.
      const street = map.props.filter((p) => p.kind === "street_tree").length;
      expect(street).toBeGreaterThan(100);
      expect(placed.forest.length / TREE_FLOATS).toBe(site.trunkIds.length);
      const alone = new Set(biome.trees.lone.species);
      const lone = named(placed, placed.forest).slice(-street);
      expect(lone.every((t) => alone.has(t.appearance))).toBe(true);
    } finally {
      world.free();
    }
  },
  WHOLE_MAP_MS,
);
