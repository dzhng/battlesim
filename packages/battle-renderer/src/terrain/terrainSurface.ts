// TerrainSurface: the ground layer as the renderer draws it. It is the
// simulation's own triangles (exported vertices in exported index order,
// flat-shaded, never resampled or exaggerated: landmine 11) plus what the
// biome material needs to paint them: the plot split, and the roads, forests
// and rivers exactly as the simulation's surface rules define them, so a road
// is drawn precisely where units find road.
//
// Rewritten from reading ~/dev/game
// battle-renderer/src/world/terrain.ts and shaders/terrainMaterial.ts: there
// the ground was a cell-centred bilinear grid with 1.6× relief and its roads
// and mud came from a baked distance texture.
import { vec2, vec3, type Vec2 } from "math";
import { triangle3 } from "math/shapes";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "../mesh";
import { drawnBy } from "../models/propAppearance";
import type { WorldExports, WorldLayout, WorldOverlay } from "../worldMesh";
import { pick } from "@packages/renderer-core/src/kindTable";
import {
  checkSurfaceKinds,
  drawnStrokes,
  SURFACE_AREA_KINDS,
  type SurfaceAreaKind,
  type SurfaceGeometry,
} from "./surfaces";
import { roadRow, type Biome } from "./biome";
import { generatePlots, plotAt, type PlotTree } from "./plots";
import { buildForestShapes, type ForestShape } from "./forestShapes";
import { RIVER_FIELDS, RIVER_FLOATS } from "./rivers";
import { checkStrokeLayout } from "./strokes";
import type { TerrainGrid } from "./terrainGrid";

/** Rects `x, y, w, h` per record. */
export const RECT_FLOATS = 4;
/** Prop footprints `x, y, yaw, hx, hy` per record: an oriented box. */
export const FOOTPRINT_FLOATS = 5;

/** Where the ground's features are, in world metres. */
export interface TerrainSite extends SurfaceGeometry {
  /** Physical terrain footprint for field/scenery placement, `[minX, minY, maxX, maxY]`.
   *  Common map extents publish the height grid's rounded boundary. */
  map: readonly [number, number, number, number];
  /** Map-owned landscape extent; authored arenas retain their environment backdrop. */
  rendered?: readonly [number, number, number, number];
  /** The height samples' spacing in metres: the ground's triangles are this
   *  wide. */
  gridM: number;
  /** Exact rectangle fast-path forests (`RECT_FLOATS` each): the forest floor,
   *  as the simulation's forest ground (less its cleared lanes, drawn as
   *  crushed ground). */
  forests: Float32Array;
  /** Authored-order primitives and canopy metadata; also used to fit tree crowns. */
  forestShapes: readonly ForestShape[];
  /** Every river's rounded stretches (`RIVER_FLOATS` each, `terrain/rivers.ts`). */
  rivers: Float32Array;
  buildings: readonly Vec2[];
  /** Every static prop's footprint (`FOOTPRINT_FLOATS` each), where no grass grows. */
  footprints: Float32Array;
}

export interface TerrainSurface {
  /** Ground triangles in the one vertex format. The vertex colour is a tint
   *  over the biome material, its alpha the tint's weight: 0 draws the biome,
   *  1 the tint alone (the traversal view, render-only fixtures). */
  mesh: Mesh;
  site: TerrainSite;
  plots: PlotTree;
  /** The site's paved strokes as the ground draws them (`drawnStrokes`): a
   *  road through a settlement's own ground is tagged as the kind its row
   *  names for a town (`biome.roads.<kind>.town`). The site's own array
   *  where no road is. */
  strokes: Float32Array;
  biome: Biome;
  /** The height grid grass is seated on; null where no grass grows (the
   *  traversal view, render-only patches: where the tint replaces the biome). */
  grid: TerrainGrid | null;
}

/** The traversal view's colours: ground units can or cannot enter. */
export const OPEN: Rgba = [0.52, 0.56, 0.5, 1];
export const BLOCKED: Rgba = [0.78, 0.3, 0.26, 1];
const BIOME: Rgba = [0, 0, 0, 0];

export function terrainSurface(
  mesh: Mesh,
  site: TerrainSite,
  biome: Biome,
  grid: TerrainGrid | null,
): TerrainSurface {
  const plots = generatePlots(site, biome);
  const settlement = biome.plots.findIndex((p) => p.name === biome.field_rules.settlement_kind);
  const through = SURFACE_AREA_KINDS.map((kind) => {
    const town = roadRow(biome, kind).town;
    return town
      ? {
          as: SURFACE_AREA_KINDS.indexOf(town.kind as SurfaceAreaKind),
          besideM: town.beside_m,
          gapM: town.gap_m,
          carryM: pick(biome.roads, town.kind).join_m,
        }
      : null;
  });
  const strokes = drawnStrokes(site, through, (x, y) => {
    const at = plotAt(plots, x, y);
    return at !== null && plots.plots[at.plot].kind === settlement;
  });
  return { mesh, site, plots, strokes, biome, grid };
}

/** Rects from an area export (`x, y, w, h, z` rows) without their height. */
function rects(area: Float32Array, layout: WorldLayout): Float32Array {
  const rows = area.length / layout.areaStride;
  const out = new Float32Array(rows * RECT_FLOATS);
  const at = ["x", "y", "w", "h"].map((f) => layout.areaFields.indexOf(f));
  for (let r = 0; r < rows; r++)
    for (let k = 0; k < RECT_FLOATS; k++)
      out[r * RECT_FLOATS + k] = area[r * layout.areaStride + at[k]];
  return out;
}

/** The exported river stretches, which must be the fields the water's
 *  distance reads, in its order. */
function rivers(exports: WorldExports, layout: WorldLayout): Float32Array {
  if (
    layout.riverStride !== RIVER_FLOATS ||
    RIVER_FIELDS.some((field, k) => layout.riverFields[k] !== field)
  )
    throw new Error("river stretch fields differ from the water distance's records");
  return exports.rivers;
}

/** The ground of the exported world under `biome`; the traversal view tints
 *  each triangle by its exported blocked flag instead. */
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
  const hasMargin = exports.extents.rendered[0] < 0;
  const map = exports.extents.physical;
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
  const buildings: Vec2[] = [];
  const kindAt = layout.propFields.indexOf("kind");
  const at = ["x", "y", "yaw", "hx", "hy"].map((f) => layout.propFields.indexOf(f));
  const footprints = new Float32Array(
    (exports.props.length / layout.propStride) * FOOTPRINT_FLOATS,
  );
  for (let o = 0, r = 0; o < exports.props.length; o += layout.propStride, r++) {
    at.forEach((f, k) => (footprints[r * FOOTPRINT_FLOATS + k] = exports.props[o + f]));
    if (drawnBy(layout, layout.propKinds[exports.props[o + kindAt]], "building"))
      buildings.push(vec2.fromValues(exports.props[o + at[0]], exports.props[o + at[1]]));
  }
  checkStrokeLayout(layout);
  checkSurfaceKinds(layout);
  return terrainSurface(
    mesh,
    {
      map,
      ...(hasMargin && { rendered: exports.extents.rendered }),
      gridM: exports.terrain.spacing,
      surfaceStrokes: exports.surfaceStrokes,
      surfaceStrokeStride: layout.surfaceStrokeStride,
      surfaceRuns: exports.surfaceRuns,
      surfaceRunStride: layout.surfaceRunStride,
      surfaceTriangles: exports.surfaceTriangles,
      surfaceTriangleStride: layout.surfaceTriangleStride,
      surfaceBoundaries: exports.surfaceBoundaries,
      surfaceBoundaryStride: layout.surfaceBoundaryStride,
      forests: rects(exports.forests, layout),
      forestShapes: buildForestShapes(exports, layout),
      rivers: rivers(exports, layout),
      riverRuns: exports.riverRuns,
      riverRunStride: layout.riverRunStride,
      buildings,
      footprints,
    },
    biome,
    overlay === "surface" ? exports.terrain : null,
  );
}
