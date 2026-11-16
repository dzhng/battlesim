// A minimal glTF writer for the scene-assets tests' synthetic GLBs: accessors,
// box meshes, nodes, one skin, animations, embedded PNG textures.

import { encodeGlb, type GltfJson } from "@packages/scene-assets/src/glb.ts";
import type { Mat4, Quat, Vec3 } from "math";
import { inverse, mul, trsMatrix } from "@packages/scene-assets/src/trs.ts";
import { encodePng } from "./png.ts";

interface NodeSpec {
  name: string;
  t?: Vec3;
  r?: Quat;
  s?: Vec3;
  mesh?: number;
  skin?: number;
  children?: number[];
  extras?: Record<string, unknown>;
}

/** A minimal glTF writer: accessors, box meshes, nodes, one skin, animations. */
export class GltfBuilder {
  json: GltfJson = {
    asset: { version: "2.0", generator: "scene-assets synthetic" },
    scene: 0,
    scenes: [{ nodes: [] }],
    nodes: [],
    meshes: [],
    accessors: [],
    bufferViews: [],
    materials: [
      {
        name: "paint",
        pbrMetallicRoughness: {
          baseColorFactor: [0.4, 0.45, 0.3, 1],
          metallicFactor: 0,
          roughnessFactor: 0.8,
        },
      },
    ],
  };
  private chunks: Uint8Array[] = [];
  private length = 0;
  /** Boxes carry UVs (one per face, metres) and, unless false, tangents. */
  surface: { tangents: boolean } | null = null;
  /** Every box vertex's colour (glTF `COLOR_0`, linear rgba), when set. */
  colour: [number, number, number, number] | null = null;

  private view(bytes: Uint8Array): number {
    const padded = new Uint8Array(Math.ceil(bytes.byteLength / 4) * 4);
    padded.set(bytes);
    this.json.bufferViews.push({
      buffer: 0,
      byteOffset: this.length,
      byteLength: bytes.byteLength,
    });
    this.chunks.push(padded);
    this.length += padded.byteLength;
    return this.json.bufferViews.length - 1;
  }

  /** Embed a square RGBA8 image as a PNG; returns its glTF texture index. */
  texture(size: number, pixel: (x: number, y: number) => [number, number, number, number]): number {
    const rgba = new Uint8Array(size * size * 4);
    for (let y = 0; y < size; y++)
      for (let x = 0; x < size; x++) rgba.set(pixel(x, y), (y * size + x) * 4);
    const bufferView = this.view(encodePng(size, size, rgba));
    this.json.images = [...(this.json.images ?? []), { bufferView, mimeType: "image/png" }];
    this.json.textures = [...(this.json.textures ?? []), { source: this.json.images.length - 1 }];
    return this.json.textures.length - 1;
  }

  /** Give material 0 an albedo, a flat normal map and an ORM image, each `size` px. */
  textureMaterial(size: number) {
    const albedo = this.texture(size, (x, y) => [(x * 37) & 255, (y * 91) & 255, 90, 200]);
    const normal = this.texture(size, () => [128, 128, 255, 255]);
    const orm = this.texture(size, (x) => [255, 180 + (x & 7), 0, 255]);
    const m = this.json.materials[0];
    m.pbrMetallicRoughness.baseColorTexture = { index: albedo };
    m.pbrMetallicRoughness.metallicRoughnessTexture = { index: orm };
    m.occlusionTexture = { index: orm };
    m.normalTexture = { index: normal };
  }

  accessor(
    data: Float32Array | Uint16Array | Uint32Array,
    type: string,
    extra: GltfJson = {},
  ): number {
    const view = this.view(new Uint8Array(data.buffer, data.byteOffset, data.byteLength));
    const componentType =
      data instanceof Float32Array ? 5126 : data instanceof Uint16Array ? 5123 : 5125;
    const width = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 }[type]!;
    this.json.accessors.push({
      bufferView: view,
      componentType,
      type,
      count: data.length / width,
      ...extra,
    });
    return this.json.accessors.length - 1;
  }

  /** An axis-aligned box from `min` to `max`, flat-shaded; `influence(p)` skins each vertex. */
  box(
    min: Vec3,
    max: Vec3,
    influence?: (p: Vec3) => [number[], number[]],
    attributes: GltfJson = {},
  ): number {
    const positions: number[] = [];
    const normals: number[] = [];
    const joints: number[] = [];
    const weights: number[] = [];
    const uvs: number[] = [];
    const tangents: number[] = [];
    const indices: number[] = [];
    const faces: [number, number][] = [
      [0, 1],
      [0, -1],
      [1, 1],
      [1, -1],
      [2, 1],
      [2, -1],
    ];
    for (const [axis, sign] of faces) {
      const [u, v] = [(axis + 1) % 3, (axis + 2) % 3];
      const base = positions.length / 3;
      for (const [a, b] of [
        [0, 0],
        [1, 0],
        [1, 1],
        [0, 1],
      ]) {
        const p: Vec3 = [0, 0, 0];
        p[axis] = sign > 0 ? max[axis] : min[axis];
        p[u] = a ? max[u] : min[u];
        p[v] = b ? max[v] : min[v];
        positions.push(...p);
        const n = [0, 0, 0];
        n[axis] = sign;
        normals.push(...n);
        uvs.push(p[u], p[v]);
        const t = [0, 0, 0, sign];
        t[u] = 1;
        tangents.push(...t);
        if (influence) {
          const [j, w] = influence(p);
          joints.push(...j);
          weights.push(...w);
        }
      }
      indices.push(
        ...(sign > 0
          ? [base, base + 1, base + 2, base, base + 2, base + 3]
          : [base, base + 2, base + 1, base, base + 3, base + 2]),
      );
    }
    const attrs: GltfJson = {
      POSITION: this.accessor(Float32Array.from(positions), "VEC3", { min, max }),
      NORMAL: this.accessor(Float32Array.from(normals), "VEC3"),
      ...attributes,
    };
    if (this.surface) {
      attrs.TEXCOORD_0 = this.accessor(Float32Array.from(uvs), "VEC2");
      if (this.surface.tangents) attrs.TANGENT = this.accessor(Float32Array.from(tangents), "VEC4");
    }
    if (this.colour) {
      const count = positions.length / 3;
      const colours = new Float32Array(count * 4);
      for (let v = 0; v < count; v++) colours.set(this.colour, v * 4);
      attrs.COLOR_0 = this.accessor(colours, "VEC4");
    }
    if (influence) {
      attrs.JOINTS_0 = this.accessor(Uint16Array.from(joints), "VEC4");
      attrs.WEIGHTS_0 = this.accessor(Float32Array.from(weights), "VEC4");
    }
    this.json.meshes.push({
      primitives: [
        {
          attributes: attrs,
          indices: this.accessor(Uint16Array.from(indices), "SCALAR"),
          material: 0,
        },
      ],
    });
    return this.json.meshes.length - 1;
  }

  node(spec: NodeSpec): number {
    const node: GltfJson = { name: spec.name };
    if (spec.t) node.translation = spec.t;
    if (spec.r) node.rotation = spec.r;
    if (spec.s) node.scale = spec.s;
    if (spec.mesh !== undefined) node.mesh = spec.mesh;
    if (spec.skin !== undefined) node.skin = spec.skin;
    if (spec.children?.length) node.children = spec.children;
    if (spec.extras) node.extras = spec.extras;
    this.json.nodes.push(node);
    return this.json.nodes.length - 1;
  }

  roots(...roots: number[]) {
    this.json.scenes[0].nodes = roots;
  }

  /** World matrix of a node in glTF space (parents must be linked already). */
  world(index: number): Mat4 {
    const parent = this.json.nodes.findIndex((n: GltfJson) => n.children?.includes(index));
    const n = this.json.nodes[index];
    const local = trsMatrix({
      t: n.translation ?? [0, 0, 0],
      r: n.rotation ?? [0, 0, 0, 1],
      s: n.scale ?? [1, 1, 1],
    });
    return parent < 0 ? local : mul(this.world(parent), local);
  }

  skin(joints: number[]): number {
    const inverseBinds = joints.flatMap((j) => inverse(this.world(j)));
    this.json.skins = [
      ...(this.json.skins ?? []),
      { joints, inverseBindMatrices: this.accessor(Float32Array.from(inverseBinds), "MAT4") },
    ];
    return this.json.skins.length - 1;
  }

  animation(
    name: string,
    channels: {
      node: number;
      path: "rotation" | "translation";
      times: number[];
      values: number[];
    }[],
  ) {
    const samplers: GltfJson[] = [];
    const out: GltfJson[] = [];
    for (const c of channels) {
      samplers.push({
        input: this.accessor(Float32Array.from(c.times), "SCALAR", {
          min: [c.times[0]],
          max: [c.times[c.times.length - 1]],
        }),
        output: this.accessor(Float32Array.from(c.values), c.path === "rotation" ? "VEC4" : "VEC3"),
        interpolation: "LINEAR",
      });
      out.push({ sampler: samplers.length - 1, target: { node: c.node, path: c.path } });
    }
    this.json.animations = [...(this.json.animations ?? []), { name, samplers, channels: out }];
  }

  glb(): Uint8Array {
    const bin = new Uint8Array(this.length);
    let offset = 0;
    for (const chunk of this.chunks) {
      bin.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return encodeGlb(this.json, bin);
  }
}
