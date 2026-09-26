// @vitest-environment node
import { expect, test } from "vitest";
import { buildWorldLayers, type WorldLayout } from "@packages/battle-renderer/src/worldMesh.ts";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import summer from "@fixtures/biomes/summer.json";

const biome = validateBiome(summer as unknown as Biome);

const layout: WorldLayout = {
  surfaceKinds: ["ground", "road", "water", "bridge"],
  propKinds: ["building", "wall", "crate", "trunk", "bridgedeck", "wreck", "ruin"],
  blockingPropKinds: {
    infantry: ["building", "wall", "crate", "ruin"],
    vehicle: ["building", "wall", "crate", "wreck", "ruin"],
  },
  occludingPropKinds: ["building", "wall", "crate", "wreck", "ruin"],
  flags: { forest: 1, blocked: 2 },
  propStride: 9,
  areaStride: 5,
  propFields: ["id", "kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ"],
  areaFields: ["x", "y", "w", "h", "z"],
  roadStride: 5,
  roadFields: ["ax", "ay", "bx", "by", "halfWidth"],
};

const exports = {
  positions: Float32Array.of(0, 0, 1.25, 4, 0, 2.5, 4, 4, 3.75, 0, 4, 0.5),
  indices: Uint32Array.of(0, 1, 2, 0, 2, 3),
  // Triangle 0 open ground, triangle 1 blocked water.
  triangleSurfaces: Uint8Array.of(0, 0, 2, 2),
  // One crate, standing on a ledge above the ground.
  props: Float32Array.of(7, 2, 2, 2, 0, 0.5, 0.5, 0.5, 5),
  water: new Float32Array(0),
  forests: new Float32Array(0),
  roads: new Float32Array(0),
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
