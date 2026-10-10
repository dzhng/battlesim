// A GPU table as the words the frame uploaded, with the layout a reader
// needs to decode it without this code. A second client draws the
// browser's ground and grass from them (`terrainTables.ts`, `grassTables.ts`).
import { d, writeToArrayBuffer } from "typegpu";
import type { GrassTables } from "./grassTables";
import type { TerrainTables } from "./terrainTables";

type AnyWgslData = d.AnyWgslData;

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
  /** `count` u16 values, two to a word, the first in the low half; the
   *  last word may end in padding. */
  | { kind: "u16"; count: number }
  /** The vertex grid (`nx` × `ny` vertices `spacing` metres apart from the
   *  origin) in square pages of `pageSize`: three header words (page size,
   *  page columns, directory length), the directory (row-major page id →
   *  present page + 1, or 0 for flat ground at height 0), then each present
   *  page's f32 heights row-major. */
  | { kind: "height-pages"; nx: number; ny: number; spacing: number; pageSize: number }
  /** Prop footprints bucketed by a grid of cells, in 4-word elements:
   *  0: the grid's origin x, y and its cell's side (f32), 0; 1: cells across
   *  and up (u32), 0, 0; then two cells an element, row-major, each (first
   *  record's element, record count) as u32; then the records, grouped by
   *  cell, two elements each: (x, y, cos yaw, sin yaw) and (half extents
   *  with the margin, 0, 0), f32. A point is clear of a prop when it lies
   *  outside every record of its own cell. */
  | { kind: "prop-cells"; count: number };

export interface WordTable {
  words: Uint32Array;
  layout: WordLayout;
}

/** Every table the frame uploads for a second client to read. */
export interface UploadedTables {
  terrain: TerrainTables;
  grass: GrassTables;
}

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
export function recordTable(schema: AnyWgslData, bytes: ArrayBufferLike): WordTable {
  const words = new Uint32Array(bytes);
  const stride = d.sizeOf(schema) / 4;
  return {
    words,
    layout: { kind: "records", stride, count: words.length / stride, fields: schemaFields(schema) },
  };
}

/** `value` serialised as the GPU write of `schema` serialises it. */
export function structTable<T extends AnyWgslData>(schema: T, value: d.InferInput<T>): WordTable {
  const bytes = new ArrayBuffer(d.sizeOf(schema));
  writeToArrayBuffer(bytes, schema, value);
  return recordTable(schema, bytes);
}

/** `values` as uploaded: u16s two to a word. */
export function u16Table(values: Uint16Array, count: number): WordTable {
  return {
    words: new Uint32Array(values.buffer, values.byteOffset, values.byteLength / 4),
    layout: { kind: "u16", count },
  };
}
