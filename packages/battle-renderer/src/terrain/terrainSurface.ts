// TerrainSurface: the ground layer as the renderer draws it. It is the
// simulation's own triangles (exported vertices in exported index order,
// flat-shaded, never resampled or exaggerated: landmine 11) plus what the
// biome material needs to paint them: the plot split, and the roads, forests
// and water exactly as the simulation's surface rules define them, so a road
// is drawn precisely where units find road.
//
// Rewritten (reuse manifest, technique) from reading ~/dev/game
// battle-renderer/src/world/terrain.ts and shaders/terrainMaterial.ts: there
// the ground was a cell-centred bilinear grid with 1.6× relief and its roads
// and mud came from a baked distance texture.
import { vec2, type Vec2 } from "math";
import { MeshBuilder, type Mesh, type Rgba } from "../mesh";
import { drawnBy } from "../models/propAppearance";
import type { WorldExports, WorldLayout, WorldOverlay } from "../worldMesh";
import type { Biome } from "./biome";
import { generatePlots, type PlotTree } from "./plots";
import { terrainGrid, type TerrainGrid } from "./terrainGrid";

/** Rects `x, y, w, h` per record. */
export const RECT_FLOATS = 4;
/** Prop footprints `x, y, yaw, hx, hy` per record: an oriented box. */
export const FOOTPRINT_FLOATS = 5;

/** Where the ground's features are, in world metres. */
export interface TerrainSite {
  /** Map box `[minX, minY, maxX, maxY]`. */
  map: readonly [number, number, number, number];
  /** Road segments, `roadStride` floats each: `ax, ay, bx, by, halfWidth`. */
  roads: Float32Array;
  roadStride: number;
  /** The authored forests' rects (`RECT_FLOATS` each): the forest floor,
   *  as the simulation's forest ground (less its cleared lanes, drawn as
   *  crushed ground). */
  forests: Float32Array;
  water: Float32Array;
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
  return { mesh, site, plots: generatePlots(site, biome), biome, grid };
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

/** The ground of the exported world under `biome`; the traversal view tints
 *  each triangle by its exported blocked flag instead. */
export function buildTerrainSurface(
  exports: WorldExports,
  layout: WorldLayout,
  biome: Biome,
  overlay: WorldOverlay = "surface",
): TerrainSurface {
  const { positions, indices, triangleSurfaces } = exports;
  const mesh = new MeshBuilder();
  const p = (i: number) => [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]] as const;
  let maxX = 0,
    maxY = 0;
  for (let i = 0; i < positions.length; i += 3) {
    maxX = Math.max(maxX, positions[i]);
    maxY = Math.max(maxY, positions[i + 1]);
  }
  for (let t = 0; t < indices.length; t += 3) {
    const tint =
      overlay === "traversal"
        ? triangleSurfaces[(t / 3) * 2 + 1] & layout.flags.blocked
          ? BLOCKED
          : OPEN
        : BIOME;
    mesh.triangle(p(indices[t]), p(indices[t + 1]), p(indices[t + 2]), tint);
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
  return terrainSurface(
    mesh.build(),
    {
      map: [0, 0, maxX, maxY],
      roads: exports.roads,
      roadStride: layout.roadStride,
      forests: rects(exports.forests, layout),
      water: rects(exports.water, layout),
      buildings,
      footprints,
    },
    biome,
    overlay === "surface" ? terrainGrid(exports) : null,
  );
}
