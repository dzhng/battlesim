// TerrainSurface: the ground layer as the renderer draws it. It is the
// simulation's own triangles (exported vertices in exported index order,
// flat-shaded, never resampled or exaggerated: landmine 11) under the biome's
// material, whose tables (the plot split, the roads, forests and rivers
// exactly as the simulation's surface rules define them, so a road is drawn
// precisely where units find road), the ground's meshes, the water's and the
// scenery are resolved by the scene's presentation (`crates/presentation`'s
// `static_world::ground`) from what is set here (`GroundInput`); the
// presentation module must have loaded (`loadPresentation`).
import { presentationSettings } from "@web/presentationSettings";
import { Presentation, terrain_mesh } from "@wasm/renderer_browser";
import type summer from "@fixtures/biomes/summer.json";
import type { Mesh, Rgba } from "../mesh";
import type { WorldExports, WorldLayout, WorldOverlay } from "../worldMesh";
import type { TerrainGrid } from "./terrainGrid";
import type { FogSource } from "../fog";

/** A biome file (`fixtures/biomes/`), as the page reads it: Rust admits it
 *  (`ground::biome`) and resolves the ground from it; the page reads the grass
 *  window's and the scenery's few numbers. */
export type Biome = typeof summer;

/** What a presentation resolves a map's ground from (`Presentation.set_ground`). */
export interface GroundInput {
  exports: Partial<WorldExports> & Pick<WorldExports, "extents" | "terrain">;
  layout: string;
  biome: Biome;
  family: string | null;
  /** Whether the map's props draw as their appearances (the surface view). */
  props?: boolean;
}

/** Ground queries and lab controls used by browser scene composition. */
export type PresentedGround = Pick<
  Presentation,
  "ground_water_mesh" | "grass_kinds" | "retune_grass"
> &
  FogSource;

export interface TerrainSurface {
  /** Ground triangles in the one vertex format. The vertex colour is a tint
   *  over the biome material, its alpha the tint's weight: 0 draws the biome,
   *  1 the tint alone (the traversal view, render-only fixtures). */
  mesh: Mesh;
  /** What the scene's presentation resolves the ground from. */
  input: GroundInput;
  biome: Biome;
  /** The height grid grass is seated on; null where no grass grows (the
   *  traversal view, render-only patches: where the tint replaces the biome). */
  grid: TerrainGrid | null;
  /** Whether the ground's water surface draws (not in the traversal view). */
  water: boolean;
}

/** The traversal view's colours: ground units can or cannot enter. The
 *  ground's own triangles take them in Rust (`ground::mesh`); the props'
 *  boxes and the labs' marks take them here. */
export const OPEN: Rgba = [0.52, 0.56, 0.5, 1];
export const BLOCKED: Rgba = [0.78, 0.3, 0.26, 1];

/** Sets `input`'s ground on `presentation` (`Presentation.set_ground`). */
export function setGround(presentation: Presentation, input: GroundInput): void {
  const none = new Float32Array(0);
  const ids = new Uint32Array(0);
  const { exports: e, layout, biome, family } = input;
  const { physical, playable, rendered } = e.extents;
  const { pageIds, heights, ...grid } = e.terrain;
  presentation.set_ground(
    Float64Array.from([...physical, ...playable, ...(rendered[0] < 0 ? rendered : [])]),
    JSON.stringify(grid),
    pageIds,
    heights,
    e.props ?? none,
    e.foliage ?? none,
    e.surfaceStrokes ?? none,
    e.surfaceRuns ?? none,
    e.surfaceTriangles ?? none,
    e.surfaceBoundaries ?? none,
    e.rivers ?? none,
    e.riverRuns ?? none,
    e.swamps ?? none,
    e.forests ?? none,
    e.forestRectIds ?? ids,
    e.forestMetadata ?? none,
    e.forestStrokes ?? none,
    e.forestTriangles ?? none,
    e.forestBoundaries ?? none,
    e.forestTrunkRanges ?? ids,
    layout,
    JSON.stringify(biome),
    family ?? undefined,
    JSON.stringify({
      terrain: presentationSettings.terrain,
      fog_geometry: presentationSettings.fog_geometry,
    }),
  );
}

/** A presentation of `input`'s ground alone: the ground scenes' and the
 *  tests' probes of what the material reads. Free it when done. */
export function presentGround(input: GroundInput): Presentation {
  const presentation = new Presentation();
  setGround(presentation, input);
  return presentation;
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
  // The playable rectangle is `[x, y, width, height]`, as the simulation's.
  const playable = [map[0], map[1], map[2] - map[0], map[3] - map[1]] as const;
  const extents = { physical: map, rendered: [0, 0, 0, 0] as const, playable };
  const layout = JSON.stringify({ propKinds: [], propAppearance: {} });
  const input = { exports: { ...streams, extents, terrain }, layout, biome, family: null };
  return { mesh, input, biome, grid: null, water: true };
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
  const input = {
    exports,
    layout: JSON.stringify(layout),
    biome,
    family: exports.buildings.regionalFamily,
    props: overlay === "surface",
  };
  // The traversal overlay shows the blocked flag alone; a water tint would
  // muddy it. The water's look is its own shader's (`renderer-core` `water.rs`).
  const surface = overlay === "surface";
  return { mesh, input, biome, grid: surface ? exports.terrain : null, water: surface };
}
