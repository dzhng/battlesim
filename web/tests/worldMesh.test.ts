// @vitest-environment node
import { expect, test } from "vitest";
import { buildWorldMeshes, type WorldLayout } from "@packages/battle-renderer/src/worldMesh.ts";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh.ts";

const layout: WorldLayout = {
  surfaceKinds: ["ground", "road", "water", "bridge"],
  propKinds: ["building", "wall", "crate", "trunk", "bridgedeck", "wreck", "ruin"],
  blockingPropKinds: {
    infantry: ["building", "wall", "crate", "ruin"],
    vehicle: ["building", "wall", "crate", "wreck", "ruin"],
  },
  flags: { forest: 1, blocked: 2 },
  propStride: 9,
  areaStride: 5,
  propFields: ["id", "kind", "x", "y", "yaw", "hx", "hy", "hz", "baseZ"],
  areaFields: ["x", "y", "w", "h", "z"],
};

const exports = {
  positions: Float32Array.of(0, 0, 1.25, 4, 0, 2.5, 4, 4, 3.75, 0, 4, 0.5),
  indices: Uint32Array.of(0, 1, 2, 0, 2, 3),
  // Triangle 0 open ground, triangle 1 blocked water.
  triangleSurfaces: Uint8Array.of(0, 0, 2, 2),
  props: new Float32Array(0),
  water: new Float32Array(0),
  forests: new Float32Array(0),
};

test("terrain triangles are the exported vertices in exported index order", () => {
  const { opaque } = buildWorldMeshes(exports, layout, "surface");
  // Terrain comes first; a boundary skirt follows it.
  expect(opaque.length / VERTEX_FLOATS).toBeGreaterThan(exports.indices.length);
  exports.indices.forEach((vertex, k) => {
    const drawn = Array.from(opaque.subarray(k * VERTEX_FLOATS, k * VERTEX_FLOATS + 3));
    expect(drawn).toEqual(Array.from(exports.positions.subarray(vertex * 3, vertex * 3 + 3)));
  });
});

test("the traversal overlay colours each triangle by its exported blocked flag", () => {
  const surface = buildWorldMeshes(exports, layout, "surface").opaque;
  const traversal = buildWorldMeshes(exports, layout, "traversal").opaque;
  const color = (mesh: Float32Array, vertex: number) =>
    Array.from(mesh.subarray(vertex * VERTEX_FLOATS + 6, vertex * VERTEX_FLOATS + 10));
  // All three corners of a triangle share its colour; the blocked triangle differs.
  expect(color(traversal, 1)).toEqual(color(traversal, 0));
  expect(color(traversal, 3)).not.toEqual(color(traversal, 0));
  expect(color(surface, 3)).not.toEqual(color(traversal, 3));
});
