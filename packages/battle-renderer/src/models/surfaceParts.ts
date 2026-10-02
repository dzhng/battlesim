// Which of a mesh's triangles each pipeline draws. A material says how much
// of its surface is there, and whether it is a room behind a window
// (`Coverage`, `Material.interior`, scene-assets); the frame draws each kind
// of surface through its own pipelines, so a mesh's index buffer is ordered
// by kind once, at install, and a pipeline's share of it is one range
// (`SurfaceParts`). A mesh that is opaque throughout keeps its order, and its
// opaque part is the whole of it.
import type { Material, MeshData } from "@packages/scene-assets/src/schema";

/** How a material's surface is drawn, in the order a mesh's indices take:
 *  - `cutout`: there or not, texel by texel. The depth prepass and the sun's
 *    casters cut it, and the colour pass shades the samples the prepass kept;
 *  - `opaque`: all of it there;
 *  - `room`: a wall of the box behind a window. As solid as an opaque surface
 *    to depth and shadow; its colour is a cell of the interior atlas, unlit;
 *  - `blended`: partly there (glass). Drawn after the opaque world over what
 *    is behind it, reading depth and writing none, and casting no shadow. */
export const SURFACE_CLASSES = ["cutout", "opaque", "room", "blended"] as const;
export type SurfaceClass = (typeof SURFACE_CLASSES)[number];

export function surfaceClass(material: Material): SurfaceClass {
  return material.interior === undefined ? material.coverage.kind : "room";
}

export interface IndexRange {
  first: number;
  count: number;
}

/** What a pipeline draws of a mesh: one class, or `solid`, the opaque
 *  surfaces and the rooms together (what the fragment-less depth and caster
 *  pipelines draw: the two are neighbours in the index order). */
export type SurfacePart = SurfaceClass | "solid";
/** Each class's share of a mesh: one range of its indices. */
export type SurfaceParts = Record<SurfaceClass, IndexRange>;

const NOTHING: IndexRange = { first: 0, count: 0 };
const _range: IndexRange = { first: 0, count: 0 };
/** The range of `parts` a pipeline drawing `part` draws, leaving out the
 *  classes `hidden` names. The result is scratch: read it before asking again. */
export function partRange(
  parts: SurfaceParts,
  part: SurfacePart,
  hidden: Readonly<Record<SurfaceClass, boolean>>,
): IndexRange {
  if (part !== "solid") return hidden[part] ? NOTHING : parts[part];
  const opaque = hidden.opaque ? 0 : parts.opaque.count;
  const room = hidden.room ? 0 : parts.room.count;
  _range.first = opaque ? parts.opaque.first : parts.room.first;
  _range.count = opaque + room;
  return _range;
}

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
