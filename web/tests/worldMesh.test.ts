// @vitest-environment node
import { expect, test } from "vitest";
import {
  apartKinds,
  buildWorldLayers,
  worldStructures,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh.ts";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";
import { validateStandIns } from "@packages/battle-renderer/src/models/propAppearance.ts";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";
import type { StaticBundle } from "@packages/scene-assets/src/schema.ts";
import { STAND_IN_KIT, STAND_IN_MODULE } from "@packages/scene-assets/src/standInKit.ts";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import summer from "@fixtures/biomes/summer.json";

const biome = validateBiome(summer as unknown as Biome);

const layout: WorldLayout = {
  surfaceKinds: ["ground", "road", "water", "bridge"],
  surfaceAreaKinds: ["road", "country_road", "dirt_track", "paving"],
  roadAreaKinds: ["road", "country_road", "dirt_track"],
  propKinds: ["building", "wall", "crate", "trunk", "bridge_deck", "heavy_wreck", "ruin"],
  unitKinds: [],
  unitAppearance: {},
  blockingPropKinds: {
    infantry: ["building", "wall", "crate", "ruin"],
    vehicle: ["building", "wall", "crate", "heavy_wreck", "ruin"],
  },
  occludingPropKinds: ["building", "wall", "ruin"],
  movablePropKinds: ["crate", "heavy_wreck"],
  destroyablePropKinds: ["building", "wall", "crate", "trunk", "heavy_wreck"],
  propAppearance: {
    building: { drawn_by: "building", map_only: true },
    wall: { drawn_by: "wall", modular: true },
    crate: { drawn_by: "crate" },
    trunk: { drawn_by: "forest" },
    bridge_deck: { drawn_by: "bridge_deck", map_only: true },
    heavy_wreck: { drawn_by: "wreck" },
    ruin: { drawn_by: "ruin" },
  },
  flags: { forest: 1, blocked: 2 },
  propStride: 10,
  sightGapStride: 11,
  sightGapFields: ["x", "y", "yaw", "hx", "hy", "base", "top", "aLo", "aHi", "bLo", "bHi"],
  limbBits: 16,
  areaStride: 5,
  propFields: ["idLo", "idHi", "kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ"],
  areaFields: ["x", "y", "w", "h", "z"],
  surfaceStrokeStride: 7,
  surfaceStrokeFields: ["ax", "ay", "bx", "by", "halfWidth", "kind", "cuts"],
  strokeCuts: { a: 1, b: 2 },
  surfaceRunStride: 4,
  surfaceRunFields: ["ax", "ay", "bx", "by"],
  surfaceTriangleStride: 7,
  surfaceBoundaryStride: 5,
  surfaceTriangleFields: ["ax", "ay", "bx", "by", "cx", "cy", "kind"],
  surfaceBoundaryFields: ["ax", "ay", "bx", "by", "kind"],
  riverStride: 11,
  riverFields: [
    "ax",
    "ay",
    "bx",
    "by",
    "halfA",
    "halfB",
    "gradeA",
    "gradeB",
    "bankA",
    "bankB",
    "surfaceZ",
  ],
  riverRunStride: 4,
  riverRunFields: ["ax", "ay", "bx", "by"],
  forestTrunkRangeStride: 2,
  forestTrunkRangeFields: ["firstProp", "onePastProp"],
  forestMetadataStride: 3,
  forestMetadataFields: ["id", "canopy", "shapeKind"],
  forestShapeKinds: ["rectangle", "stroke", "polygon"],
  forestStrokeStride: 7,
  forestStrokeFields: ["ax", "ay", "bx", "by", "halfWidth", "id", "cuts"],
  forestTriangleStride: 7,
  forestTriangleFields: ["ax", "ay", "bx", "by", "cx", "cy", "id"],
  forestBoundaryStride: 5,
  forestBoundaryFields: ["ax", "ay", "bx", "by", "id"],
};

const heights = new Float32Array(16 * 16);
heights[0] = 1.25;
heights[1] = 2.5;
heights[16] = 0.5;
heights[17] = 3.75;
const exports = {
  extents: { playable: [0, 0, 4, 4], physical: [0, 0, 4, 4], rendered: [0, 0, 4, 4] } as const,
  terrain: {
    nx: 2,
    ny: 2,
    spacing: 4,
    pageSize: 16,
    minHeight: 0.5,
    pageIds: Uint32Array.of(0),
    heights,
  },
  positions: Float32Array.of(0, 0, 1.25, 4, 0, 2.5, 4, 4, 3.75, 0, 4, 0.5),
  indices: Uint32Array.of(0, 1, 2, 0, 2, 3),
  // Triangle 0 open ground, triangle 1 blocked water.
  triangleSurfaces: Uint8Array.of(0, 0, 2, 2),
  // One crate, standing on a ledge above the ground.
  props: Float32Array.of(7, 0, 2, 2, 2, 0, 0.5, 0.5, 0.5, 5),
  buildings: { catalogueHash: null, regionalFamily: null, buildings: [] },
  rivers: new Float32Array(0),
  riverRuns: new Float32Array(0),
  sightGaps: new Float32Array(0),
  forests: new Float32Array(0),
  forestTrunkRanges: new Uint32Array(0),
  forestRectIds: new Uint32Array(0),
  forestMetadata: new Float32Array(0),
  forestStrokes: new Float32Array(0),
  forestTriangles: new Float32Array(0),
  forestBoundaries: new Float32Array(0),
  foliage: new Float32Array(0),
  surfaceStrokes: new Float32Array(0),
  surfaceRuns: new Float32Array(0),
  surfaceTriangles: new Float32Array(0),
  surfaceBoundaries: new Float32Array(0),
};

test("the terrain layer is the exported triangles, in exported index order, and nothing else", () => {
  const { terrain } = buildWorldLayers(exports, layout, biome, "surface");
  expect(terrain.mesh.length / VERTEX_FLOATS).toBe(exports.indices.length);
  exports.indices.forEach((vertex, k) => {
    const drawn = Array.from(terrain.mesh.subarray(k * VERTEX_FLOATS, k * VERTEX_FLOATS + 3));
    expect(drawn).toEqual(Array.from(exports.positions.subarray(vertex * 3, vertex * 3 + 3)));
  });
});

test("the skirt the map stands on is a layer of its own; props are boxes only in the traversal view", () => {
  const heights = (mesh: Float32Array) => {
    const zs: number[] = [];
    for (let v = 0; v < mesh.length; v += VERTEX_FLOATS) zs.push(mesh[v + 2]);
    return zs;
  };
  // The surface view draws the skirt, below the lowest ground, and no box:
  // its props are appearances (`structures`), here with none installed.
  const surface = buildWorldLayers(exports, layout, biome, "surface");
  expect(Math.min(...heights(surface.props))).toBeLessThan(0.5);
  expect(Math.max(...heights(surface.props))).toBeLessThan(6);
  expect(surface.structures).toEqual([]);
  // The traversal view shows what blocks: the crate's box, its top at 6.
  const traversal = buildWorldLayers(exports, layout, biome, "traversal");
  expect(Math.max(...heights(traversal.props))).toBeCloseTo(6);
});

test("the traversal overlay tints each triangle by its exported blocked flag, over the biome", () => {
  const surface = buildWorldLayers(exports, layout, biome, "surface").terrain.mesh;
  const traversal = buildWorldLayers(exports, layout, biome, "traversal").terrain.mesh;
  const tint = (mesh: Float32Array, vertex: number) =>
    Array.from(mesh.subarray(vertex * VERTEX_FLOATS + 6, vertex * VERTEX_FLOATS + 10));
  // All three corners of a triangle share its tint; the blocked triangle differs.
  expect(tint(traversal, 1)).toEqual(tint(traversal, 0));
  expect(tint(traversal, 3)).not.toEqual(tint(traversal, 0));
  // The traversal tint replaces the biome; the surface view leaves the biome alone.
  expect(tint(traversal, 3)[3]).toBe(1);
  expect(tint(surface, 3)[3]).toBe(0);
});

test("a battle draws apart what can move and, when asked, what the integrity column says can be destroyed", () => {
  // Trees stay the scenery's: a felled one is dropped where its ground is cleared.
  expect(apartKinds(layout, false).sort()).toEqual(["crate", "heavy_wreck"]);
  expect(apartKinds(layout, true).sort()).toEqual(["building", "crate", "heavy_wreck", "wall"]);
});

test("a static prop with no art is drawn in the world as its stand-in box, as one drawn apart is", () => {
  // The crate's kind as a static one: nothing shoves or destroys it, so the
  // world draws it, not the battle's known props.
  const still: WorldLayout = { ...layout, movablePropKinds: [], destroyablePropKinds: [] };
  const bundle: StaticBundle = {
    kind: "static",
    states: [{ name: STAND_IN_MODULE, tiers: [], bounds: { min: [0, 0, 0], max: [1, 1, 1] } }],
    materials: [],
    textures: [],
    bounds: { min: [0, 0, 0], max: [1, 1, 1] },
  };
  const kit: InstalledAppearances = {
    generation: 1,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    skeletons: new Map(),
    onRequest: new Map(),
    appearances: new Map([
      [
        STAND_IN_KIT,
        { unit: "kit", scenery: null, footprint: null, mounts: null, regionalFamily: null, bundle },
      ],
    ]),
  } as unknown as InstalledAppearances;
  const standIns = validateStandIns({ tints: { default: [0.5, 0.5, 0.5] } });
  expect(worldStructures(exports, still, [], kit)).toEqual([]);
  const [box, ...rest] = worldStructures(exports, still, [], kit, standIns);
  expect(rest).toEqual([]);
  expect(box.appearance).toBe(STAND_IN_KIT);
  expect([box.x, box.y, box.z]).toEqual([2, 2, 5]);
  expect(box.scale).toEqual([1, 1, 1]);
});
