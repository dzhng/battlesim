// Presentation of the exported authoritative geometry, in layers. The ground
// is the terrain surface (`terrain/terrainSurface.ts`): the simulation's own
// triangles under the biome's material. Props are their appearances fitted to
// their exported boxes (`models/propAppearance.ts`), water is laid along the
// exported rivers, and forests are the scenery's trees (`scenery/placement.ts`),
// which draw the simulation's trunks too. Colours are presentation only.
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  drawnBy,
  mapProps,
  PropAppearances,
  structureModels,
  type PropAppearance,
} from "./models/propAppearance";
import { grassAppearancesOf } from "./terrain/grassField";
import { MeshBuilder, type Rgba } from "./mesh";
import type { WorldLayers, WorldScenery } from "./scene";
import type { Biome } from "./terrain/biome";
import { BLOCKED, buildTerrainSurface, OPEN, type TerrainSurface } from "./terrain/terrainSurface";
import { waterSurfaceMesh } from "./terrain/rivers";
import { kindSize, sceneryAppearances } from "./scenery/appearance";
import { placeScenery, sceneryKinds, scenerySite } from "./scenery/placement";

/** Parsed `world_layout()` from the WASM boundary. */
export interface WorldLayout {
  surfaceKinds: string[];
  /** The kinds a paved stretch, triangle or boundary edge's `kind` names (its
   *  area's own, as the map says it), and which of them are carriageways. */
  surfaceAreaKinds: string[];
  roadAreaKinds: string[];
  propKinds: string[];
  /** Per mover class ("infantry", "vehicle"), the prop kinds that stop it. */
  blockingPropKinds: Record<string, string[]>;
  /** The prop kinds that hide what lies behind them from sight. */
  occludingPropKinds: string[];
  /** The prop kinds something can shove: they move, so a battle draws them
   *  from what the side knows. */
  movablePropKinds: string[];
  /** The prop kinds fire can destroy (the body row's integrity column):
   *  a battle draws them from what the side knows, so one seen destroyed
   *  leaves the world. */
  destroyablePropKinds: string[];
  /** What draws each prop kind (its catalog `appearance`). */
  propAppearance: Record<string, PropAppearance>;
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
const NO_RIVERS = new Float32Array(0);
const SKIRT: Rgba = [0.33, 0.3, 0.26, 1];
const SKIRT_DEPTH_M = 6;

function fieldReader(fields: string[], stride: number, data: Float32Array) {
  const offset = Object.fromEntries(fields.map((f, i) => [f, i]));
  return {
    count: data.length / stride,
    get: (row: number, field: string) => data[row * stride + offset[field]],
  };
}

/** The prop kinds a battle draws apart from the world, from what the side
 *  knows (`structureModels`), so a change never rebuilds the whole world:
 *  every kind something can shove, and (`destroyable`) every kind fire can
 *  destroy. Trees stay the scenery's either way. */
export function apartKinds(layout: WorldLayout, destroyable: boolean): string[] {
  const kinds = new Set([
    ...layout.movablePropKinds,
    ...(destroyable ? layout.destroyablePropKinds : []),
  ]);
  // Trees are drawn by the scenery's trees in the surface view; boxes only in
  // the traversal view, where they show what blocks.
  return [...kinds].filter((kind) => !drawnBy(layout, kind, "forest"));
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

/** The static world's layers: the ground under `biome`, and, given the
 *  installed appearances, the props on it as appearances, the scenery and
 *  the grass kinds (the surface view; the traversal view draws the props' boxes instead).
 *  `apart` (`apartKinds`) leaves out the kinds a route draws from what the
 *  side knows (`structureModels`). */
export function buildWorldLayers(
  exports: WorldExports,
  layout: WorldLayout,
  biome: Biome,
  overlay: WorldOverlay,
  apart: readonly string[] = [],
  appearances: InstalledAppearances | null = null,
): WorldLayers {
  const props = new MeshBuilder();
  addSkirt(props, exports.positions);
  if (overlay === "traversal") addTraversalProps(props, exports, layout);
  const drawn =
    overlay === "surface" && appearances
      ? structureModels(
          mapProps(exports, layout),
          [],
          new PropAppearances(appearances, layout),
          (prop) => !drawnBy(layout, prop.kind, "forest") && !apart.includes(prop.kind),
        )
      : [];

  const terrain = buildTerrainSurface(exports, layout, biome, overlay);
  return {
    terrain,
    props: props.build(),
    structures: drawn,
    // The traversal overlay shows the blocked flag alone; a water tint would
    // muddy it. The water's look is the terrain material's (`waterSurface`).
    water: waterSurfaceMesh(overlay === "traversal" ? NO_RIVERS : terrain.site.rivers),
    scenery:
      overlay === "surface" && appearances
        ? worldScenery(exports, layout, terrain, biome, appearances)
        : null,
    grass: overlay === "surface" && appearances ? grassAppearancesOf(appearances) : null,
  };
}

/** The trees and hedgerows of `biome.trees` and the forest floor's dressing,
 *  instancing installed appearances. */
function worldScenery(
  exports: WorldExports,
  layout: WorldLayout,
  terrain: TerrainSurface,
  biome: Biome,
  installed: InstalledAppearances,
): WorldScenery {
  const trees = biome.trees;
  const dressing = biome.forest_floor.dressing;
  const dressed = dressing.kinds.map((k) => k.appearance);
  const appearances = new Map([
    ...sceneryAppearances(
      installed,
      sceneryKinds(biome).filter((name) => !dressed.includes(name)),
      ["tree", "hedgerow"],
    ),
    ...sceneryAppearances(installed, dressed, ["dressing"]),
  ]);
  const sizes = new Map([...appearances].map(([name, bundle]) => [name, kindSize(bundle)]));
  return {
    placement: placeScenery(scenerySite(exports, layout, terrain), biome, sizes),
    appearances,
    lodPx: trees.lod_px,
    dressing: { lodPx: dressing.lod_px, fadePx: dressing.fade_px },
  };
}

/** Vertical faces down from the map's boundary vertices, so the bounded map
 *  reads as solid ground rather than a paper-thin sheet. They stand on the
 *  ground, so they draw with the props, as faces. Presentation only. */
function addSkirt(mesh: MeshBuilder, positions: Float32Array) {
  let maxX = 0,
    maxY = 0,
    minZ = Infinity;
  for (let i = 0; i < positions.length; i += 3) {
    maxX = Math.max(maxX, positions[i]);
    maxY = Math.max(maxY, positions[i + 1]);
    minZ = Math.min(minZ, positions[i + 2]);
  }
  const bottom = minZ - SKIRT_DEPTH_M;
  const edges: [number, number, number][][] = [[], [], [], []];
  for (let i = 0; i < positions.length; i += 3) {
    const v: [number, number, number] = [positions[i], positions[i + 1], positions[i + 2]];
    if (v[1] === 0) edges[0].push(v);
    if (v[0] === maxX) edges[1].push(v);
    if (v[1] === maxY) edges[2].push(v);
    if (v[0] === 0) edges[3].push(v);
  }
  edges[0].sort((a, b) => a[0] - b[0]);
  edges[1].sort((a, b) => a[1] - b[1]);
  edges[2].sort((a, b) => b[0] - a[0]);
  edges[3].sort((a, b) => b[1] - a[1]);
  for (const edge of edges) {
    for (let k = 0; k + 1 < edge.length; k++) {
      const [a, b] = [edge[k], edge[k + 1]];
      mesh.quad(a, b, [b[0], b[1], bottom], [a[0], a[1], bottom], SKIRT);
    }
  }
}
