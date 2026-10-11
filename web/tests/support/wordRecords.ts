// A second client's reading of a records table: each record's fields by
// name, following only the table's layout (`wordTables.ts`).
import type { WordTable } from "@packages/battle-renderer/src/frame/wordTables";

export type ReadRecord = Record<string, number[]>;

/** Every record of `table` (a `records` layout), its fields read by name. */
export function readRecords({ words, layout }: WordTable): ReadRecord[] {
  if (layout.kind !== "records") throw new Error(`a ${layout.kind} table holds no records`);
  const buffer = words.buffer.slice(words.byteOffset, words.byteOffset + words.byteLength);
  const views = {
    f32: new Float32Array(buffer),
    u32: new Uint32Array(buffer),
    i32: new Int32Array(buffer),
  };
  return Array.from({ length: layout.count }, (_, i) =>
    Object.fromEntries(
      layout.fields.map((f) => {
        const at = i * layout.stride + f.word;
        return [f.name, Array.from(views[f.type].subarray(at, at + f.count))];
      }),
    ),
  );
}
