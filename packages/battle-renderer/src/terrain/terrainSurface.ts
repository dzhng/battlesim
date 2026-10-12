// TerrainSurface: the ground layer as the renderer draws it. It is the
// simulation's own triangles (exported vertices in exported index order,
// flat-shaded, never resampled or exaggerated: landmine 11) under the biome's
// material, whose tables (the plot split, the roads, forests and rivers
// exactly as the simulation's surface rules define them, so a road is drawn
// precisely where units find road), the ground's meshes, the water and the
// scenery are resolved in Rust (`crates/presentation/src/static_world/ground`),
// which the presentation module must have loaded (`loadPresentation`).
import { Ground, terrain_mesh } from "@wasm/presentation/presentation_wasm.js";
import game from "@fixtures/game.json";
import type summer from "@fixtures/biomes/summer.json";
import type { Mesh, Rgba } from "../mesh";
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

/** The traversal view's colours: ground units can or cannot enter. The
 *  ground's own triangles take them in Rust (`ground::mesh`); the props'
 *  boxes and the labs' marks take them here. */
export const OPEN: Rgba = [0.52, 0.56, 0.5, 1];
export const BLOCKED: Rgba = [0.78, 0.3, 0.26, 1];

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
  const traversal = overlay === "traversal" ? triangleSurfaces : undefined;
  const mesh = terrain_mesh(positions, indices, traversal, layout.flags.blocked) as Mesh;
  const ground = resolveGround(
    exports,
    JSON.stringify(layout),
    biome,
    exports.buildings.regionalFamily,
  );
  return { mesh, ground, biome, grid: overlay === "surface" ? exports.terrain : null };
}
