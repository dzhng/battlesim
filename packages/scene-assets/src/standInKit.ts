// The stand-in kit: one unit box, which a prop kind with no art of its own is
// drawn as, stretched to the prop's box and tinted by its kind
// (`battle-renderer` `models/propAppearance.ts`). It is a kit in the catalog
// like any other, fetched when a map asks for it, and no template set places
// it: a building is never a stand-in.
//
// Generated, not modelled (`asset stand-in`), and deterministic.

import { encodeGlb, type GltfJson } from "./glb.ts";

/** The kit's appearance in the catalog. */
export const STAND_IN_KIT = "city_kit_prototype";
/** The kit's one module: a box one metre on a side, its base centred on the
 *  origin, tint-masked all over. */
export const STAND_IN_MODULE = "unit_box";

/** The kit's GLB: `unit_box`, one mesh in every tier. */
export function standInKitGlb(): Uint8Array {
  // Engine space (Z up); written as glTF's (x, z, -y).
  const positions: number[] = [];
  const normals: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  const [min, max] = [
    [-0.5, -0.5, 0],
    [0.5, 0.5, 1],
  ];
  for (const [axis, sign] of [
    [0, 1],
    [0, -1],
    [1, 1],
    [1, -1],
    [2, 1],
    [2, -1],
  ]) {
    const [u, v] = [(axis + 1) % 3, (axis + 2) % 3];
    const base = positions.length / 3;
    for (const [a, b] of [
      [0, 0],
      [1, 0],
      [1, 1],
      [0, 1],
    ]) {
      const p = [0, 0, 0];
      p[axis] = sign > 0 ? max[axis] : min[axis];
      p[u] = a ? max[u] : min[u];
      p[v] = b ? max[v] : min[v];
      const n = [0, 0, 0];
      n[axis] = sign;
      positions.push(p[0], p[2], -p[1]);
      normals.push(n[0], n[2], -n[1]);
      uvs.push(a, b);
    }
    indices.push(
      ...(sign > 0
        ? [base, base + 1, base + 2, base, base + 2, base + 3]
        : [base, base + 2, base + 1, base, base + 3, base + 2]),
    );
  }
  const chunks: Uint8Array[] = [];
  let length = 0;
  const json: GltfJson = {
    asset: { version: "2.0", generator: "scene-assets prototype city kit" },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [
      { name: STAND_IN_MODULE, children: [1] },
      { name: `${STAND_IN_MODULE}_shell`, mesh: 0 },
    ],
    meshes: [],
    accessors: [],
    bufferViews: [],
    materials: [
      {
        name: "massing",
        pbrMetallicRoughness: {
          baseColorFactor: [1, 1, 1, 1],
          metallicFactor: 0,
          roughnessFactor: 0.9,
        },
        extras: { tint: 1 },
      },
    ],
  };
  const accessor = (data: Float32Array | Uint16Array, type: string, extra: GltfJson = {}) => {
    const bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    const padded = new Uint8Array(Math.ceil(bytes.byteLength / 4) * 4);
    padded.set(bytes);
    json.bufferViews.push({ buffer: 0, byteOffset: length, byteLength: bytes.byteLength });
    chunks.push(padded);
    length += padded.byteLength;
    json.accessors.push({
      bufferView: json.bufferViews.length - 1,
      componentType: data instanceof Float32Array ? 5126 : 5123,
      type,
      count: data.length / { SCALAR: 1, VEC2: 2, VEC3: 3 }[type]!,
      ...extra,
    });
    return json.accessors.length - 1;
  };
  json.meshes.push({
    name: `${STAND_IN_MODULE}_shell`,
    primitives: [
      {
        attributes: {
          POSITION: accessor(Float32Array.from(positions), "VEC3", {
            min: [-0.5, 0, -0.5],
            max: [0.5, 1, 0.5],
          }),
          NORMAL: accessor(Float32Array.from(normals), "VEC3"),
          TEXCOORD_0: accessor(Float32Array.from(uvs), "VEC2"),
        },
        indices: accessor(Uint16Array.from(indices), "SCALAR"),
        material: 0,
      },
    ],
  });
  const bin = new Uint8Array(length);
  let at = 0;
  for (const chunk of chunks) {
    bin.set(chunk, at);
    at += chunk.byteLength;
  }
  return encodeGlb(json, bin);
}
