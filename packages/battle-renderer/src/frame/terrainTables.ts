// The terrain's GPU tables as words with their layouts (`wordTables.ts`):
// the height pages (`packTerrainHeights`) and the material's tables
// (`createTerrainSource`), exactly as they were uploaded, with the water
// surface's cells (which its mesh lays).
import {
  SURFACE_CELL_WORDS,
  SURFACE_EDGE,
  SURFACE_KIND_SHIFT,
  SURFACE_LEVEL_WORDS,
  SURFACE_NEW_SHAPE,
  SURFACE_RECORD_MASK,
  SURFACE_RECT,
  SURFACE_STROKE,
  SURFACE_TRIANGLE,
} from "../terrain/surfaceField";
import type { TerrainGrid } from "../terrain/terrainGrid";
import { d } from "typegpu";
import { PlotNode, PlotRecord, ScarParams, SurfaceRecord, TerrainParams } from "./terrainMaterial";
import type { TerrainSource } from "./terrainMaterial";
import { packTerrainHeights } from "./terrainHeights";
import { recordTable, structTable, type WordTable } from "./wordTables";

export type TerrainTableName =
  | "heights"
  | "nodes"
  | "plots"
  | "surfaces"
  | "surfaceIndex"
  | "params"
  | "scars"
  | "water";
export type TerrainTables = Record<TerrainTableName, WordTable>;

/** One square cell of the water surface (`Ground.water_cells`): its south-west
 *  corner, the surface's height and the cell's side, in metres. */
const WaterCell = d.struct({ corner: d.vec2f, z: d.f32, size: d.f32 });

/** The height grid as `createTerrainHeights` uploads it. */
export function heightTable(grid: TerrainGrid): WordTable {
  const { nx, ny, spacing, pageSize } = grid;
  return {
    words: packTerrainHeights(grid),
    layout: { kind: "height-pages", nx, ny, spacing, pageSize },
  };
}

/** The ground's tables as the frame last uploaded them. */
export function terrainTables(terrain: TerrainSource): TerrainTables {
  const uploaded = terrain.uploaded();
  if (!uploaded?.grid) throw new Error("the terrain has no height grid to export");
  const index = new Uint32Array(uploaded.surfaceIndex);
  return {
    heights: heightTable(uploaded.grid),
    nodes: recordTable(PlotNode, uploaded.nodes),
    plots: recordTable(PlotRecord, uploaded.plots),
    surfaces: recordTable(SurfaceRecord, uploaded.surfaces),
    surfaceIndex: {
      words: index,
      layout: {
        kind: "words",
        count: index.length,
        constants: {
          levelWords: SURFACE_LEVEL_WORDS,
          cellWords: SURFACE_CELL_WORDS,
          kindShift: SURFACE_KIND_SHIFT,
          newShape: SURFACE_NEW_SHAPE,
          recordMask: SURFACE_RECORD_MASK,
          rect: SURFACE_RECT,
          stroke: SURFACE_STROKE,
          triangle: SURFACE_TRIANGLE,
          edge: SURFACE_EDGE,
        },
      },
    },
    params: structTable(TerrainParams, uploaded.params),
    // The grid and the biome's scar look; its cache fields are the browser's.
    scars: structTable(ScarParams, uploaded.scars),
    water: recordTable(WaterCell, uploaded.water.buffer as ArrayBuffer),
  };
}
