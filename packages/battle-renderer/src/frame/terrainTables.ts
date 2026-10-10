// The terrain's GPU tables as words, each with the layout a reader needs to
// decode it without this code: the height pages (`packTerrainHeights`) and
// the material's tables (`createTerrainSource`), exactly as they were
// uploaded. A second client draws the browser's ground from them.
import { d, writeToArrayBuffer } from "typegpu";

type AnyWgslData = d.AnyWgslData;
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
import { PlotNode, PlotRecord, SurfaceRecord, TerrainParams } from "./terrainMaterial";
import type { TerrainSource } from "./terrainMaterial";
import { packTerrainHeights } from "./terrainHeights";

/** One primitive of a record: `count` components of `type` from word `word`. */
export interface WordField {
  name: string;
  word: number;
  type: "f32" | "u32" | "i32";
  count: number;
}

/** How a table's words are read. */
export type WordLayout =
  /** `count` records of `stride` words, each holding `fields`. */
  | { kind: "records"; stride: number; count: number; fields: WordField[] }
  /** Plain u32 words, read by the named `constants`. */
  | { kind: "words"; count: number; constants: Record<string, number> }
  /** The vertex grid (`nx` × `ny` vertices `spacing` metres apart from the
   *  origin) in square pages of `pageSize`: three header words (page size,
   *  page columns, directory length), the directory (row-major page id →
   *  present page + 1, or 0 for flat ground at height 0), then each present
   *  page's f32 heights row-major. */
  | { kind: "height-pages"; nx: number; ny: number; spacing: number; pageSize: number };

export interface WordTable {
  words: Uint32Array;
  layout: WordLayout;
}

export type TerrainTableName =
  | "heights"
  | "nodes"
  | "plots"
  | "surfaces"
  | "surfaceIndex"
  | "params";
export type TerrainTables = Record<TerrainTableName, WordTable>;

const PRIMITIVES: Record<string, Pick<WordField, "type" | "count">> = {
  f32: { type: "f32", count: 1 },
  u32: { type: "u32", count: 1 },
  i32: { type: "i32", count: 1 },
  vec2f: { type: "f32", count: 2 },
  vec3f: { type: "f32", count: 3 },
  vec4f: { type: "f32", count: 4 },
  vec2u: { type: "u32", count: 2 },
  vec3u: { type: "u32", count: 3 },
  vec4u: { type: "u32", count: 4 },
  vec2i: { type: "i32", count: 2 },
  vec3i: { type: "i32", count: 3 },
  vec4i: { type: "i32", count: 4 },
};

type Schema = AnyWgslData & {
  type: string;
  propTypes?: Record<string, Schema>;
  elementType?: Schema;
  elementCount?: number;
};

/** Every primitive of `schema` in memory order, named by its path
 *  (`roads[2].core`), at the word TypeGPU lays it out at. */
export function schemaFields(schema: AnyWgslData): WordField[] {
  const fields: WordField[] = [];
  const walk = (node: Schema, name: string, path: (string | number)[]) => {
    const primitive = PRIMITIVES[node.type];
    if (primitive) {
      const { offset } = d.memoryLayoutOf(schema, (p) =>
        path.reduce<unknown>((at, key) => (at as Record<string | number, unknown>)[key], p),
      );
      fields.push({ name, word: offset / 4, ...primitive });
    } else if (node.type === "struct")
      for (const [key, child] of Object.entries(node.propTypes!))
        walk(child, name ? `${name}.${key}` : key, [...path, key]);
    else if (node.type === "array")
      for (let i = 0; i < node.elementCount!; i++)
        walk(node.elementType!, `${name}[${i}]`, [...path, i]);
    else throw new Error(`no word layout for ${node.type}`);
  };
  walk(schema as Schema, "", []);
  return fields;
}

/** `bytes` read as records of `schema`. */
function recordTable(schema: AnyWgslData, bytes: ArrayBuffer): WordTable {
  const words = new Uint32Array(bytes);
  const stride = d.sizeOf(schema) / 4;
  return {
    words,
    layout: { kind: "records", stride, count: words.length / stride, fields: schemaFields(schema) },
  };
}

/** `value` serialised as the GPU write of `schema` serialises it. */
function structTable<T extends AnyWgslData>(schema: T, value: d.InferInput<T>): WordTable {
  const bytes = new ArrayBuffer(d.sizeOf(schema));
  writeToArrayBuffer(bytes, schema, value);
  return recordTable(schema, bytes);
}

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
  };
}
