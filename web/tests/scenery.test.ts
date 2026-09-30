// @vitest-environment node
// Scenery placement against the simulation's own geometry: the drawn forest
// is the simulation's forest volume (every crown inside a forest rect and
// under its canopy over the simulation's ground, the rect's foliage covered
// by crowns, the simulation's trunks each a drawn tree), and scenery past the
// map stays off it.
import { VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import type { WorldExports, WorldLayout } from "@packages/battle-renderer/src/worldMesh.ts";
import { buildTerrainSurface } from "@packages/battle-renderer/src/terrain/terrainSurface.ts";
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
import village from "@fixtures/village.json";

const biome = validateBiome(summer as unknown as Biome);
/** Unscaled appearance sizes: the loader reads them from the bundles. */
const SIZES = new Map<string, KindSize>([
  ["tree_broadleaf", { height: 11, radius: 5.6 }],
  ["tree_spreading", { height: 10, radius: 6.1 }],
  ["tree_tall", { height: 11.8, radius: 3.8 }],
  ["hedge_shrub", { height: 2.8, radius: 3.2 }],
]);

let view: WorldView;
let layout: WorldLayout;
let exports: WorldExports;
let placement: SceneryPlacement;

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  layout = JSON.parse(world_layout(JSON.stringify(VILLAGE_RULES))) as WorldLayout;
  view = new WorldView(JSON.stringify(village.map), JSON.stringify(VILLAGE_RULES));
  exports = {
    terrain: {
      ...JSON.parse(view.terrain_grid()),
      pageIds: view.terrain_page_ids(),
      heights: view.terrain_heights(),
    },
    positions: view.terrain_positions(),
    indices: view.terrain_indices(),
    triangleSurfaces: view.terrain_triangle_surfaces(),
    props: view.props(),
    buildings: JSON.parse(view.buildings()),
    water: view.water(),
    forests: view.forests(),
    foliage: view.foliage(),
    roads: view.roads(),
  };
  const site = scenerySite(exports, layout, buildTerrainSurface(exports, layout, biome));
  placement = placeScenery(site, biome, SIZES);
});

interface Tree {
  x: number;
  y: number;
  z: number;
  radius: number;
  top: number;
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
    });
  }
  return out;
}
const ground = (x: number, y: number) => view.surface_at(x, y)[0];
const forests = () =>
  village.map.forests.map((f) => ({
    rect: f.rect as [number, number, number, number],
    canopy: f.canopy_height_m,
  }));

test("every forest tree's crown stays inside a forest rect, under its canopy over the simulation's ground", () => {
  const drawn = trees(placement.forest);
  expect(drawn.length).toBeGreaterThan(400);
  for (const t of drawn) {
    const forest = forests().find(
      ({ rect: [x, y, w, h] }) =>
        t.x - t.radius >= x - 1e-3 &&
        t.x + t.radius <= x + w + 1e-3 &&
        t.y - t.radius >= y - 1e-3 &&
        t.y + t.radius <= y + h + 1e-3,
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

/** Whether (x, y) is on a road's surface. */
function onRoad(x: number, y: number): boolean {
  const r = exports.roads;
  for (let o = 0; o < r.length; o += layout.roadStride) {
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
  const kinds = new Set<string>();
  for (let o = 0; o < placement.backdrop.length; o += TREE_FLOATS)
    kinds.add(placement.kinds[placement.backdrop[o + TREE_FIELD.kind]]);
  expect([...kinds].sort()).toEqual([...SIZES.keys()].sort());
});

test("placement is deterministic", () => {
  const site = scenerySite(exports, layout, buildTerrainSurface(exports, layout, biome));
  const again = placeScenery(site, biome, SIZES);
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
