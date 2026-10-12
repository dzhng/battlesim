// TerrainSurface: the ground layer as the renderer draws it. It is the
// simulation's own triangles (exported vertices in exported index order,
// flat-shaded, never resampled or exaggerated: landmine 11) under the biome's
// material, whose tables (the plot split, the roads, forests and rivers
// exactly as the simulation's surface rules define them, so a road is drawn
// precisely where units find road), the water's cells and the scenery are
// resolved in Rust (`crates/presentation/src/static_world/ground`), which the
// presentation module must have loaded (`loadPresentation`).
import { vec3 } from "math";
import { triangle3 } from "math/shapes";
import { Ground } from "@wasm/presentation/presentation_wasm.js";
import game from "@fixtures/game.json";
import type summer from "@fixtures/biomes/summer.json";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "../mesh";
import type { WorldExports, WorldLayout, WorldOverlay } from "../worldMesh";
import type { TerrainGrid } from "./terrainGrid";

/** A biome file (`fixtures/biomes/`), as the page reads it: Rust admits it
 *  (`ground::biome`) and resolves the ground from it; the page reads the grass
 *  window's and the scenery's few numbers. */
export type Biome = typeof summer;

export interface TerrainSurface {
  /** Ground triangles in the one vertex format. The vertex colour is a tint
   *  over the biome material, its alpha the tint's weight: 0 draws the biome,
   *  1 the tint alone (the traversal view, render-only fixtures). */
  mesh: Mesh;
  /** The ground Rust resolved: its tables, water and scenery. */
  ground: Ground;
  biome: Biome;
  /** The height grid grass is seated on; null where no grass grows (the
   *  traversal view, render-only patches: where the tint replaces the biome). */
  grid: TerrainGrid | null;
}

/** The traversal view's colours: ground units can or cannot enter. */
export const OPEN: Rgba = [0.52, 0.56, 0.5, 1];
export const BLOCKED: Rgba = [0.78, 0.3, 0.26, 1];
const BIOME: Rgba = [0, 0, 0, 0];

/** The ground of `exports` under `biome`, in its map's region's finish. */
function resolveGround(
  exports: Partial<WorldExports> & Pick<WorldExports, "extents" | "terrain">,
  layout: string,
  biome: Biome,
  family: string | null,
): Ground {
  const none = new Float32Array(0);
  const ids = new Uint32Array(0);
  const { physical, rendered } = exports.extents;
  const { pageIds, heights, ...grid } = exports.terrain;
  return new Ground(
    Float64Array.from([...physical, ...(rendered[0] < 0 ? rendered : [])]),
    JSON.stringify(grid),
    pageIds,
    heights,
    exports.props ?? none,
    exports.foliage ?? none,
    exports.surfaceStrokes ?? none,
    exports.surfaceRuns ?? none,
    exports.surfaceTriangles ?? none,
    exports.surfaceBoundaries ?? none,
    exports.rivers ?? none,
    exports.riverRuns ?? none,
    exports.forests ?? none,
    exports.forestRectIds ?? ids,
    exports.forestMetadata ?? none,
    exports.forestStrokes ?? none,
    exports.forestTriangles ?? none,
    exports.forestBoundaries ?? none,
    exports.forestTrunkRanges ?? ids,
    layout,
    JSON.stringify(biome),
    family ?? undefined,
    JSON.stringify(game.presentation),
  );
}

/** A ground of `map`, its triangles `gridM` wide, holding only `streams`
 *  (none: nothing on it), drawn as `mesh`'s tints: no grass grows on it. */
export function bareTerrain(
  mesh: Mesh,
  map: readonly [number, number, number, number],
  gridM: number,
  biome: Biome,
  streams: Partial<WorldExports> = {},
): TerrainSurface {
  const terrain = {
    nx: 2,
    ny: 2,
    spacing: gridM,
    pageSize: 1,
    minHeight: 0,
    pageIds: new Uint32Array(0),
    heights: new Float32Array(0),
  };
  const extents = { physical: map, rendered: [0, 0, 0, 0] as const, playable: map };
  const layout = JSON.stringify({ propKinds: [], propAppearance: {} });
  const ground = resolveGround({ ...streams, extents, terrain }, layout, biome, null);
  return { mesh, ground, biome, grid: null };
}

/** The ground of the exported world under `biome`, in its map's region's
 *  finish; the traversal view tints each triangle by its exported blocked
 *  flag instead. */
export function buildTerrainSurface(
  exports: WorldExports,
  layout: WorldLayout,
  biome: Biome,
  overlay: WorldOverlay = "surface",
): TerrainSurface {
  const { positions, indices, triangleSurfaces } = exports;
  const mesh = new Float32Array(indices.length * VERTEX_FLOATS);
  const a = vec3.create(),
    b = vec3.create(),
    c = vec3.create(),
    normal = vec3.create();
  const corners = [a, b, c];
  for (let t = 0; t < indices.length; t += 3) {
    const tint =
      overlay === "traversal"
        ? triangleSurfaces[(t / 3) * 2 + 1] & layout.flags.blocked
          ? BLOCKED
          : OPEN
        : BIOME;
    vec3.fromBuffer(a, positions, indices[t] * 3);
    vec3.fromBuffer(b, positions, indices[t + 1] * 3);
    vec3.fromBuffer(c, positions, indices[t + 2] * 3);
    triangle3.normal(normal, a, b, c);
    for (let k = 0; k < 3; k++) {
      const at = (t + k) * VERTEX_FLOATS;
      vec3.toBuffer(mesh, corners[k], at);
      vec3.toBuffer(mesh, normal, at + 3);
      mesh.set(tint, at + 6);
    }
  }
  const ground = resolveGround(
    exports,
    JSON.stringify(layout),
    biome,
    exports.buildings.regionalFamily,
  );
  return { mesh, ground, biome, grid: overlay === "surface" ? exports.terrain : null };
}
