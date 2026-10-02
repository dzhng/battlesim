// glTF → an engine-space scene: the node tree with world transforms, triangle
// primitives, the skin, animation channels and materials. The basis
// conversion (glTF Y up → engine Z up, then the source's declared yaw) is one
// matrix above every root, applied here at bake time and never at load.

import { AssetError, parseGlb, type GltfJson } from "./glb.ts";
import { mat4, quat, type Mat4, type Quat, type Vec3 } from "math";
import { decomposeTrs, isUniformScale, mul, trsMatrix, type Trs } from "./trs.ts";
import {
  CHANNEL_FORMAT,
  TEXTURE_CHANNELS,
  TEXTURE_MAX_PX,
  TEXTURE_MIN_PX,
  type Finding,
  type Material,
  type Texture,
  type TextureChannel,
} from "./schema.ts";
import { sourceCoverage, sourceInterior } from "./material.ts";
import { bakeTexture, decodePng, validTextureSize, type Rgba8 } from "./texture.ts";

export interface SceneNode {
  index: number;
  name: string;
  parent: number;
  children: number[];
  local: Trs;
  world: Mat4; // engine space, rest pose
  live: boolean; // reachable from the default scene's roots
  mesh: number | null;
  skin: number | null;
  extras: Record<string, unknown>;
}

export interface Primitive {
  positions: Float32Array;
  normals: Float32Array;
  tangents: Float32Array | null; // xyzw, w the bitangent's sign
  uvs: Float32Array;
  colors: Float32Array; // rgba
  joints: Uint16Array | null; // skin slots
  weights: Float32Array | null;
  indices: Uint32Array;
  material: number;
}

export interface Channel {
  node: number;
  path: "translation" | "rotation" | "scale";
  times: Float32Array;
  values: Float32Array;
  step: boolean;
}

export interface SceneImage {
  name: string;
  mime: string;
  bytes: Uint8Array;
}

/** A source material: the bundle's fields, the images its channels sample,
 *  and (once `bindTextures` ran) the textures baked from them. */
export interface SceneMaterial extends Omit<Material, "textures"> {
  images?: Partial<Record<TextureChannel, number>>;
  sources?: Partial<Record<TextureChannel, Texture>>;
}

export interface Scene {
  nodes: SceneNode[];
  roots: number[];
  meshes: { name: string; primitives: Primitive[] }[];
  skins: { joints: number[]; inverseBinds: Mat4[] }[];
  animations: { name: string; channels: Channel[] }[];
  materials: SceneMaterial[];
  /** Embedded images, undecoded (`bindTextures` decodes what materials sample). */
  images: SceneImage[];
  /** Engine basis: glTF Y up to Z up, then the declared yaw about Z. */
  basis: { matrix: Mat4; rotation: Quat };
}

const COMPONENT_SIZE: Record<number, number> = {
  5120: 1,
  5121: 1,
  5122: 2,
  5123: 2,
  5125: 4,
  5126: 4,
};
const TYPE_WIDTH: Record<string, number> = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 };
const NORMALIZED_SCALE: Record<number, number> = { 5120: 127, 5121: 255, 5122: 32767, 5123: 65535 };

/** glTF (x, y, z) → Z up (x, −z, y), then yaw about +Z. */
export function engineBasis(yawDeg: number): { matrix: Mat4; rotation: Quat } {
  const yUpToZUp = quat.setAxisAngle(quat.create(), [1, 0, 0], Math.PI / 2);
  const yaw = quat.setAxisAngle(quat.create(), [0, 0, 1], (yawDeg * Math.PI) / 180);
  const rotation = quat.multiply(quat.create(), yaw, yUpToZUp);
  return { matrix: mat4.fromQuat(mat4.create(), rotation), rotation };
}

export function importScene(
  bytes: Uint8Array,
  path: string,
  yawDeg: number,
): { scene: Scene | null; findings: Finding[] } {
  const findings: Finding[] = [];
  const add = (
    code: Finding["code"],
    message: string,
    fix: string,
    severity: Finding["severity"] = "error",
  ) => findings.push({ code, severity, message: `${path}: ${message}`, fix });
  let json: GltfJson;
  let bin: Uint8Array | null;
  try {
    ({ json, bin } = parseGlb(bytes, path));
  } catch (error) {
    if (!(error instanceof AssetError)) throw error;
    findings.push({ code: error.code, severity: "error", message: error.message, fix: error.fix });
    return { scene: null, findings };
  }

  for (const [i, buffer] of (json.buffers ?? []).entries())
    if (buffer.uri !== undefined)
      add(
        "structure.external_buffer",
        `buffer ${i} references "${String(buffer.uri).slice(0, 60)}"`,
        "export a self-contained .glb with every buffer embedded",
      );
  for (const [i, image] of (json.images ?? []).entries())
    if (image.uri !== undefined)
      add(
        "structure.external_buffer",
        `image ${i} references "${String(image.uri).slice(0, 60)}"`,
        "embed images in the .glb, or drop them",
      );
  if ((json.extensionsRequired ?? []).length)
    add(
      "structure.unsupported",
      `requires extensions ${json.extensionsRequired.join(", ")}`,
      "export uncompressed core glTF",
    );
  for (const [i, accessor] of (json.accessors ?? []).entries())
    if (accessor.sparse || accessor.bufferView === undefined)
      add(
        "structure.unsupported",
        `accessor ${i} is sparse or has no buffer view`,
        "export dense buffers",
      );
  if (findings.some((f) => f.severity === "error")) return { scene: null, findings };

  const readAccessor = (index: number): Float64Array => {
    const accessor = json.accessors?.[index];
    const view = json.bufferViews?.[accessor?.bufferView];
    if (!accessor || !view || !bin) throw new Error(`accessor ${index} has no data`);
    const width = TYPE_WIDTH[accessor.type];
    const size = COMPONENT_SIZE[accessor.componentType];
    if (!width || !size) throw new Error(`accessor ${index} has an unknown type`);
    const dv = new DataView(bin.buffer, bin.byteOffset, bin.byteLength);
    const stride = view.byteStride || width * size;
    const base = (view.byteOffset ?? 0) + (accessor.byteOffset ?? 0);
    const out = new Float64Array(accessor.count * width);
    const scale = accessor.normalized ? NORMALIZED_SCALE[accessor.componentType] : 1;
    for (let i = 0; i < accessor.count; i++)
      for (let c = 0; c < width; c++) {
        const o = base + i * stride + c * size;
        let v: number;
        switch (accessor.componentType) {
          case 5120:
            v = dv.getInt8(o);
            break;
          case 5121:
            v = dv.getUint8(o);
            break;
          case 5122:
            v = dv.getInt16(o, true);
            break;
          case 5123:
            v = dv.getUint16(o, true);
            break;
          case 5125:
            v = dv.getUint32(o, true);
            break;
          default:
            v = dv.getFloat32(o, true);
        }
        out[i * width + c] = accessor.normalized ? Math.max(-1, v / scale) : v;
      }
    return out;
  };

  const basis = engineBasis(yawDeg);
  const gltfNodes: GltfJson[] = json.nodes ?? [];
  const parent = Array.from({ length: gltfNodes.length }, () => -1);
  gltfNodes.forEach((node, i) => {
    for (const child of node.children ?? []) parent[child] = i;
  });
  const nodes: SceneNode[] = gltfNodes.map((node, index) => {
    let local: Trs;
    if (node.matrix) {
      const trs = decomposeTrs(node.matrix as Mat4);
      if (!trs) {
        add(
          "structure.scale",
          `node "${node.name ?? index}" has shear or mirroring`,
          "apply transforms in Blender before export",
        );
        local = {
          t: [node.matrix[12], node.matrix[13], node.matrix[14]],
          r: [0, 0, 0, 1],
          s: [1, 1, 1],
        };
      } else local = trs;
    } else
      local = {
        t: (node.translation ?? [0, 0, 0]) as Vec3,
        r: (node.rotation ?? [0, 0, 0, 1]) as Quat,
        s: (node.scale ?? [1, 1, 1]) as Vec3,
      };
    if (!isUniformScale(local.s))
      add(
        "structure.scale",
        `node "${node.name ?? index}" has a nonuniform, mirrored or zero scale`,
        "apply scale in Blender before export",
      );
    return {
      index,
      name: node.name ?? `node_${index}`,
      parent: parent[index],
      children: node.children ?? [],
      local,
      world: basis.matrix,
      live: false,
      mesh: node.mesh ?? null,
      skin: node.skin ?? null,
      extras: node.extras && typeof node.extras === "object" ? node.extras : {},
    };
  });
  const sceneRoots: number[] =
    json.scenes?.[json.scene ?? 0]?.nodes ?? nodes.filter((n) => n.parent < 0).map((n) => n.index);
  const visit = (index: number, parentWorld: Mat4) => {
    const node = nodes[index];
    node.world = mul(parentWorld, trsMatrix(node.local));
    node.live = true;
    for (const child of node.children) visit(child, node.world);
  };
  for (const root of sceneRoots) visit(root, basis.matrix);

  const meshes = (json.meshes ?? []).map((mesh: GltfJson, meshIndex: number) => ({
    name: mesh.name ?? `mesh_${meshIndex}`,
    primitives: (mesh.primitives ?? []).flatMap((primitive: GltfJson, p: number): Primitive[] => {
      const label = `mesh "${mesh.name ?? meshIndex}" primitive ${p}`;
      const a = primitive.attributes ?? {};
      if (primitive.mode !== undefined && primitive.mode !== 4) {
        add(
          "structure.unsupported",
          `${label} is not a triangle list`,
          "export triangulated meshes",
        );
        return [];
      }
      if (primitive.targets?.length)
        add(
          "structure.unsupported",
          `${label} has morph targets`,
          "bake deformation into the skeleton",
        );
      if (Object.keys(a).some((k) => /^(JOINTS|WEIGHTS)_[1-9]/.test(k)))
        add(
          "structure.weights",
          `${label} has more than four influences per vertex`,
          "limit weights to 4 in Blender (Weights → Limit Total) and normalise",
        );
      if (a.POSITION === undefined || a.NORMAL === undefined) {
        add(
          "structure.attributes",
          `${label} lacks ${a.POSITION === undefined ? "POSITION" : "NORMAL"}`,
          "export positions and normals",
        );
        return [];
      }
      const positions = Float32Array.from(readAccessor(a.POSITION));
      const count = positions.length / 3;
      const colors = new Float32Array(count * 4).fill(1);
      if (a.COLOR_0 !== undefined) {
        const source = readAccessor(a.COLOR_0);
        const width = source.length / count;
        for (let v = 0; v < count; v++)
          for (let c = 0; c < width; c++) colors[v * 4 + c] = source[v * width + c];
      }
      const indices =
        primitive.indices !== undefined
          ? Uint32Array.from(readAccessor(primitive.indices))
          : Uint32Array.from({ length: count }, (_, i) => i);
      return [
        {
          positions,
          normals: Float32Array.from(readAccessor(a.NORMAL)),
          tangents: a.TANGENT !== undefined ? Float32Array.from(readAccessor(a.TANGENT)) : null,
          uvs:
            a.TEXCOORD_0 !== undefined
              ? Float32Array.from(readAccessor(a.TEXCOORD_0))
              : new Float32Array(count * 2),
          colors,
          joints: a.JOINTS_0 !== undefined ? Uint16Array.from(readAccessor(a.JOINTS_0)) : null,
          weights: a.WEIGHTS_0 !== undefined ? Float32Array.from(readAccessor(a.WEIGHTS_0)) : null,
          indices,
          material: primitive.material ?? -1,
        },
      ];
    }),
  }));

  const skins = (json.skins ?? []).map((skin: GltfJson) => {
    const joints: number[] = skin.joints ?? [];
    const matrices =
      skin.inverseBindMatrices !== undefined ? readAccessor(skin.inverseBindMatrices) : null;
    return {
      joints,
      inverseBinds: joints.map((_, i) =>
        matrices ? (Array.from(matrices.subarray(i * 16, i * 16 + 16)) as Mat4) : mat4.create(),
      ),
    };
  });

  const animations = (json.animations ?? []).map((animation: GltfJson, i: number) => ({
    name: animation.name ?? `clip_${i}`,
    channels: (animation.channels ?? []).flatMap((channel: GltfJson): Channel[] => {
      const sampler = animation.samplers[channel.sampler];
      const path = channel.target?.path;
      if (!["translation", "rotation", "scale"].includes(path)) {
        add(
          "structure.unsupported",
          `clip "${animation.name}" animates ${path}`,
          "bake skeletal TRS animation",
        );
        return [];
      }
      if (sampler.interpolation === "CUBICSPLINE") {
        add(
          "structure.unsupported",
          `clip "${animation.name}" uses CUBICSPLINE`,
          "bake sampled LINEAR tracks in Blender",
        );
        return [];
      }
      return [
        {
          node: channel.target.node,
          path,
          times: Float32Array.from(readAccessor(sampler.input)),
          values: Float32Array.from(readAccessor(sampler.output)),
          step: sampler.interpolation === "STEP",
        },
      ];
    }),
  }));

  const images: SceneImage[] = (json.images ?? []).map((image: GltfJson, i: number) => {
    const view = json.bufferViews?.[image.bufferView];
    return {
      name: image.name ?? `image_${i}`,
      mime: image.mimeType ?? "",
      bytes:
        view && bin
          ? bin.subarray(view.byteOffset ?? 0, (view.byteOffset ?? 0) + view.byteLength)
          : new Uint8Array(0),
    };
  });
  const imageOf = (ref: GltfJson | undefined): number | undefined => {
    const source = ref === undefined ? undefined : json.textures?.[ref.index]?.source;
    return typeof source === "number" && images[source] ? source : undefined;
  };
  const materials: SceneMaterial[] = (json.materials ?? []).map((m: GltfJson, i: number) => {
    const pbr = m.pbrMetallicRoughness ?? {};
    const name = m.name ?? `material_${i}`;
    const channels: Partial<Record<TextureChannel, number>> = {};
    const albedo = imageOf(pbr.baseColorTexture);
    const normal = imageOf(m.normalTexture);
    const orm = imageOf(pbr.metallicRoughnessTexture);
    const occlusion = imageOf(m.occlusionTexture);
    if (albedo !== undefined) channels.albedo = albedo;
    if (normal !== undefined) channels.normal = normal;
    if (orm !== undefined) channels.orm = orm;
    if (occlusion !== undefined && occlusion !== orm)
      add(
        "structure.texture",
        `material "${name}" has an occlusion image apart from its metallic-roughness image`,
        "pack occlusion, roughness and metalness into one ORM image (R, G, B)",
      );
    const wear = m.extras?.wear;
    const colourScale = Number(m.extras?.colour_scale);
    const interior = sourceInterior(m, name, add);
    return {
      name,
      base_color: (pbr.baseColorFactor ?? [1, 1, 1, 1]) as Material["base_color"],
      metallic: pbr.metallicFactor ?? 1,
      roughness: pbr.roughnessFactor ?? 1,
      tint: Math.max(0, Math.min(1, Number(m.extras?.tint ?? 0) || 0)),
      ...(Array.isArray(wear) && wear.length === 4 && wear.every((x) => typeof x === "number")
        ? { wear: wear as Material["wear"] }
        : {}),
      ...(colourScale > 0 && colourScale !== 1 ? { colour_scale: colourScale } : {}),
      coverage: sourceCoverage(m, name, add),
      ...(interior ? { interior } : {}),
      ...(Object.keys(channels).length ? { images: channels } : {}),
    };
  });

  return {
    scene: { nodes, roots: sceneRoots, meshes, skins, animations, materials, images, basis },
    findings,
  };
}

/**
 * Decode every image a material samples and bind each channel's texture to
 * its material (`SceneMaterial.sources`), with its mip chain and address.
 * Async, since PNG inflates through the platform's streams; a scene that is
 * never bound builds without textures.
 */
export async function bindTextures(scene: Scene, path: string): Promise<Finding[]> {
  const findings: Finding[] = [];
  const decoded = new Map<number, Rgba8 | null>();
  const decode = async (index: number) => {
    if (decoded.has(index)) return decoded.get(index)!;
    const image = scene.images[index];
    let out: Rgba8 | null = null;
    if (image.mime !== "image/png")
      findings.push({
        code: "structure.texture",
        severity: "error",
        message: `${path}: image "${image.name}" is ${image.mime || "untyped"}, not PNG`,
        fix: "embed textures as PNG",
      });
    else
      try {
        out = await decodePng(image.bytes);
      } catch (error) {
        findings.push({
          code: "structure.texture",
          severity: "error",
          message: `${path}: image "${image.name}" does not decode (${(error as Error).message})`,
          fix: "embed 8-bit, non-interlaced PNGs",
        });
      }
    if (out && !validTextureSize(out.width, out.height)) {
      findings.push({
        code: "texture.size",
        severity: "error",
        message: `${path}: image "${image.name}" is ${out.width}×${out.height}; textures are square powers of two from ${TEXTURE_MIN_PX} to ${TEXTURE_MAX_PX} px`,
        fix: `bake it square at a power of two, at most ${TEXTURE_MAX_PX} px`,
      });
      out = null;
    }
    decoded.set(index, out);
    return out;
  };
  for (const material of scene.materials) {
    if (!material.images) continue;
    const bound: Partial<Record<TextureChannel, Texture>> = {};
    for (const channel of TEXTURE_CHANNELS) {
      const index = material.images[channel];
      if (index === undefined) continue;
      const image = await decode(index);
      if (image) bound[channel] = await bakeTexture(image, channel, CHANNEL_FORMAT[channel]);
    }
    material.sources = bound;
  }
  return findings;
}

/** Walk up from a node; the first ancestor (excluding itself) matching `test`. */
export function nearestAncestor(
  scene: Scene,
  index: number,
  test: (node: SceneNode) => boolean,
): number {
  for (let p = scene.nodes[index].parent; p >= 0; p = scene.nodes[p].parent)
    if (test(scene.nodes[p])) return p;
  return -1;
}

/** The tier a mesh node's geometry belongs to: `_LOD<n>` suffix, or every tier when unsuffixed. */
export function meshTier(name: string): number | null {
  const match = name.match(/_LOD(\d+)$/);
  return match ? Number(match[1]) : null;
}
