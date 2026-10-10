// The grass pass's GPU tables as words with their layouts (`wordTables.ts`),
// exactly as `rebuild` uploaded them, with the numbers its draws read from
// uniforms. `params` is the uniform as last written: its per-frame fields
// (`planes`, `eye`, `window.xy`, `grid.xy`) belong to the frame that wrote
// them, and a second client recomputes them from each frame record; the rest
// is fixed for the map.
import { GRASS_HEIGHT_VARIATION, GRASS_SEGMENTS } from "@packages/scene-assets/src/grass.ts";
import { FIELD_LODS, GRASS_EDGE_M, GRASS_TILE_M } from "../terrain/grassField";
import { GrassParams, ShapeVertex, type GrassPass } from "./grassPass";
import { recordTable, structTable, u16Table, type WordTable } from "./wordTables";

export type GrassTableName = "params" | "props" | "shapes" | "indicesNear" | "indicesFar";

/** The numbers the build and draw read from uniforms, or size them by;
 *  a list holds one per tier (near, far). */
export interface GrassConstants {
  /** A build tile's side, metres. */
  tileM: number;
  /** A blade's segments, and its vertices. */
  segments: number[];
  bladeVertices: number[];
  /** The most clumps a tier holds, and where its clumps start in the clump table. */
  capacity: number[];
  firstClump: number[];
  /** Indices a tier's draw reads per clump instance (`indicesNear`, `indicesFar`). */
  indexCount: number[];
  /** Blades a clump's index strips cover: every kind is padded to the most. */
  maxBlades: number;
  /** Metres past a bare margin over which the stand thins and lowers. */
  edgeM: number;
  /** The least and most a clump's own hash scales its height by. */
  heightVariation: number[];
}

export interface GrassTables {
  tables: Record<GrassTableName, WordTable>;
  constants: GrassConstants;
  /** The grass kinds the field draws, by catalog name, in kind order: a
   *  clump's kind and `params.kinds[k]` index this. */
  kinds: string[];
}

/** The grass pass's tables as it last uploaded them. */
export function grassTables(grass: GrassPass): GrassTables {
  const up = grass.uploaded();
  if (!up) throw new Error("the grass pass has no field to export");
  const [near, far] = up.indices;
  return {
    tables: {
      params: structTable(GrassParams, up.params),
      props: {
        words: new Uint32Array(up.props.buffer, up.props.byteOffset, up.props.length),
        layout: { kind: "prop-cells", count: up.props.length / 4 },
      },
      shapes: recordTable(ShapeVertex, up.shapes.buffer),
      indicesNear: u16Table(near, up.indexCounts[0]),
      indicesFar: u16Table(far, up.indexCounts[1]),
    },
    constants: {
      tileM: GRASS_TILE_M,
      segments: FIELD_LODS.map((lod) => GRASS_SEGMENTS[lod]),
      bladeVertices: up.tiers.map((t) => t.z),
      capacity: [...up.capacity],
      firstClump: up.tiers.map((t) => t.y),
      indexCount: [...up.indexCounts],
      maxBlades: up.maxBlades,
      edgeM: GRASS_EDGE_M,
      heightVariation: [...GRASS_HEIGHT_VARIATION],
    },
    kinds: [...up.kinds],
  };
}
