// @vitest-environment node
import { expect, test } from "vitest";
import { buildWorldLayers, type WorldLayout } from "@packages/battle-renderer/src/worldMesh.ts";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";
import summer from "@fixtures/biomes/summer.json";
import { loadPresentation } from "@web/battle/sim/presentation";

// The ground's shapes are resolved by the presentation module.
await loadPresentation();

const biome = summer;

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
  flags: { forest: 1, blocked: 2 },
  propStride: 10,
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
  riverStride: 12,
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
    "river",
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
  swamps: new Float32Array(0),
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

test("props are boxes only in the traversal view", () => {
  const heights = (mesh: Float32Array) => {
    const zs: number[] = [];
    for (let v = 0; v < mesh.length; v += VERTEX_FLOATS) zs.push(mesh[v + 2]);
    return zs;
  };
  // The surface view supplies no extra faces: its props are appearances,
  // the scene's presentation's to draw.
  const surface = buildWorldLayers(exports, layout, biome, "surface");
  expect(surface.extraFaces).toEqual(new Float32Array(0));
  expect(surface.terrain.input.props).toBe(true);
  // The traversal view shows what blocks: the crate's box, its top at 6.
  const traversal = buildWorldLayers(exports, layout, biome, "traversal");
  expect(Math.max(...heights(traversal.extraFaces))).toBeCloseTo(6);
  expect(traversal.terrain.input.props).toBe(false);
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
