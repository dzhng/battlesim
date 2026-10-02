// @vitest-environment node
// A mesh's triangles are ordered by how their material is drawn, so each
// pipeline draws one index range: what it must hold is every triangle once,
// in the range of its own surface, and an all-opaque mesh exactly as it was.
import { expect, test } from "vitest";
import { orderSurfaces, surfaceClass } from "@packages/battle-renderer/src/models/surfaceParts";
import type { Coverage, Material } from "@packages/scene-assets/src/schema";

const material = (coverage: Coverage, more: Partial<Material> = {}): Material => ({
  name: coverage.kind,
  base_color: [1, 1, 1, 1],
  metallic: 0,
  roughness: 1,
  tint: 0,
  coverage,
  ...more,
});
const OPAQUE = material({ kind: "opaque" });
const CUTOUT = material({ kind: "cutout", cutoff: 0.5 });

/** A mesh of `vertices` vertices whose draws are runs of triangles, each
 *  triangle (3k, 3k + 1, 3k + 2) in turn. */
function mesh(vertices: number, draws: { material: number; triangles: number }[]) {
  const indices = Uint16Array.from({ length: vertices }, (_, i) => i);
  let first = 0;
  return {
    vertices,
    indices,
    draws: draws.map((d) => {
      const draw = { material: d.material, first, count: d.triangles * 3 };
      first += draw.count;
      return draw;
    }),
  };
}

const triangles = (indices: ArrayLike<number>, first: number, count: number) =>
  Array.from({ length: count / 3 }, (_, t) =>
    [0, 1, 2].map((k) => indices[first + t * 3 + k]).join(","),
  );

test("an all-opaque mesh keeps its index order, and its opaque part is the whole of it", () => {
  const classes = [OPAQUE, OPAQUE].map(surfaceClass);
  const m = mesh(12, [
    { material: 0, triangles: 1 },
    { material: 1, triangles: 3 },
  ]);
  const { indices, parts } = orderSurfaces([m], (i) => classes[i]);
  expect(Array.from(indices)).toEqual(Array.from(m.indices));
  expect(parts.opaque).toEqual({ first: 0, count: 12 });
  expect(parts.cutout.count).toBe(0);
});

test("a cutout between opaque draws: each triangle once, the cutout's in its own range", () => {
  const classes = [OPAQUE, CUTOUT, OPAQUE].map(surfaceClass);
  const m = mesh(18, [
    { material: 0, triangles: 2 },
    { material: 1, triangles: 1 },
    { material: 2, triangles: 3 },
  ]);
  const { indices, parts } = orderSurfaces([m], (i) => classes[i]);
  const all = triangles(indices, 0, indices.length);
  expect([...all].sort()).toEqual(triangles(m.indices, 0, 18).sort());
  // The cutout is the discarding pipelines' range, and nothing else is in it.
  expect(triangles(indices, parts.cutout.first, parts.cutout.count)).toEqual(["6,7,8"]);
  expect(triangles(indices, parts.opaque.first, parts.opaque.count).sort()).toEqual(
    ["0,1,2", "3,4,5", "9,10,11", "12,13,14", "15,16,17"].sort(),
  );
});

test("meshes merged into one buffer index their own vertices", () => {
  const classes = [OPAQUE, CUTOUT].map(surfaceClass);
  const a = mesh(3, [{ material: 0, triangles: 1 }]);
  const b = mesh(6, [
    { material: 1, triangles: 1 },
    { material: 0, triangles: 1 },
  ]);
  const { indices, parts } = orderSurfaces([a, b], (i) => classes[i]);
  // b's vertices follow a's three.
  expect(triangles(indices, parts.cutout.first, parts.cutout.count)).toEqual(["3,4,5"]);
  expect(triangles(indices, parts.opaque.first, parts.opaque.count)).toEqual(["0,1,2", "6,7,8"]);
});
