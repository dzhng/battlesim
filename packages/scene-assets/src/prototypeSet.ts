// The prototype city set: labelled stand-in art for every template of the
// physical catalogue that no real set dresses yet. It is a set like any
// Blender script writes (`blender/city/README.md`), made here without
// Blender: a kit of one unit box, and for each template one row per physical
// part, the box stretched to the part and tinted by the building's category.
// So a town with no real art still draws through the template path, as
// massing. Every template is `status: "prototype"`, which is never coverage.
//
// Generated, not modelled (`asset prototypes`), and deterministic: the files'
// bytes are the catalogue's and the tints'.

import { encodeGlb, type GltfJson } from "./glb.ts";
import { ALL_TIERS } from "./templateLibrary.ts";
import type { TemplateDescriptor, TemplateSetSource } from "./templateSource.ts";

export const PROTOTYPE_SET = "prototype";
export const PROTOTYPE_KIT = "city_kit_prototype";
/** The kit's one module: a box one metre on a side, its base centred on the
 *  origin, tint-masked all over. */
export const PROTOTYPE_MODULE = "unit_box";

type Rgb = readonly [number, number, number];

/** The prototype kit's GLB: `unit_box`, one mesh in every tier. */
export function prototypeKitGlb(): Uint8Array {
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
      { name: PROTOTYPE_MODULE, children: [1] },
      { name: `${PROTOTYPE_MODULE}_shell`, mesh: 0 },
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
    name: `${PROTOTYPE_MODULE}_shell`,
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

/**
 * The prototype set for `descriptors`: each physical part one row, the unit
 * box stretched to it, tinted by the template's category (`tints`, sRGB in
 * 0..1, as `presentation.massing.tints`; a category it lacks takes
 * `default`). The box is the part exactly, so the set's fit is zero.
 */
export function prototypeTemplates(
  descriptors: readonly TemplateDescriptor[],
  tints: Readonly<Record<string, Rgb>>,
): TemplateSetSource {
  return {
    set: PROTOTYPE_SET,
    kit: PROTOTYPE_KIT,
    fit: { side_m: 0, top_m: 0 },
    source: { generator: "packages/scene-assets/src/prototypeSet.ts" },
    modules: [PROTOTYPE_MODULE],
    templates: descriptors.map((descriptor) => {
      const tint = tints[descriptor.category] ?? tints.default;
      if (!tint)
        throw new Error(`no massing tint for category ${descriptor.category}, and no default`);
      const rgb = tint.map((c) => Math.round(Math.max(0, Math.min(1, c)) * 255));
      return {
        status: "prototype",
        descriptor,
        states: {
          intact: descriptor.parts.map((part) => [
            0,
            part.center[0],
            part.center[1],
            part.base_z,
            part.yaw,
            2 * part.half_extents[0],
            2 * part.half_extents[1],
            2 * part.half_extents[2],
            ALL_TIERS,
            ...rgb,
          ]),
        },
      };
    }),
  };
}

/** A set's `templates.json` text: a template to a line, so the same set is
 *  always the same bytes. */
export function templateSetText(set: TemplateSetSource): string {
  const { templates, ...head } = set;
  const fields = Object.entries(head).map(
    ([k, v]) => `  ${JSON.stringify(k)}: ${JSON.stringify(v)}`,
  );
  const rows = templates.map((t) => `    ${JSON.stringify(t)}`);
  return `{\n${fields.join(",\n")},\n  "templates": [\n${rows.join(",\n")}\n  ]\n}\n`;
}
