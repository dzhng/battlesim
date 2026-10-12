// TerrainSurface: the ground layer as the renderer draws it. It is the
// simulation's own triangles (exported vertices in exported index order,
// flat-shaded, never resampled or exaggerated: landmine 11) plus what the
// biome material needs to paint them: the plot split, and the roads, forests
// and rivers exactly as the simulation's surface rules define them, so a road
// is drawn precisely where units find road. The shapes are resolved in Rust
// (`crates/static-world/src/ground`), which the presentation module must
// have loaded (`loadPresentation`).
import { vec3 } from "math";
import { triangle3 } from "math/shapes";
import { Ground } from "@wasm/presentation/presentation_wasm.js";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "../mesh";
import { drawnBy } from "../models/propAppearance";
import type { WorldExports, WorldLayout, WorldOverlay } from "../worldMesh";
import { checkSurfaceKinds } from "./surfaces";
import { regionalBiome, type Biome } from "./biome";
import { plotTree, type PlotTree } from "./plots";
import {
  FOREST_BOUNDARY_FLOATS,
  FOREST_TRIANGLE_FLOATS,
  forestShapes,
  type ForestShape,
} from "./forestShapes";
import { RIVER_FIELDS, RIVER_FLOATS } from "./rivers";
import { checkStrokeLayout } from "./strokes";
import type { TerrainGrid } from "./terrainGrid";

/** Prop footprints `x, y, yaw, hx, hy` per record: an oriented box. */
export const FOOTPRINT_FLOATS = 5;

/** What the ground is resolved from: the exported streams, as the
 *  simulation's `WorldView` names them (`worldMesh.ts`), and the map. */
export interface GroundSource extends Pick<
  WorldExports,
  | "surfaceStrokes"
  | "surfaceRuns"
  | "surfaceTriangles"
  | "surfaceBoundaries"
  | "rivers"
  | "riverRuns"
  | "forests"
  | "forestRectIds"
  | "forestMetadata"
  | "forestStrokes"
  | "forestTriangles"
  | "forestBoundaries"
  | "forestTrunkRanges"
> {
  /** Physical terrain footprint for field/scenery placement, `[minX, minY, maxX, maxY]`.
   *  Common map extents publish the height grid's rounded boundary. */
  map: readonly [number, number, number, number];
  /** Map-owned landscape extent; authored arenas retain their environment backdrop. */
  rendered?: readonly [number, number, number, number];
  /** The height samples' spacing in metres: the ground's triangles are this
   *  wide. */
  gridM: number;
  /** Building centres, `x, y` each. */
  buildings: Float32Array;
  /** Every static prop's footprint (`FOOTPRINT_FLOATS` each), where no grass grows. */
  footprints: Float32Array;
}

/** Where the ground's features are, in world metres. */
export interface TerrainSite extends GroundSource {
  /** The forests in authored order, with their canopy; also used to fit
   *  tree crowns. */
  forestShapes: readonly ForestShape[];
}

export interface TerrainSurface {
  /** Ground triangles in the one vertex format. The vertex colour is a tint
   *  over the biome material, its alpha the tint's weight: 0 draws the biome,
   *  1 the tint alone (the traversal view, render-only fixtures). */
  mesh: Mesh;
  site: TerrainSite;
  plots: PlotTree;
  /** The site's paved strokes as the ground draws them: a road through a
   *  settlement's own ground is tagged as the kind its row names for a
   *  town (`biome.roads.<kind>.town`). The site's own array where no road
   *  is. */
  strokes: Float32Array;
  biome: Biome;
  /** The height grid grass is seated on; null where no grass grows (the
   *  traversal view, render-only patches: where the tint replaces the biome). */
  grid: TerrainGrid | null;
  /** The resolved ground, for its surface fields (`surfaceField`) and its
   *  water's cells. */
  ground: Ground;
}

/** The traversal view's colours: ground units can or cannot enter. */
export const OPEN: Rgba = [0.52, 0.56, 0.5, 1];
export const BLOCKED: Rgba = [0.78, 0.3, 0.26, 1];
const BIOME: Rgba = [0, 0, 0, 0];

/** The ground of source `s` under `biome` (in its map's region's finish). */
export function terrainSurface(
  mesh: Mesh,
  s: GroundSource,
  biome: Biome,
  grid: TerrainGrid | null,
): TerrainSurface {
  const ground = new Ground(
    Float64Array.from([...s.map, ...(s.rendered ?? [])]),
    s.surfaceStrokes,
    s.surfaceRuns,
    s.surfaceTriangles,
    s.surfaceBoundaries,
    s.rivers,
    s.riverRuns,
    s.forests,
    s.forestRectIds,
    s.forestMetadata,
    s.forestStrokes,
    s.forestTriangles,
    s.forestBoundaries,
    s.forestTrunkRanges,
    s.buildings,
    JSON.stringify(biome),
  );
  return {
    mesh,
    site: { ...s, forestShapes: forestShapes(ground.forests(), s) },
    plots: plotTree(ground),
    strokes: ground.drawn_strokes() ?? s.surfaceStrokes,
    biome,
    grid,
    ground,
  };
}

/** A ground of `map` with nothing on it, its triangles `gridM` wide. */
export function bareSource(map: GroundSource["map"], gridM: number): GroundSource {
  const none = new Float32Array(0);
  const ids = new Uint32Array(0);
  return {
    map,
    gridM,
    surfaceStrokes: none,
    surfaceRuns: none,
    surfaceTriangles: none,
    surfaceBoundaries: none,
    rivers: none,
    riverRuns: none,
    forests: none,
    forestRectIds: ids,
    forestMetadata: none,
    forestStrokes: none,
    forestTriangles: none,
    forestBoundaries: none,
    forestTrunkRanges: ids,
    buildings: none,
    footprints: none,
  };
}

/** Refuse a native layout whose streams the ground would misread. */
function checkLayout(layout: WorldLayout) {
  checkStrokeLayout(layout);
  checkSurfaceKinds(layout);
  if (
    layout.forestTriangleStride !== FOREST_TRIANGLE_FLOATS ||
    layout.forestBoundaryStride !== FOREST_BOUNDARY_FLOATS
  )
    throw new Error("forest primitive strides differ from the forest-floor shader's records");
  if (
    layout.riverStride !== RIVER_FLOATS ||
    RIVER_FIELDS.some((field, k) => layout.riverFields[k] !== field)
  )
    throw new Error("river stretch fields differ from the water distance's records");
}

/** The ground of the exported world under `biome`, in its map's region's
 *  finish (`regionalBiome`); the traversal view tints
 *  each triangle by its exported blocked flag instead. */
export function buildTerrainSurface(
  exports: WorldExports,
  layout: WorldLayout,
  biome: Biome,
  overlay: WorldOverlay = "surface",
): TerrainSurface {
  checkLayout(layout);
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
  const buildings: number[] = [];
  const kindAt = layout.propFields.indexOf("kind");
  const at = ["x", "y", "yaw", "hx", "hy"].map((f) => layout.propFields.indexOf(f));
  const footprints = new Float32Array(
    (exports.props.length / layout.propStride) * FOOTPRINT_FLOATS,
  );
  for (let o = 0, r = 0; o < exports.props.length; o += layout.propStride, r++) {
    at.forEach((f, k) => (footprints[r * FOOTPRINT_FLOATS + k] = exports.props[o + f]));
    if (drawnBy(layout, layout.propKinds[exports.props[o + kindAt]], "building"))
      buildings.push(exports.props[o + at[0]], exports.props[o + at[1]]);
  }
  const { physical, rendered } = exports.extents;
  return terrainSurface(
    mesh,
    {
      ...exports,
      map: physical,
      ...(rendered[0] < 0 && { rendered }),
      gridM: exports.terrain.spacing,
      buildings: Float32Array.from(buildings),
      footprints,
    },
    regionalBiome(biome, exports.buildings.regionalFamily),
    overlay === "surface" ? exports.terrain : null,
  );
}
