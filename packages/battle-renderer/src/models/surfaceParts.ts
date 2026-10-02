// Which of a mesh's triangles each pipeline draws. A material says how much
// of its surface is there (`Coverage`, scene-assets); the frame draws each
// kind of surface through its own pipelines, so a mesh's index buffer is
// ordered by kind once, at install, and a kind's share of it is one range
// (`SurfaceParts`). A mesh that is opaque throughout keeps its order, and its
// opaque part is the whole of it.
import type { Material, MeshData } from "@packages/scene-assets/src/schema";

/** How a material's surface is drawn, in the order a mesh's indices take:
 *  - `cutout`: there or not, texel by texel. The depth prepass and the sun's
 *    casters cut it, and the colour pass shades the samples the prepass kept;
 *  - `opaque`: everything else. */
export const SURFACE_CLASSES = ["cutout", "opaque"] as const;
export type SurfaceClass = (typeof SURFACE_CLASSES)[number];

export function surfaceClass(material: Material): SurfaceClass {
  return material.coverage.kind === "cutout" ? "cutout" : "opaque";
}

export interface IndexRange {
  first: number;
  count: number;
}

/** Each class's share of a mesh: one range of its indices. */
export type SurfaceParts = Record<SurfaceClass, IndexRange>;

/**
 * The indices of `meshes`, merged into one buffer's (each mesh's vertices
 * follow the last one's, from `vertexBase`) and ordered by surface class, and
 * their parts in a buffer where they start at index `first`. Within a class
 * the draws keep their order.
 */
export function orderSurfaces(
  meshes: readonly { vertices: number; indices: ArrayLike<number>; draws: MeshData["draws"] }[],
  classOf: (material: number) => SurfaceClass,
  vertexBase = 0,
  first = 0,
): { indices: Uint32Array; parts: SurfaceParts } {
  const total = meshes.reduce((n, m) => n + m.draws.reduce((k, d) => k + d.count, 0), 0);
  const indices = new Uint32Array(total);
  const parts = {} as SurfaceParts;
  let at = 0;
  for (const c of SURFACE_CLASSES) {
    const start = at;
    let base = vertexBase;
    for (const mesh of meshes) {
      for (const draw of mesh.draws) {
        if (classOf(draw.material) !== c) continue;
        for (let i = 0; i < draw.count; i++) indices[at + i] = mesh.indices[draw.first + i] + base;
        at += draw.count;
      }
      base += mesh.vertices;
    }
    parts[c] = { first: first + start, count: at - start };
  }
  return { indices, parts };
}
