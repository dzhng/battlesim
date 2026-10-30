// Presentation of the exported authoritative geometry, in layers. The ground
// is the terrain surface (`terrain/terrainSurface.ts`): the simulation's own
// triangles under the biome's material. Props are their appearances fitted to
// their exported boxes, water is laid along the exported rivers, and forests
// are the scenery's trees, which draw the simulation's trunks too: all three
// resolved by the scene's presentation (`crates/presentation`). Colours are
// presentation only.
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { grassAppearancesOf } from "./terrain/grassField";
import { MeshBuilder, type Rgba } from "./mesh";
import type { WorldLayers, WorldScenery } from "./scene";
import { BLOCKED, buildTerrainSurface, OPEN, type Biome } from "./terrain/terrainSurface";
import { sceneryAppearances } from "./scenery/appearance";

/** Parsed `world_layout()` from the WASM boundary. */
export interface WorldLayout {
  surfaceKinds: string[];
  /** The kinds a paved stretch, triangle or boundary edge's `kind` names (its
   *  area's own, as the map says it), and which of them are carriageways. */
  surfaceAreaKinds: string[];
  roadAreaKinds: string[];
  propKinds: string[];
  /** The unit types, in index order: a map wreck's `wreckOf` field indexes it. */
  unitKinds: string[];
  /** A hull's model, by unit type: a wreck of that type is drawn as that
   *  model's own wreck (its asset catalog `wreck`). */
  unitAppearance: Record<string, string>;
  /** Per mover class ("infantry", "vehicle"), the prop kinds that stop it. */
  blockingPropKinds: Record<string, string[]>;
  /** The prop kinds that hide what lies behind them from sight. */
  occludingPropKinds: string[];
  flags: { forest: number; blocked: number };
  propStride: number;
  limbBits: number;
  areaStride: number;
  propFields: string[];
  areaFields: string[];
  surfaceStrokeStride: number;
  surfaceStrokeFields: string[];
  /** The bits of a stroke stretch's `cuts`: which of its ends is its
   *  stroke's own, cut square. */
  strokeCuts: { a: number; b: number };
  surfaceRunStride: number;
  surfaceRunFields: string[];
  surfaceTriangleStride: number;
  surfaceTriangleFields: string[];
  surfaceBoundaryStride: number;
  surfaceBoundaryFields: string[];
  riverStride: number;
  riverFields: string[];
  riverRunStride: number;
  riverRunFields: string[];
  forestTrunkRangeStride: number;
  forestTrunkRangeFields: string[];
  forestMetadataStride: number;
  forestMetadataFields: string[];
  forestShapeKinds: string[];
  forestStrokeStride: number;
  forestStrokeFields: string[];
  forestTriangleStride: number;
  forestTriangleFields: string[];
  forestBoundaryStride: number;
  forestBoundaryFields: string[];
}

import type { TerrainGrid } from "./terrain/terrainGrid";

/** Immutable public references, not a second physical geometry table. */
export interface PublicBuildings {
  catalogueHash: string | null;
  /** The map's region (`MapDefinition.regional_family`): what its ground
   *  and scenery are dressed in. Null on a map without buildings. */
  regionalFamily: string | null;
  buildings: {
    owner: number;
    kind: string;
    templateId: string;
    category: string;
    regionalFamily: string;
    /** Where its template was placed (the contract's `PlacementFrame`): its
     *  template's art is drawn here. */
    frame: { translation: [number, number, number]; yaw: number };
    parts: { part: string; prop: number }[];
  }[];
}

/** Derived by the map contract. The display environment is separate from
 *  these map-owned bounds; physical queries never include visual surroundings. */
export interface MapExtents {
  playable: readonly [number, number, number, number];
  physical: readonly [number, number, number, number];
  rendered: readonly [number, number, number, number];
}

export interface WorldExports {
  extents: MapExtents;
  terrain: TerrainGrid;
  positions: Float32Array;
  indices: Uint32Array;
  /** Two bytes per triangle: ground kind tag, flags. */
  triangleSurfaces: Uint8Array;
  props: Float32Array;
  buildings: PublicBuildings;
  /** Every river's rounded stretches (`layout.riverFields`): the water. */
  rivers: Float32Array;
  /** The rivers' long runs (`ax, ay, bx, by`): what fields are cut along. */
  riverRuns: Float32Array;
  /** Every swamp's outline, edge by edge (`layout.swampStride`): its water. */
  swamps: Float32Array;
  /** Exact rectangular fast-path rows; general shapes stay in native primitive streams. */
  forests: Float32Array;
  forestTrunkRanges: Uint32Array;
  forestRectIds: Uint32Array;
  forestMetadata: Float32Array;
  forestStrokes: Float32Array;
  forestTriangles: Float32Array;
  forestBoundaries: Float32Array;
  /** The foliage grid standing trees give (`WorldView.foliage`). */
  foliage: Float32Array;
  /** Native strokes, polygon membership triangles and exposed union boundaries. */
  surfaceStrokes: Float32Array;
  /** The road strokes' authored control runs (`ax, ay, bx, by`): what fields are cut along. */
  surfaceRuns: Float32Array;
  surfaceTriangles: Float32Array;
  surfaceBoundaries: Float32Array;
}

/** The static-world exports a `WorldView` publishes, read in one place so a
 *  new export is wired once. */
export interface WorldExportSource {
  extents(): string;
  terrain_grid(): string;
  terrain_page_ids(): Uint32Array;
  terrain_heights(): Float32Array;
  terrain_positions(): Float32Array;
  terrain_indices(): Uint32Array;
  terrain_triangle_surfaces(): Uint8Array;
  props(): Float32Array;
  buildings(): string;
  rivers(): Float32Array;
  river_runs(): Float32Array;
  swamps(): Float32Array;
  forests(): Float32Array;
  forest_trunk_ranges(): Uint32Array;
  forest_rect_ids(): Uint32Array;
  forest_metadata(): Float32Array;
  forest_strokes(): Float32Array;
  forest_triangles(): Float32Array;
  forest_boundaries(): Float32Array;
  foliage(): Float32Array;
  surface_strokes(): Float32Array;
  surface_runs(): Float32Array;
  surface_triangles(): Float32Array;
  surface_boundaries(): Float32Array;
}

export function readWorldExports(view: WorldExportSource): WorldExports {
  return {
    extents: JSON.parse(view.extents()),
    terrain: {
      ...JSON.parse(view.terrain_grid()),
      pageIds: view.terrain_page_ids(),
      heights: view.terrain_heights(),
    },
    positions: view.terrain_positions(),
    indices: view.terrain_indices(),
    triangleSurfaces: view.terrain_triangle_surfaces(),
    props: view.props(),
    buildings: JSON.parse(view.buildings()),
    rivers: view.rivers(),
    riverRuns: view.river_runs(),
    swamps: view.swamps(),
    forests: view.forests(),
    forestTrunkRanges: view.forest_trunk_ranges(),
    forestRectIds: view.forest_rect_ids(),
    forestMetadata: view.forest_metadata(),
    forestStrokes: view.forest_strokes(),
    forestTriangles: view.forest_triangles(),
    forestBoundaries: view.forest_boundaries(),
    foliage: view.foliage(),
    surfaceStrokes: view.surface_strokes(),
    surfaceRuns: view.surface_runs(),
    surfaceTriangles: view.surface_triangles(),
    surfaceBoundaries: view.surface_boundaries(),
  };
}

/** "surface" draws the biome; "traversal" marks what ground units cannot enter. */
export type WorldOverlay = "surface" | "traversal";

/** Stops some mover classes but not others (a wreck stops vehicles only). */
const PARTLY_BLOCKED: Rgba = [0.86, 0.6, 0.22, 1];

function fieldReader(fields: string[], stride: number, data: Float32Array) {
  const offset = Object.fromEntries(fields.map((f, i) => [f, i]));
  return {
    count: data.length / stride,
    get: (row: number, field: string) => data[row * stride + offset[field]],
  };
}

/** A prop's traversal colour: how many mover classes it stops. */
function propTraversal(layout: WorldLayout, kind: string): Rgba {
  const classes = Object.values(layout.blockingPropKinds);
  const stops = classes.filter((kinds) => kinds.includes(kind)).length;
  return stops === 0 ? OPEN : stops === classes.length ? BLOCKED : PARTLY_BLOCKED;
}

/** The traversal view's prop boxes, tinted by how many mover classes each stops. */
function addTraversalProps(mesh: MeshBuilder, exports: WorldExports, layout: WorldLayout) {
  const props = fieldReader(layout.propFields, layout.propStride, exports.props);
  for (let r = 0; r < props.count; r++) {
    const kind = layout.propKinds[props.get(r, "kind")];
    mesh.orientedBox(
      props.get(r, "x"),
      props.get(r, "y"),
      props.get(r, "yaw"),
      [props.get(r, "hx"), props.get(r, "hy"), props.get(r, "hz")],
      props.get(r, "baseZ"),
      propTraversal(layout, kind),
    );
  }
}

/** The static world's layers: the ground under `biome` (whose props the
 *  scene's presentation draws as their appearances in the surface view),
 *  and, given the installed appearances, the scenery's and the grass's art
 *  (the surface view; the traversal view draws the props' boxes instead). */
export function buildWorldLayers(
  exports: WorldExports,
  layout: WorldLayout,
  biome: Biome,
  overlay: WorldOverlay,
  appearances: InstalledAppearances | null = null,
  drawScenery = true,
): WorldLayers {
  const props = new MeshBuilder();
  if (overlay === "traversal") addTraversalProps(props, exports, layout);

  const terrain = buildTerrainSurface(exports, layout, biome, overlay);
  return {
    terrain,
    skirtPositions: exports.positions,
    skirt_and_props: props.build(),
    scenery:
      overlay === "surface" && appearances && drawScenery ? worldScenery(biome, appearances) : null,
    grass:
      overlay === "surface" && appearances && drawScenery ? grassAppearancesOf(appearances) : null,
  };
}

/** The trees and hedgerows of `biome.trees` and the forest floor's dressing:
 *  the installed appearances the scenery's rows name. Where they stand is
 *  the scene's presentation's. */
function worldScenery(biome: Biome, installed: InstalledAppearances): WorldScenery {
  const { trees, forest_floor } = biome;
  const dressed = forest_floor.dressing.kinds.map((k) => k.appearance);
  const named = [
    ...new Set([
      ...trees.species.map((s) => s.appearance),
      trees.hedgerows.appearance,
      trees.understorey.appearance,
    ]),
  ].filter((name) => !dressed.includes(name));
  return {
    appearances: new Map([
      ...sceneryAppearances(installed, named, ["tree", "hedgerow"]),
      ...sceneryAppearances(installed, dressed, ["dressing"]),
    ]),
  };
}
