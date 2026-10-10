// @vitest-environment node
import { expect, test } from "vitest";
import { d, writeToArrayBuffer } from "typegpu";
import { schemaFields, u16Table } from "@packages/battle-renderer/src/frame/wordTables";
import { PlotNode, TerrainParams } from "@packages/battle-renderer/src/frame/terrainMaterial";
import { GrassParams, ShapeVertex } from "@packages/battle-renderer/src/frame/grassPass";

test("a reader following the layout gets back every value the GPU write serialised", () => {
  const Look = d.struct({ core: d.vec4f, row: d.vec4u });
  const Params = d.struct({ region: d.vec4f, looks: d.arrayOf(Look, 2), view: d.vec4i });
  const value = {
    region: d.vec4f(1.5, -2, 3, 4),
    looks: [
      { core: d.vec4f(5, 6, 7, 8), row: d.vec4u(9, 10, 11, 12) },
      { core: d.vec4f(13, 14, 15, 16), row: d.vec4u(17, 18, 19, 20) },
    ],
    view: d.vec4i(-1, 0, 1, 2),
  };
  const bytes = new ArrayBuffer(d.sizeOf(Params));
  writeToArrayBuffer(bytes, Params, value);
  const views = {
    f32: new Float32Array(bytes),
    u32: new Uint32Array(bytes),
    i32: new Int32Array(bytes),
  };
  const read = Object.fromEntries(
    schemaFields(Params).map((f) => [
      f.name,
      Array.from(views[f.type].subarray(f.word, f.word + f.count)),
    ]),
  );
  expect(read).toEqual({
    region: [1.5, -2, 3, 4],
    "looks[0].core": [5, 6, 7, 8],
    "looks[0].row": [9, 10, 11, 12],
    "looks[1].core": [13, 14, 15, 16],
    "looks[1].row": [17, 18, 19, 20],
    view: [-1, 0, 1, 2],
  });
});

test("the terrain's and grass's own records are laid out word for word, with nothing unnamed", () => {
  expect(schemaFields(PlotNode)).toEqual([
    { name: "line", word: 0, type: "f32", count: 4 },
    { name: "children", word: 4, type: "i32", count: 4 },
  ]);
  for (const schema of [TerrainParams, GrassParams, ShapeVertex]) {
    const fields = schemaFields(schema);
    const covered = fields.reduce((words, f) => words + f.count, 0);
    expect(covered).toBe(d.sizeOf(schema) / 4);
    expect(new Set(fields.map((f) => f.word)).size).toBe(fields.length);
  }
});

test("u16 values lie two to a word, the first in the low half", () => {
  const { words, layout } = u16Table(Uint16Array.from([1, 2, 3, 0]), 3);
  expect(Array.from(words)).toEqual([0x00020001, 0x00000003]);
  expect(layout).toEqual({ kind: "u16", count: 3 });
});
