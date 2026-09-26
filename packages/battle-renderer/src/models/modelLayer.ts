// The models layer: real appearance bundles in the battle frame. One vertex
// path draws all three bundle kinds, because each is a palette-skinned mesh:
//
// - skinned bodies take their palette from the pose kernel (GPU), per soldier;
// - articulated vehicles are rigidly skinned, one palette matrix per node,
//   posed on the CPU from the pose inputs (`articulate`);
// - static buildings use the palette's identity slot.
//
// Rewritten from reading ~/dev/game battle-renderer/src/world/crowd.ts
// (reuse manifest, technique): per-appearance, per-tier vertex and index
// buffers, instances packed per draw, a palette storage buffer read by the
// vertex stage, and a caster variant for the sun's cascades. The layer is lit,
// shadowed and fogged exactly like the rest of the world (environment `shade`,
// `sampleSunShadow`, `FogTerm`) and draws in the frame's own passes.

import { tgpu, d, std } from "typegpu";
import { mat4, type Mat4 } from "math";
import type {
  ArticulatedBundle,
  Bounds,
  Bundle,
  MeshData,
  SkeletonClips,
} from "@packages/scene-assets/src/schema";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  REST_ARTICULATION,
  articulate,
  articulationRig,
  restLocals,
  trackScroll,
  type ArticulationRig,
} from "@packages/scene-assets/src/articulation";
import { farPoseBounds, worldTransforms } from "@packages/scene-assets/src/pose";
import type { Trs } from "@packages/scene-assets/src/trs";
import { typegpuCameraLayout } from "../world/camera";
import type { EnvironmentFrame } from "../frame/environmentFrame";
import { fogCoverage, fogTerm } from "../frame/fogTerm";
import { WORLD_OUT } from "../frame/targets";
import type { GpuRegistry, GpuSlot } from "../frame/registry";
import { buildClipTable, clipFrames, type ClipFrames, type ClipTable } from "./clipTable";
import { CONTROL_WORDS, poseKernelWgsl, writeControl } from "./poseKernel";
import type { ModelInstance, ModelPose } from "./modelInstances";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** 48 bytes: position, normal, uv, colour, four joints and weights, and
 *  (global material index, track side: 0 none, 1 left, 2 right). */
export const ModelVertex = d.unstruct({
  position: d.float32x3,
  normal: d.snorm16x4,
  uv: d.float32x2,
  color: d.unorm8x4,
  joints: d.uint8x4,
  weights: d.unorm16x4,
  material: d.uint16x2,
});
const VERTEX_BYTES = 48;
/** Per model: x, y, z, yaw; palette base, left and right track scroll, highlight;
 *  the side's tint (rgb) on tint-masked materials. */
export const ModelRecord = d.unstruct({
  placement: d.float32x4,
  data: d.float32x4,
  tint: d.float32x4,
});
const RECORD_FLOATS = 12;
export const modelVertexLayout = tgpu.vertexLayout(d.disarrayOf(ModelVertex));
export const modelRecordLayout = tgpu.vertexLayout(d.disarrayOf(ModelRecord), "instance");
export const modelAttribs = { ...modelVertexLayout.attrib, ...modelRecordLayout.attrib };

export const modelLayout = tgpu.bindGroupLayout({
  palette: {
    storage: (n: number) => d.arrayOf(d.mat4x4f, n),
    access: "readonly",
    visibility: ["vertex"],
  },
  materials: {
    storage: (n: number) => d.arrayOf(d.vec4f, n),
    access: "readonly",
    visibility: ["fragment"],
  },
});

/** Palette slot 0 is the identity every static model uses. */
const IDENTITY_SLOT = 0;
/** Selection glow, as the proxies' (worldPass `HIGHLIGHT`). */
const HIGHLIGHT = [0.95, 0.8, 0.2] as const;
/** A model drawn without a side keeps its authored colours. */
const NO_TINT = [1, 1, 1] as const;
/** Track links: the dark half of each link's pitch. */
const TRACK_LINK_SHADE = 0.55;

export const modelVertex = tgpu.vertexFn({
  in: {
    position: d.vec3f,
    normal: d.vec4f,
    uv: d.vec2f,
    color: d.vec4f,
    joints: d.vec4u,
    weights: d.vec4f,
    material: d.vec2u,
    placement: d.vec4f,
    data: d.vec4f,
    tint: d.vec4f,
  },
  out: {
    // Invariant, so the depth prepass and the colour pass agree exactly.
    clip: d.invariant(d.builtin.position) as unknown as typeof d.builtin.position,
    world: d.vec3f,
    normal: d.vec3f,
    color: d.vec4f,
    uv: d.vec2f,
    material: d.interpolate("flat", d.u32),
    track: d.f32,
    highlight: d.f32,
    tint: d.vec3f,
  },
})((v) => {
  "use gpu";
  const base = d.u32(v.data.x);
  const p = d.vec4f(v.position, 1);
  const n = d.vec4f(v.normal.xyz, 0);
  const m0 = modelLayout.$.palette[base + v.joints.x];
  const m1 = modelLayout.$.palette[base + v.joints.y];
  const m2 = modelLayout.$.palette[base + v.joints.z];
  const m3 = modelLayout.$.palette[base + v.joints.w];
  const skinned = std.add(
    std.add(std.mul(std.mul(m0, p), v.weights.x), std.mul(std.mul(m1, p), v.weights.y)),
    std.add(std.mul(std.mul(m2, p), v.weights.z), std.mul(std.mul(m3, p), v.weights.w)),
  );
  const skinnedNormal = std.add(
    std.add(std.mul(std.mul(m0, n), v.weights.x), std.mul(std.mul(m1, n), v.weights.y)),
    std.add(std.mul(std.mul(m2, n), v.weights.z), std.mul(std.mul(m3, n), v.weights.w)),
  );
  const c = std.cos(v.placement.w);
  const s = std.sin(v.placement.w);
  const world = d.vec3f(
    skinned.x * c - skinned.y * s + v.placement.x,
    skinned.x * s + skinned.y * c + v.placement.y,
    skinned.z + v.placement.z,
  );
  const normal = d.vec3f(
    skinnedNormal.x * c - skinnedNormal.y * s,
    skinnedNormal.x * s + skinnedNormal.y * c,
    skinnedNormal.z,
  );
  let track = d.f32(0);
  if (v.material.y === 1) {
    track = 1 + std.fract(v.uv.x - v.data.y);
  }
  if (v.material.y === 2) {
    track = 1 + std.fract(v.uv.x - v.data.z);
  }
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal,
    color: v.color,
    uv: v.uv,
    material: v.material.x,
    track,
    highlight: v.data.w,
    tint: v.tint.xyz,
  };
});

const modelVaryings = {
  clip: d.builtin.position,
  world: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  uv: d.vec2f,
  material: d.interpolate("flat", d.u32),
  track: d.f32,
  highlight: d.f32,
  tint: d.vec3f,
};

/** Linear albedo: the vertex colour (glTF COLOR_0 is linear) times the
 *  material's base colour, recoloured by the side's tint as far as the
 *  material's tint mask says, and darkened on a track's link gaps. */
const modelAlbedo = tgpu.fn(
  [d.vec4f, d.u32, d.f32, d.vec3f],
  d.vec3f,
)((color, material, track, tint) => {
  "use gpu";
  const base = modelLayout.$.materials[material * 2];
  const mask = modelLayout.$.materials[material * 2 + 1].z;
  let albedo = std.mul(std.mul(color.xyz, base.xyz), std.mix(d.vec3f(1), tint, mask));
  // A track's scroll arrives as 1 + fract(u − offset): links split light and dark.
  if (track >= 1 && track - 1 < 0.5) {
    albedo = std.mul(albedo, TRACK_LINK_SHADE);
  }
  return albedo;
});

export function createModelFragments(environment: EnvironmentFrame) {
  const lit = tgpu.fragmentFn({ in: modelVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    const albedo = modelAlbedo(v.color, v.material, v.track, v.tint);
    const surface = modelLayout.$.materials[v.material * 2 + 1];
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const shaded = environment.shade(
      albedo,
      d.vec3f(0),
      surface.y,
      0,
      surface.x,
      1,
      n,
      v.world,
      sun,
      eye,
    );
    const glow = std.mul(d.vec3f(HIGHLIGHT[0], HIGHLIGHT[1], HIGHLIGHT[2]), v.highlight * 0.7);
    const seen = fogTerm(v.world, n, v.clip.xy, false);
    return { color: d.vec4f(std.add(shaded.xyz, glow), 1), fog: fogCoverage(seen, 1) };
  });
  /** The impostor bake's targets: display-encoded albedo with coverage, and
   *  the world normal (mapped to 0..1) with coverage. Unlit: the battle
   *  relights impostors itself. */
  const impostor = tgpu.fragmentFn({
    in: modelVaryings,
    out: { albedo: d.vec4f, normal: d.vec4f },
  })((v) => {
    "use gpu";
    const albedo = modelAlbedo(v.color, v.material, v.track, v.tint);
    const n = std.normalize(v.normal);
    return {
      albedo: d.vec4f(std.pow(std.max(albedo, d.vec3f(0)), d.vec3f(1 / 2.2)), 1),
      normal: d.vec4f(std.add(std.mul(n, 0.5), d.vec3f(0.5)), 1),
    };
  });
  return { lit, impostor };
}

/** One tier of one appearance on the GPU, with a draw range per state. */
interface TierMesh {
  vertices: GPUBuffer;
  indices: GPUBuffer;
  triangles: number;
  /** Index ranges by static state ("" for every other kind). */
  ranges: Map<string, { first: number; count: number }>;
}

interface GpuAppearance {
  name: string;
  bundle: Exclude<Bundle, SkeletonClips>;
  tiers: TierMesh[];
  /** Palette matrices one model takes. */
  joints: number;
  /** Skinned: the body's rows in the joint table and its clip table. */
  jointBase: number;
  clips: ClipTable | null;
  clipBase: number;
  /** Articulated: the rig and reusable locals. */
  rig: ArticulationRig | null;
  locals: Trs[];
}

/** Fold a MeshData into the 48-byte vertex layout. */
function packVertices(
  mesh: MeshData,
  materialBase: number,
  rigid: number | null,
  trackSide: (vertex: number) => number,
): ArrayBuffer {
  const count = mesh.positions.length / 3;
  const bytes = new ArrayBuffer(count * VERTEX_BYTES);
  const f32 = new Float32Array(bytes);
  const i16 = new Int16Array(bytes);
  const u8 = new Uint8Array(bytes);
  const u16 = new Uint16Array(bytes);
  const material = new Uint16Array(count);
  for (const draw of mesh.draws)
    for (let i = draw.first; i < draw.first + draw.count; i++)
      material[mesh.indices[i]] = materialBase + draw.material;
  for (let v = 0; v < count; v++) {
    const o = v * VERTEX_BYTES;
    f32[o / 4] = mesh.positions[v * 3];
    f32[o / 4 + 1] = mesh.positions[v * 3 + 1];
    f32[o / 4 + 2] = mesh.positions[v * 3 + 2];
    for (let c = 0; c < 4; c++) i16[(o + 12) / 2 + c] = mesh.normals[v * 4 + c];
    f32[(o + 20) / 4] = mesh.uvs[v * 2];
    f32[(o + 20) / 4 + 1] = mesh.uvs[v * 2 + 1];
    for (let c = 0; c < 4; c++) u8[o + 28 + c] = mesh.colors[v * 4 + c];
    for (let c = 0; c < 4; c++) {
      u8[o + 32 + c] = rigid ?? (mesh.joints ? mesh.joints[v * 4 + c] : 0);
      u16[(o + 36) / 2 + c] =
        rigid !== null ? (c === 0 ? 65535 : 0) : (mesh.weights?.[v * 4 + c] ?? 0);
    }
    if (rigid === null && !mesh.weights) u16[(o + 36) / 2] = 65535;
    u16[(o + 44) / 2] = material[v];
    u16[(o + 44) / 2 + 1] = trackSide(v);
  }
  return bytes;
}

function concatBytes(parts: ArrayBuffer[]): ArrayBuffer {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.byteLength, 0));
  let at = 0;
  for (const p of parts) {
    out.set(new Uint8Array(p), at);
    at += p.byteLength;
  }
  return out.buffer;
}

const VERTEX_USAGE = 0x20 | 0x08; // VERTEX | COPY_DST
const INDEX_USAGE = 0x10 | 0x08; // INDEX | COPY_DST
const STORAGE_USAGE = 0x80 | 0x08; // STORAGE | COPY_DST
const PALETTE_USAGE = 0x80 | 0x08 | 0x04; // STORAGE | COPY_DST | COPY_SRC

function tierCount(bundle: Exclude<Bundle, SkeletonClips>): number {
  if (bundle.kind === "skinned") return bundle.tiers.length;
  if (bundle.kind === "articulated") return bundle.nodes[0]?.tiers.length ?? 0;
  return bundle.states[0]?.tiers.length ?? 0;
}

/** A draw: one appearance tier and state range, over a run of records. */
interface DrawRun {
  mesh: TierMesh;
  first: number;
  count: number;
  firstInstance: number;
  instances: number;
}

export interface ModelStats {
  /** Appearance names installed on the GPU. */
  installed: string[];
  appearances: number;
  instances: number;
  triangles: number;
  draws: number;
  paletteMatrices: number;
  skinned: number;
}

export async function createModelLayer(root: Root, registry: GpuRegistry) {
  const device = root.device;
  const buffer = (label: string, size: number, usage: number) =>
    device.createBuffer({ label, size: Math.max(16, Math.ceil(size / 16) * 16), usage });
  const upload = (
    label: string,
    data: ArrayBuffer | ArrayBufferView<ArrayBuffer>,
    usage: number,
  ) => {
    const bytes = ArrayBuffer.isView(data)
      ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
      : new Uint8Array(data);
    const b = buffer(label, bytes.byteLength, usage);
    device.queue.writeBuffer(b, 0, bytes);
    return b;
  };

  // Everything built from the installed appearances lives in one scope,
  // released whole when a new generation installs.
  let scope: GpuRegistry | null = null;
  let appearances = new Map<string, GpuAppearance>();
  let skeletons = new Map<string, SkeletonClips>();
  let materials: GPUBuffer | null = null;
  let jointTables: { parents: GPUBuffer; inverseBinds: GPUBuffer; samples: GPUBuffer } | null =
    null;
  let kernel: GPUComputePipeline | null = null;

  const palette: GpuSlot<GPUBuffer> = registry.slot();
  const controls: GpuSlot<GPUBuffer> = registry.slot();
  const records: GpuSlot<GPUBuffer> = registry.slot();
  const dispatch = registry.buffer({ label: "pose-dispatch", size: 16, usage: 0x40 | 0x08 });
  let paletteCapacity = 0;
  let controlCapacity = 0;
  let recordCapacity = 0;
  let renderGroup: GPUBindGroup | null = null;
  let computeGroup: GPUBindGroup | null = null;

  let paletteStaging = new Float32Array(16);
  let controlWords = new Uint32Array(CONTROL_WORDS);
  let controlFloats = new Float32Array(controlWords.buffer);
  let recordStaging = new Float32Array(RECORD_FLOATS);
  let runs: DrawRun[] = [];
  let skinnedCount = 0;
  let paletteUsed = 1;
  let current: readonly ModelInstance[] = [];
  const stats: ModelStats = {
    installed: [],
    appearances: 0,
    instances: 0,
    triangles: 0,
    draws: 0,
    paletteMatrices: 0,
    skinned: 0,
  };

  const rebind = () => {
    if (!materials || !palette.current) return;
    renderGroup = device.createBindGroup({
      label: "models",
      layout: root.unwrap(modelLayout),
      entries: [
        { binding: 0, resource: { buffer: palette.current } },
        { binding: 1, resource: { buffer: materials } },
      ],
    });
    computeGroup =
      kernel && jointTables && controls.current
        ? device.createBindGroup({
            label: "pose-kernel",
            layout: kernel.getBindGroupLayout(0),
            entries: [
              { binding: 0, resource: { buffer: jointTables.samples } },
              { binding: 1, resource: { buffer: jointTables.parents } },
              { binding: 2, resource: { buffer: jointTables.inverseBinds } },
              { binding: 3, resource: { buffer: controls.current } },
              { binding: 4, resource: { buffer: palette.current } },
              { binding: 5, resource: { buffer: dispatch } },
            ],
          })
        : null;
  };

  const growPalette = (matrices: number) => {
    if (matrices <= paletteCapacity) return;
    paletteCapacity = Math.max(64, matrices * 2);
    palette.set(buffer("model-palette", paletteCapacity * 64, PALETTE_USAGE));
    paletteStaging = new Float32Array(paletteCapacity * 16);
    rebind();
  };
  const growControls = (count: number) => {
    if (count <= controlCapacity) return;
    controlCapacity = Math.max(16, count * 2);
    controls.set(buffer("pose-controls", controlCapacity * CONTROL_WORDS * 4, STORAGE_USAGE));
    controlWords = new Uint32Array(controlCapacity * CONTROL_WORDS);
    controlFloats = new Float32Array(controlWords.buffer);
    rebind();
  };
  const growRecords = (count: number) => {
    if (count <= recordCapacity) return;
    recordCapacity = Math.max(16, count * 2);
    records.set(buffer("model-records", recordCapacity * RECORD_FLOATS * 4, VERTEX_USAGE));
    recordStaging = new Float32Array(recordCapacity * RECORD_FLOATS);
  };
  growPalette(1);
  growControls(1);
  growRecords(1);

  async function install(installed: InstalledAppearances | null) {
    // Build the new generation beside the old one and swap at the end, so a
    // frame drawn meanwhile still binds live buffers.
    const next = registry.scope();
    const built = new Map<string, GpuAppearance>();
    let nextJoints = 0;
    const own = <T extends GPUBuffer>(b: T) => next.own(b);
    const materialRows: number[] = [];
    const parents: number[] = [];
    const inverseBinds: number[] = [];
    const sampleParts: Float32Array[] = [];
    let sampleFloats = 0;
    for (const [name, { bundle }] of installed?.appearances ?? []) {
      const materialBase = materialRows.length / 8;
      for (const m of bundle.materials)
        materialRows.push(...m.base_color, m.metallic, m.roughness, m.tint, 0);
      const tiers: TierMesh[] = [];
      const gpu: GpuAppearance = {
        name,
        bundle,
        tiers,
        joints: 1,
        jointBase: 0,
        clips: null,
        clipBase: 0,
        rig: null,
        locals: [],
      };
      const tierMesh = (
        parts: { key: string; meshes: { mesh: MeshData; pack: ArrayBuffer }[] }[],
      ) => {
        const vertexParts: ArrayBuffer[] = [];
        const indexParts: Uint32Array[] = [];
        const ranges = new Map<string, { first: number; count: number }>();
        let base = 0;
        let first = 0;
        for (const part of parts) {
          const start = first;
          for (const { mesh, pack } of part.meshes) {
            vertexParts.push(pack);
            const idx = new Uint32Array(mesh.indices.length);
            for (let i = 0; i < idx.length; i++) idx[i] = mesh.indices[i] + base;
            indexParts.push(idx);
            base += mesh.positions.length / 3;
            first += idx.length;
          }
          ranges.set(part.key, { first: start, count: first - start });
        }
        const indices = new Uint32Array(first);
        let at = 0;
        for (const part of indexParts) {
          indices.set(part, at);
          at += part.length;
        }
        return {
          vertices: own(upload(`${name}-vertices`, concatBytes(vertexParts), VERTEX_USAGE)),
          indices: own(upload(`${name}-indices`, indices, INDEX_USAGE)),
          triangles: first / 3,
          ranges,
        };
      };
      const noTrack = () => 0;
      for (let t = 0; t < tierCount(bundle); t++) {
        if (bundle.kind === "skinned") {
          tiers.push(
            tierMesh([
              {
                key: "",
                meshes: [
                  {
                    mesh: bundle.tiers[t],
                    pack: packVertices(bundle.tiers[t], materialBase, null, noTrack),
                  },
                ],
              },
            ]),
          );
        } else if (bundle.kind === "articulated") {
          tiers.push(
            tierMesh([
              {
                key: "",
                meshes: bundle.nodes.map((node, i) => {
                  const side = node.name === "track_L" ? 1 : node.name === "track_R" ? 2 : 0;
                  return {
                    mesh: node.tiers[t],
                    pack: packVertices(node.tiers[t], materialBase, i, () => side),
                  };
                }),
              },
            ]),
          );
        } else {
          tiers.push(
            tierMesh(
              bundle.states.map((state) => ({
                key: state.name,
                meshes: [
                  {
                    mesh: state.tiers[t],
                    pack: packVertices(state.tiers[t], materialBase, 0, noTrack),
                  },
                ],
              })),
            ),
          );
        }
      }
      if (bundle.kind === "skinned") {
        const skeleton = installed!.skeletons.get(bundle.skeleton)!;
        gpu.joints = bundle.joints.length;
        gpu.jointBase = parents.length;
        for (const joint of bundle.joints) {
          parents.push(joint.parent);
          inverseBinds.push(...joint.inverse_bind);
        }
        gpu.clips = buildClipTable(skeleton, bundle.joints);
        gpu.clipBase = sampleFloats;
        sampleParts.push(gpu.clips.data);
        sampleFloats += gpu.clips.data.length;
        nextJoints = Math.max(nextJoints, gpu.joints);
      } else if (bundle.kind === "articulated") {
        gpu.joints = bundle.nodes.length;
        gpu.rig = articulationRig(bundle.nodes);
        gpu.locals = restLocals(bundle.nodes);
      }
      built.set(name, gpu);
    }
    const nextMaterials = own(
      upload(
        "model-materials",
        Float32Array.from(materialRows.length ? materialRows : [1, 1, 1, 1, 0, 1, 0, 0]),
        STORAGE_USAGE,
      ),
    );
    let nextTables: typeof jointTables = null;
    let nextKernel: GPUComputePipeline | null = null;
    if (nextJoints > 0) {
      const samples = new Float32Array(sampleFloats);
      let at = 0;
      for (const part of sampleParts) {
        samples.set(part, at);
        at += part.length;
      }
      nextTables = {
        parents: own(upload("pose-parents", Int32Array.from(parents), STORAGE_USAGE)),
        inverseBinds: own(
          upload("pose-inverse-binds", Float32Array.from(inverseBinds), STORAGE_USAGE),
        ),
        samples: own(upload("pose-samples", samples, STORAGE_USAGE)),
      };
      const module = device.createShaderModule({
        label: "pose-kernel",
        code: poseKernelWgsl(nextJoints),
      });
      try {
        nextKernel = await device.createComputePipelineAsync({
          label: "pose-kernel",
          layout: "auto",
          compute: { module, entryPoint: "main" },
        });
      } catch (error) {
        next.release();
        throw error;
      }
    }
    if (registry.isReleased) {
      next.release();
      return;
    }
    const old = scope;
    scope = next;
    appearances = built;
    skeletons = new Map(installed?.skeletons ?? []);
    materials = nextMaterials;
    jointTables = nextTables;
    kernel = nextKernel;
    stats.appearances = appearances.size;
    stats.installed = [...appearances.keys()];
    rebind();
    place(current);
    old?.release();
  }

  const _frames_a: ClipFrames = { f0: 0, f1: 0, w: 0 };
  const _frames_b: ClipFrames = { f0: 0, f1: 0, w: 0 };
  const _world = mat4.create();

  /** Pack records, palettes and pose controls for `list`. */
  function place(list: readonly ModelInstance[]) {
    current = list;
    const order: { inst: ModelInstance; gpu: GpuAppearance; tier: number; key: string }[] = [];
    for (const inst of list) {
      const gpu = appearances.get(inst.appearance);
      if (!gpu || !gpu.tiers.length) continue;
      const tier = Math.min(gpu.tiers.length - 1, Math.max(0, inst.tier ?? 0));
      const key = inst.pose.kind === "static" ? inst.pose.state : "";
      if (!gpu.tiers[tier].ranges.has(key)) continue;
      order.push({ inst, gpu, tier, key });
    }
    order.sort((a, b) =>
      a.gpu.name === b.gpu.name
        ? a.tier - b.tier || (a.key < b.key ? -1 : a.key > b.key ? 1 : 0)
        : a.gpu.name < b.gpu.name
          ? -1
          : 1,
    );
    let matrices = 1;
    for (const o of order) if (o.gpu.bundle.kind !== "static") matrices += o.gpu.joints;
    growPalette(matrices);
    growRecords(order.length);
    let skinned = 0;
    for (const o of order) if (o.gpu.bundle.kind === "skinned") skinned++;
    growControls(skinned);

    mat4.identity(_world);
    paletteStaging.set(_world, IDENTITY_SLOT * 16);
    let cursor = 1;
    let control = 0;
    runs = [];
    let triangles = 0;
    order.forEach((o, i) => {
      const inst = o.inst;
      let base = IDENTITY_SLOT;
      let scrollL = 0;
      let scrollR = 0;
      const bundle = o.gpu.bundle;
      if (bundle.kind === "skinned" && inst.pose.kind === "skinned" && o.gpu.clips) {
        base = cursor;
        cursor += o.gpu.joints;
        const table = o.gpu.clips;
        const main = table.clips.get(inst.pose.clip) ?? table.clips.values().next().value!;
        const fade = inst.pose.blend ? table.clips.get(inst.pose.blend.clip) : undefined;
        clipFrames(_frames_a, main, inst.pose.phase);
        const a = { offset: o.gpu.clipBase + main.offset, ..._frames_a };
        const b = fade
          ? {
              offset: o.gpu.clipBase + fade.offset,
              ...clipFrames(_frames_b, fade, inst.pose.blend!.phase),
            }
          : a;
        writeControl(
          controlWords,
          controlFloats,
          control++,
          a,
          b,
          fade ? inst.pose.blend!.weight : 0,
          o.gpu.joints,
          o.gpu.jointBase,
          base,
        );
      } else if (bundle.kind === "skinned") {
        // Unposed or mis-posed body: its bind pose, which the palette of
        // identity-times-inverse-bind would not give, so pose its first clip.
        base = cursor;
        cursor += o.gpu.joints;
        const first = o.gpu.clips!.clips.values().next().value!;
        const a = { offset: o.gpu.clipBase + first.offset, f0: 0, f1: 0, w: 0 };
        writeControl(
          controlWords,
          controlFloats,
          control++,
          a,
          a,
          0,
          o.gpu.joints,
          o.gpu.jointBase,
          base,
        );
      } else if (bundle.kind === "articulated") {
        base = cursor;
        cursor += o.gpu.joints;
        const input = inst.pose.kind === "articulated" ? inst.pose.articulation : null;
        const nodes = (bundle as ArticulatedBundle).nodes;
        const locals = input
          ? articulate(o.gpu.locals, nodes, o.gpu.rig!, input)
          : nodes.map((n) => n.bind);
        const worlds = worldTransforms(
          nodes.map((n) => n.parent),
          locals,
        );
        worlds.forEach((w: Mat4, j) => paletteStaging.set(w, (base + j) * 16));
        if (input) {
          const scroll = trackScroll(o.gpu.rig!, input);
          scrollL = scroll.left;
          scrollR = scroll.right;
        }
      }
      const r = i * RECORD_FLOATS;
      recordStaging[r] = inst.x;
      recordStaging[r + 1] = inst.y;
      recordStaging[r + 2] = inst.z;
      recordStaging[r + 3] = inst.yaw;
      recordStaging[r + 4] = base;
      recordStaging[r + 5] = scrollL - Math.floor(scrollL);
      recordStaging[r + 6] = scrollR - Math.floor(scrollR);
      recordStaging[r + 7] = inst.highlight ? 1 : 0;
      recordStaging.set(inst.tint ?? NO_TINT, r + 8);
      recordStaging[r + 11] = 0;
      const mesh = o.gpu.tiers[o.tier];
      const range = mesh.ranges.get(o.key)!;
      const last = runs[runs.length - 1];
      if (
        last &&
        last.mesh === mesh &&
        last.first === range.first &&
        last.firstInstance + last.instances === i
      )
        last.instances++;
      else
        runs.push({ mesh, first: range.first, count: range.count, firstInstance: i, instances: 1 });
      triangles += range.count / 3;
    });
    paletteUsed = cursor;
    skinnedCount = control;
    device.queue.writeBuffer(palette.current!, 0, paletteStaging.buffer, 0, cursor * 64);
    if (order.length)
      device.queue.writeBuffer(
        records.current!,
        0,
        recordStaging.buffer,
        0,
        order.length * RECORD_FLOATS * 4,
      );
    if (control) {
      device.queue.writeBuffer(
        controls.current!,
        0,
        controlWords.buffer,
        0,
        control * CONTROL_WORDS * 4,
      );
      device.queue.writeBuffer(dispatch, 0, Uint32Array.of(control, 0, 0, 0));
    }
    stats.instances = order.length;
    stats.triangles = triangles;
    stats.draws = runs.length;
    stats.paletteMatrices = cursor;
    stats.skinned = control;
  }

  /** Anything a bound pipeline can draw models through. */
  interface Drawable {
    with(layout: typeof modelVertexLayout, buffer: GPUBuffer): Drawable;
    with(layout: typeof modelRecordLayout, buffer: GPUBuffer): Drawable;
    with(layout: typeof modelLayout, group: GPUBindGroup): Drawable;
    withIndexBuffer(
      buffer: GPUBuffer,
      format: GPUIndexFormat,
    ): {
      drawIndexed(
        count: number,
        instances: number,
        first: number,
        baseVertex: number,
        firstInstance: number,
      ): void;
    };
  }

  return {
    /** Install a catalog generation's appearances (null clears them). */
    setAppearances: install,
    /** The models to draw, posed; cheap enough to call every frame. */
    setModels: place,
    get models(): readonly ModelInstance[] {
      return current;
    },
    /** An installed appearance's bundle, or null. */
    bundle(name: string): Exclude<Bundle, SkeletonClips> | null {
      return appearances.get(name)?.bundle ?? null;
    },
    /** The pose an impostor of `name` shows, and its bounds in that pose. */
    farPose(name: string): { pose: ModelPose; bounds: Bounds } | null {
      const gpu = appearances.get(name);
      if (!gpu) return null;
      const bundle = gpu.bundle;
      const skeleton = bundle.kind === "skinned" ? (skeletons.get(bundle.skeleton) ?? null) : null;
      const pose: ModelPose =
        bundle.kind === "skinned"
          ? {
              kind: "skinned",
              clip: bundle.far_pose.clip,
              phase: bundle.far_pose.phase,
              blend: null,
            }
          : bundle.kind === "articulated"
            ? { kind: "articulated", articulation: { ...REST_ARTICULATION } }
            : {
                kind: "static",
                state: bundle.states.some((s) => s.name === "intact")
                  ? "intact"
                  : bundle.states[0].name,
              };
      return { pose, bounds: farPoseBounds(bundle, skeleton) };
    },
    /** Run the pose kernel for this frame's skinned models. */
    encodePose(raw: GPUCommandEncoder) {
      if (!skinnedCount || !kernel || !computeGroup) return;
      const pass = raw.beginComputePass({ label: "pose-kernel" });
      pass.setPipeline(kernel);
      pass.setBindGroup(0, computeGroup);
      pass.dispatchWorkgroups(Math.ceil(skinnedCount / 64));
      pass.end();
    },
    draw(bound: Drawable) {
      if (!runs.length || !renderGroup || !records.current) return;
      const b = bound.with(modelLayout, renderGroup);
      for (const run of runs)
        b.with(modelVertexLayout, run.mesh.vertices)
          .with(modelRecordLayout, records.current)
          .withIndexBuffer(run.mesh.indices, "uint32")
          .drawIndexed(run.count, run.instances, run.first, 0, run.firstInstance);
    },
    /** Debug readback (bounded, named): the palette the kernel and the CPU wrote. */
    async readPalette(): Promise<Float32Array> {
      const bytes = paletteUsed * 64;
      const read = device.createBuffer({
        label: "palette-readback",
        size: bytes,
        usage: 0x01 | 0x08,
      });
      const encoder = device.createCommandEncoder();
      encoder.copyBufferToBuffer(palette.current!, 0, read, 0, bytes);
      device.queue.submit([encoder.finish()]);
      await read.mapAsync(1);
      const out = new Float32Array(read.getMappedRange().slice(0));
      read.destroy();
      return out;
    },
    /** Each drawn model's palette start, in draw order, for the debug readback. */
    paletteBases(): number[] {
      const out: number[] = [];
      for (let i = 0; i < stats.instances; i++) out.push(recordStaging[i * RECORD_FLOATS + 4]);
      return out;
    },
    stats: (): ModelStats => ({ ...stats, installed: [...stats.installed] }),
    dispose() {
      scope?.release();
    },
  };
}
export type ModelLayer = Awaited<ReturnType<typeof createModelLayer>>;
