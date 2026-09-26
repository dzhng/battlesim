// @vitest-environment node
// The models layer's 48-byte vertex: a bundle mesh packs into it without
// losing what shading needs — skin weights narrowed to unorm8 still sum to
// exactly one, and the tangent keeps its direction and handedness.
import { expect, test } from "vitest";
import { VERTEX_BYTES, packVertices } from "@packages/battle-renderer/src/models/modelLayer";
import type { MeshData } from "@packages/scene-assets/src/schema";

function mesh(weights: number[][], tangents?: number[][]): MeshData {
  const n = weights.length;
  return {
    positions: new Float32Array(n * 3),
    normals: new Int16Array(n * 4),
    uvs: new Float32Array(n * 2),
    colors: new Uint8Array(n * 4),
    joints: Uint8Array.from(weights.flatMap(() => [0, 1, 2, 3])),
    weights: Uint16Array.from(weights.flat()),
    ...(tangents ? { tangents: Int16Array.from(tangents.flat()) } : {}),
    indices: Uint16Array.from([0, 1, 2]),
    draws: [{ material: 0, first: 0, count: 3 }],
  };
}

test("skin weights narrow to unorm8 and still sum to exactly one", () => {
  // Awkward thirds and a tiny fourth influence: each rounds away from its share.
  const weights = [
    [21845, 21845, 21845, 0],
    [60000, 5000, 400, 135],
    [32767, 32768, 0, 0],
  ];
  const bytes = new Uint8Array(packVertices(mesh(weights), 0, null, () => 0));
  for (let v = 0; v < 3; v++) {
    const w = bytes.subarray(v * VERTEX_BYTES + 36, v * VERTEX_BYTES + 40);
    expect(w.reduce((a, b) => a + b, 0)).toBe(255);
    // No influence moves by more than a step and a half of the 8-bit scale.
    weights[v].forEach((x, c) => expect(Math.abs(w[c] / 255 - x / 65535)).toBeLessThan(1.5 / 255));
  }
});

test("tangents keep their direction and handedness; a mesh without them packs zero", () => {
  const tangents = [
    [32767, 0, 0, 32767],
    [0, -32767, 0, -32767],
    [23170, 23170, 0, 32767],
  ];
  const w = [65535, 0, 0, 0];
  const bytes = new Int8Array(packVertices(mesh([w, w, w], tangents), 0, null, () => 0));
  const at = (v: number) =>
    Array.from(bytes.subarray(v * VERTEX_BYTES + 40, v * VERTEX_BYTES + 44));
  expect(at(0)).toEqual([127, 0, 0, 127]);
  expect(at(1)).toEqual([0, -127, 0, -127]);
  expect(at(2)).toEqual([90, 90, 0, 127]);
  const bare = new Int8Array(packVertices(mesh([w, w, w]), 0, null, () => 0));
  expect(Array.from(bare.subarray(40, 44))).toEqual([0, 0, 0, 0]);
});
