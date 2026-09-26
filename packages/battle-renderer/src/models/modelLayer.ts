// The models layer: real appearance bundles in the battle frame. One vertex
// path draws all three bundle kinds, because each is a palette-skinned mesh:
//
// - skinned bodies take their palette from the pose kernel (GPU), per soldier;
// - articulated vehicles are rigidly skinned, one palette matrix per node,
//   posed on the CPU from the pose inputs (`articulate`);
// - static buildings, and every corpse, use the palette's identity slot. A
//   corpse is its body posed once at the bundle's `corpse_pose` at install
//   (`posedMesh`), so the thousands of the fallen are never skinned.
//
// Rewritten from reading ~/dev/game battle-renderer/src/world/crowd.ts
// (reuse manifest, technique): per-appearance, per-tier vertex and index
// buffers, instances packed per draw, a palette storage buffer read by the
// vertex stage, and a caster variant for the sun's cascades. The layer is lit,
// shadowed and fogged exactly like the rest of the world (environment `shade`,
// `sampleSunShadow`, `FogTerm`) and draws in the frame's own passes.
//
// Each frame `prepare` chooses what to draw from the camera (`modelDetail.ts`):
// models off screen are skipped, the rest take a mesh tier by projected height
// or, below `impostor_px`, their impostor card (`impostorCards.ts`); only
// bodies drawn as meshes run the pose kernel. Draws carry a fog class: units
// (posed soldiers and vehicles) are drawn by identification and never fogged;
// the world's models (buildings, corpses) take fog like any face.

import { tgpu, d, std } from "typegpu";
import { mat4, type Mat4 } from "math";
import { frustum } from "math/shapes";
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
import {
  farPoseBounds,
  poseWorlds,
  posedMesh,
  positionsBounds,
} from "@packages/scene-assets/src/pose";
import type { Trs } from "@packages/scene-assets/src/trs";
import { typegpuCameraLayout } from "../world/camera";
import type { EnvironmentFrame } from "../frame/environmentFrame";
import { fogCoverage } from "../frame/fogTerm";
import { WORLD_OUT } from "../frame/targets";
import type { DetailView } from "../frame/detailView";
import { FOG_CLASSES, FOG_INDEX, UNITS, modelFog, modelSeen, type ModelFog } from "./modelFog";
import type { GpuRegistry, GpuSlot } from "../frame/registry";
import { buildClipTable, clipFrames, type ClipFrames, type ClipTable } from "./clipTable";
import { CONTROL_WORDS, poseKernelWgsl, writeControl } from "./poseKernel";
import type { CorpseInstance, ModelInstance, ModelPose } from "./modelInstances";
import {
  CULLED,
  IMPOSTOR,
  chunkCorpses,
  chunkIsFar,
  modelDetail,
  type CorpseChunk,
  type ModelDetailPresentation,
} from "./modelDetail";
import { uploadCardAtlases, type CardGroup } from "./impostorCards";
import type { ImpostorAtlas } from "./impostor";

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
 *  the side's tint (rgb) on tint-masked materials, and the impostor atlas
 *  layer a card draws from; the per-axis scale a fitted prop takes (xyz). */
export const ModelRecord = d.unstruct({
  placement: d.float32x4,
  data: d.float32x4,
  tint: d.float32x4,
  scale: d.float32x4,
});
const RECORD_FLOATS = 16;
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

/** The pose every corpse in the static population lies in. */
const CORPSE_POSE: ModelPose = { kind: "corpse" };
/** Palette slot 0 is the identity every static model and corpse uses. */
const IDENTITY_SLOT = 0;
/** Selection glow, as the proxies' (worldPass `HIGHLIGHT`). */
const HIGHLIGHT = [0.95, 0.8, 0.2] as const;
/** A model drawn without a side keeps its authored colours. */
const NO_TINT = [1, 1, 1] as const;
const UNIT_SCALE = [1, 1, 1] as const;
/** Shadow casters draw this many tiers coarser than the view: a cascade texel
 *  is larger than the detail between tiers (as the forest's casters). */
const CASTER_COARSER = 1;
/** Track links: the dark half of each link's pitch. */
const TRACK_LINK_SHADE = 0.55;

const fogClassOf = (pose: ModelPose) => FOG_INDEX[modelFog(pose)];

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
    scale: d.vec4f,
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
    anchor: d.vec3f,
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
  // A fitted prop's scale, in its own frame; normals take the inverse.
  const local = std.mul(skinned.xyz, v.scale.xyz);
  const localNormal = std.div(skinnedNormal.xyz, v.scale.xyz);
  const world = d.vec3f(
    local.x * c - local.y * s + v.placement.x,
    local.x * s + local.y * c + v.placement.y,
    local.z + v.placement.z,
  );
  const normal = d.vec3f(
    localNormal.x * c - localNormal.y * s,
    localNormal.x * s + localNormal.y * c,
    localNormal.z,
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
    anchor: v.placement.xyz,
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
  anchor: d.vec3f,
};

/** Linear albedo before the side's tint: the vertex colour (glTF COLOR_0 is
 *  linear) times the material's base colour, darkened on a track's link gaps. */
const modelBaseAlbedo = tgpu.fn(
  [d.vec4f, d.u32, d.f32],
  d.vec3f,
)((color, material, track) => {
  "use gpu";
  const base = modelLayout.$.materials[material * 2];
  let albedo = std.mul(color.xyz, base.xyz);
  // A track's scroll arrives as 1 + fract(u − offset): links split light and dark.
  if (track >= 1 && track - 1 < 0.5) {
    albedo = std.mul(albedo, TRACK_LINK_SHADE);
  }
  return albedo;
});

/** How much of the side's tint a material takes (its tint mask). */
const modelTintMask = tgpu.fn(
  [d.u32],
  d.f32,
)((material) => {
  "use gpu";
  return modelLayout.$.materials[material * 2 + 1].z;
});

export function createModelFragments(environment: EnvironmentFrame) {
  const lit = tgpu.fragmentFn({ in: modelVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    const albedo = std.mul(
      modelBaseAlbedo(v.color, v.material, v.track),
      std.mix(d.vec3f(1), v.tint, modelTintMask(v.material)),
    );
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
    const seen = modelSeen(v.world, n, v.anchor, v.clip.xy);
    return { color: d.vec4f(std.add(shaded.xyz, glow), 1), fog: fogCoverage(seen, 1) };
  });
  /** The impostor bake's targets, each with coverage in alpha: display-encoded
   *  albedo before the side's tint (the battle tints cards itself), the
   *  model-space normal mapped to 0..1, and the tint mask. Unlit: the battle
   *  relights impostors itself. */
  const impostor = tgpu.fragmentFn({
    in: modelVaryings,
    out: { albedo: d.vec4f, normal: d.vec4f, mask: d.vec4f },
  })((v) => {
    "use gpu";
    const albedo = modelBaseAlbedo(v.color, v.material, v.track);
    const n = std.normalize(v.normal);
    return {
      albedo: d.vec4f(std.pow(std.max(albedo, d.vec3f(0)), d.vec3f(1 / 2.2)), 1),
      normal: d.vec4f(std.add(std.mul(n, 0.5), d.vec3f(0.5)), 1),
      mask: d.vec4f(modelTintMask(v.material), 0, 0, 1),
    };
  });
  return { lit, impostor };
}

/** One tier of one appearance on the GPU. */
interface TierMesh {
  vertices: GPUBuffer;
  indices: GPUBuffer;
}

/** A drawable range: one tier of a body, a corpse or a static state. Draw
 *  buckets are indexed by `id`. */
interface Drawable {
  id: number;
  tier: number;
  mesh: TierMesh;
  first: number;
  count: number;
  /** What the sun's cascades draw in its place (`CASTER_COARSER` tiers coarser). */
  caster: Drawable;
  /** A posed body's (a soldier's) mesh, counted apart in the stats. */
  body: boolean;
}

interface GpuAppearance {
  name: string;
  bundle: Exclude<Bundle, SkeletonClips>;
  /** Per tier: the posed body (skinned, articulated); empty for static. */
  body: Drawable[];
  /** Per state, per tier (static). */
  states: Map<string, Drawable[]>;
  /** Per tier: the corpse (skinned only). */
  corpse: Drawable[];
  /** Palette matrices one model takes. */
  joints: number;
  /** Skinned: the body's rows in the joint table and its clip table. */
  jointBase: number;
  clips: ClipTable | null;
  clipBase: number;
  /** Articulated: the rig, and reusable locals, node parents and worlds. */
  rig: ArticulationRig | null;
  locals: Trs[];
  parents: number[];
  worlds: Mat4[];
  /** Standing height and reach from the foot (far pose), metres: detail and culling. */
  size: number;
  radius: number;
  /** The corpse's length and reach. */
  corpseSize: number;
  corpseRadius: number;
  /** The corpse's bounds, for its impostor bake. */
  corpseBounds: Bounds | null;
  /** Impostor atlas layers, or -1 without one. */
  farCard: number;
  corpseCard: number;
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

const boundsSize = (b: Bounds) => ({
  size: Math.max(b.max[2] - b.min[2], b.max[0] - b.min[0], b.max[1] - b.min[1]),
  radius: Math.hypot(
    Math.max(Math.abs(b.min[0]), Math.abs(b.max[0])),
    Math.max(Math.abs(b.min[1]), Math.abs(b.max[1])),
    b.max[2] - b.min[2],
  ),
});

/** A mesh draw: one drawable over a run of records, in one fog class. */
interface DrawRun {
  drawable: Drawable;
  fog: number;
  firstInstance: number;
  instances: number;
}

/** A card draw: a run of records in the per-frame or the corpses' static buffer. */
interface CardRun {
  fixed: boolean;
  fog: number;
  firstInstance: number;
  instances: number;
}

export interface ModelStats {
  /** Appearance names installed on the GPU. */
  installed: string[];
  appearances: number;
  /** Models drawn as meshes (posed units and near corpses). */
  instances: number;
  triangles: number;
  draws: number;
  paletteMatrices: number;
  /** Bodies the pose kernel posed this frame. */
  skinned: number;
  /** Mesh-drawn models per tier. */
  tiers: number[];
  /** Of those, posed bodies (soldiers) per tier. */
  bodyTiers: number[];
  /** Models drawn as impostor cards (corpses in far chunks included). */
  cards: number;
  /** Models skipped as off screen. */
  culled: number;
  /** Of those, posed bodies (soldiers): with `skinned` and `cards`, every
   *  soldier handed to the frame is accounted for. */
  culledBodies: number;
  /** Corpses handed to the frame. */
  corpses: number;
  /** Impostor atlas layers installed, and how long their bake took (ms). */
  atlasLayers: number;
  atlasBakeMs: number;
}

/** An impostor atlas the battle draws cards from, and which pose it shows. */
export interface CardAtlas {
  which: "far" | "corpse";
  atlas: ImpostorAtlas;
}

/** A corpse population, chunked when it changes (`setCorpses`). */
interface Corpses {
  count: number;
  /** Records in chunk order (identity palette, no highlight, card layer set). */
  records: Float32Array<ArrayBuffer>;
  /** Per record: its appearance. */
  appearance: (GpuAppearance | null)[];
  chunks: CorpseChunk[];
  /** Per chunk: every corpse in it has a card, so it can draw whole as cards. */
  carded: boolean[];
}

export async function createModelLayer(
  root: Root,
  registry: GpuRegistry,
  detail: ModelDetailPresentation,
) {
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
  let drawables: Drawable[] = [];
  let skeletons = new Map<string, SkeletonClips>();
  let materials: GPUBuffer | null = null;
  let jointTables: { parents: GPUBuffer; inverseBinds: GPUBuffer; samples: GPUBuffer } | null =
    null;
  let kernel: GPUComputePipeline | null = null;
  let cardScope: GpuRegistry | null = null;
  let cards: CardGroup | null = null;
  let atlasLayers = 0;

  const palette: GpuSlot<GPUBuffer> = registry.slot();
  const controls: GpuSlot<GPUBuffer> = registry.slot();
  const records: GpuSlot<GPUBuffer> = registry.slot();
  const cardRecords: GpuSlot<GPUBuffer> = registry.slot();
  const corpseCards: GpuSlot<GPUBuffer> = registry.slot();
  const dispatch = registry.buffer({ label: "pose-dispatch", size: 16, usage: 0x40 | 0x08 });
  let paletteCapacity = 0;
  let controlCapacity = 0;
  let recordCapacity = 0;
  let cardCapacity = 0;
  let renderGroup: GPUBindGroup | null = null;
  let computeGroup: GPUBindGroup | null = null;

  let paletteStaging = new Float32Array(16);
  let controlWords = new Uint32Array(CONTROL_WORDS);
  let controlFloats = new Float32Array(controlWords.buffer);
  let recordStaging = new Float32Array(RECORD_FLOATS);
  let cardStaging = new Float32Array(RECORD_FLOATS);
  const runs: DrawRun[] = [];
  let runCount = 0;
  const cardRuns: CardRun[] = [];
  let cardRunCount = 0;
  let skinnedCount = 0;
  let paletteUsed = 1;
  let drawnCount = 0;

  let units: readonly ModelInstance[] = [];
  let statics: readonly ModelInstance[] = [];
  let corpseList: readonly CorpseInstance[] = [];
  let corpses: Corpses | null = null;
  /** Something changed since the last pack (models, corpses, appearances, a bake). */
  let dirty = true;
  // Grow-once scratch: each unit's choice (bucket, or −1 culled, −2 card).
  let unitChoice = new Int32Array(64);
  let corpseChoice = new Int32Array(64);
  let bucketCount = new Int32Array(8);
  let bucketCursor = new Int32Array(8);
  const nearChunks: number[] = [];

  const stats: ModelStats = {
    installed: [],
    appearances: 0,
    instances: 0,
    triangles: 0,
    draws: 0,
    paletteMatrices: 0,
    skinned: 0,
    tiers: [0, 0, 0, 0],
    bodyTiers: [0, 0, 0, 0],
    cards: 0,
    culled: 0,
    culledBodies: 0,
    corpses: 0,
    atlasLayers: 0,
    atlasBakeMs: 0,
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
  const growCards = (count: number) => {
    if (count <= cardCapacity) return;
    cardCapacity = Math.max(16, count * 2);
    cardRecords.set(buffer("model-cards", cardCapacity * RECORD_FLOATS * 4, VERTEX_USAGE));
    cardStaging = new Float32Array(cardCapacity * RECORD_FLOATS);
  };
  growPalette(1);
  growControls(1);
  growRecords(1);
  growCards(1);

  async function install(installed: InstalledAppearances | null) {
    // Build the new generation beside the old one and swap at the end, so a
    // frame drawn meanwhile still binds live buffers.
    const next = registry.scope();
    const built = new Map<string, GpuAppearance>();
    const nextDrawables: Drawable[] = [];
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
      const skeleton =
        bundle.kind === "skinned" ? installed!.skeletons.get(bundle.skeleton)! : null;
      const far = boundsSize(farPoseBounds(bundle, skeleton));
      const gpu: GpuAppearance = {
        name,
        bundle,
        body: [],
        states: new Map(),
        corpse: [],
        joints: 1,
        jointBase: 0,
        clips: null,
        clipBase: 0,
        rig: null,
        locals: [],
        parents: [],
        worlds: [],
        size: far.size,
        radius: far.radius,
        corpseSize: far.size,
        corpseRadius: far.radius,
        corpseBounds: null,
        farCard: -1,
        corpseCard: -1,
      };
      /** One tier's buffers from parts; returns a drawable per part key. */
      const tierMesh = (
        tier: number,
        tag: string,
        parts: { key: string; meshes: { mesh: MeshData; pack: ArrayBuffer }[] }[],
      ): Map<string, Drawable> => {
        const vertexParts: ArrayBuffer[] = [];
        const indexParts: Uint32Array[] = [];
        const ranges: { key: string; first: number; count: number }[] = [];
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
          ranges.push({ key: part.key, first: start, count: first - start });
        }
        const indices = new Uint32Array(first);
        let at = 0;
        for (const part of indexParts) {
          indices.set(part, at);
          at += part.length;
        }
        const mesh: TierMesh = {
          vertices: own(upload(`${name}-${tag}-vertices`, concatBytes(vertexParts), VERTEX_USAGE)),
          indices: own(upload(`${name}-${tag}-indices`, indices, INDEX_USAGE)),
        };
        const out = new Map<string, Drawable>();
        for (const r of ranges) {
          const drawable = {
            id: nextDrawables.length,
            tier,
            mesh,
            first: r.first,
            count: r.count,
          } as Drawable;
          drawable.caster = drawable;
          drawable.body = false;
          nextDrawables.push(drawable);
          out.set(r.key, drawable);
        }
        return out;
      };
      const noTrack = () => 0;
      const single = (tier: number, tag: string, mesh: MeshData, rigid: number | null) =>
        tierMesh(tier, tag, [
          { key: "", meshes: [{ mesh, pack: packVertices(mesh, materialBase, rigid, noTrack) }] },
        ]).get("")!;
      let corpseWorlds: Mat4[] | null = null;
      if (bundle.kind === "skinned" && skeleton) {
        corpseWorlds = poseWorlds(bundle, skeleton, bundle.corpse_pose);
        const lying = posedMesh(bundle.tiers[0], bundle.joints, corpseWorlds);
        gpu.corpseBounds = positionsBounds(lying.positions);
        const c = boundsSize(gpu.corpseBounds);
        gpu.corpseSize = c.size;
        gpu.corpseRadius = c.radius;
      }
      for (let t = 0; t < tierCount(bundle); t++) {
        if (bundle.kind === "skinned") {
          gpu.body.push(single(t, `t${t}`, bundle.tiers[t], null));
          if (corpseWorlds)
            gpu.corpse.push(
              single(t, `corpse-t${t}`, posedMesh(bundle.tiers[t], bundle.joints, corpseWorlds), 0),
            );
        } else if (bundle.kind === "articulated") {
          gpu.body.push(
            tierMesh(t, `t${t}`, [
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
            ]).get("")!,
          );
        } else {
          const byState = tierMesh(
            t,
            `t${t}`,
            bundle.states.map((state) => ({
              key: state.name,
              meshes: [
                {
                  mesh: state.tiers[t],
                  pack: packVertices(state.tiers[t], materialBase, 0, noTrack),
                },
              ],
            })),
          );
          for (const [state, drawable] of byState) {
            let list = gpu.states.get(state);
            if (!list) gpu.states.set(state, (list = []));
            list.push(drawable);
          }
        }
      }
      if (bundle.kind === "skinned" && skeleton) {
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
        gpu.parents = bundle.nodes.map((n) => n.parent);
        gpu.worlds = bundle.nodes.map(() => mat4.create());
      }
      if (bundle.kind === "skinned") for (const d of gpu.body) d.body = true;
      // Each tier casts with the next coarser one.
      for (const list of [gpu.body, gpu.corpse, ...gpu.states.values()])
        for (let t = 0; t < list.length; t++)
          list[t].caster = list[Math.min(t + CASTER_COARSER, list.length - 1)];
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
    drawables = nextDrawables;
    skeletons = new Map(installed?.skeletons ?? []);
    materials = nextMaterials;
    jointTables = nextTables;
    kernel = nextKernel;
    stats.appearances = appearances.size;
    stats.installed = [...appearances.keys()];
    if (bucketCount.length < drawables.length * FOG_CLASSES) {
      bucketCount = new Int32Array(drawables.length * FOG_CLASSES);
      bucketCursor = new Int32Array(drawables.length * FOG_CLASSES);
    }
    setCards([]);
    rebind();
    rechunk();
    dirty = true;
    old?.release();
  }

  /** Install impostor atlases: each skinned appearance's far pose and corpse. */
  function setCards(atlases: readonly CardAtlas[], bakeMs = 0) {
    cardScope?.release();
    cardScope = null;
    cards = null;
    for (const gpu of appearances.values()) {
      gpu.farCard = -1;
      gpu.corpseCard = -1;
    }
    atlasLayers = atlases.length;
    if (atlases.length) {
      cardScope = registry.scope();
      cards = uploadCardAtlases(
        root,
        cardScope,
        atlases.map((a) => a.atlas),
      );
      atlases.forEach(({ which, atlas }, layer) => {
        const gpu = appearances.get(atlas.appearance);
        if (!gpu) return;
        if (which === "corpse") gpu.corpseCard = layer;
        else gpu.farCard = layer;
      });
    }
    stats.atlasLayers = atlasLayers;
    stats.atlasBakeMs = bakeMs;
    rechunk();
    dirty = true;
  }

  /** Bucket the corpse list by chunk, with each record's static card. */
  function rechunk() {
    const list = corpseList;
    const count = list.length;
    const positions = new Float32Array(count * 3);
    const sizes = new Float32Array(count);
    const owners: (GpuAppearance | null)[] = list.map((c) => appearances.get(c.appearance) ?? null);
    list.forEach((c, i) => {
      positions.set([c.x, c.y, c.z], i * 3);
      sizes[i] = owners[i]?.corpseSize ?? 0;
    });
    const { order, chunks } = chunkCorpses(positions, sizes, count);
    const recordsOut = new Float32Array(count * RECORD_FLOATS);
    const appearance: (GpuAppearance | null)[] = [];
    for (let k = 0; k < count; k++) {
      const c = list[order[k]];
      const gpu = owners[order[k]];
      appearance.push(gpu);
      const r = k * RECORD_FLOATS;
      recordsOut[r] = c.x;
      recordsOut[r + 1] = c.y;
      recordsOut[r + 2] = c.z;
      recordsOut[r + 3] = c.yaw;
      recordsOut[r + 4] = IDENTITY_SLOT;
      recordsOut.set(c.tint ?? NO_TINT, r + 8);
      recordsOut[r + 11] = gpu?.corpseCard ?? -1;
      recordsOut.set(UNIT_SCALE, r + 12);
    }
    const carded = chunks.map((chunk) => {
      for (let i = chunk.start; i < chunk.end; i++)
        if ((appearance[i]?.corpseCard ?? -1) < 0) return false;
      return true;
    });
    corpses = { count, records: recordsOut, appearance, chunks, carded };
    corpseCards.set(upload("corpse-cards", recordsOut, VERTEX_USAGE));
    if (corpseChoice.length < count) corpseChoice = new Int32Array(count * 2);
    stats.corpses = count;
    dirty = true;
  }

  const _frames_a: ClipFrames = { f0: 0, f1: 0, w: 0 };
  const _frames_b: ClipFrames = { f0: 0, f1: 0, w: 0 };
  const _frames_ref = { offset: 0, f0: 0, f1: 0, w: 0 };
  const _frames_fade = { offset: 0, f0: 0, f1: 0, w: 0 };
  const _world = mat4.create();

  /** The drawable an instance's pose takes at `tier`, or null. */
  function drawableOf(gpu: GpuAppearance, pose: ModelPose, tier: number): Drawable | null {
    if (pose.kind === "corpse") return gpu.corpse[tier] ?? null;
    if (pose.kind === "static") return gpu.states.get(pose.state)?.[tier] ?? null;
    return gpu.body[tier] ?? null;
  }

  /** Choose, sort and upload this frame's draws. With `view` null every
   *  model draws at its own tier (0 unless given) and nothing is culled. */
  function pack(view: DetailView | null) {
    const unitCount = units.length;
    const total = unitCount + statics.length;
    /** The i-th model: the units, then the statics. */
    const modelAt = (i: number) => (i < unitCount ? units[i] : statics[i - unitCount]);
    if (unitChoice.length < total) unitChoice = new Int32Array(total * 2);
    const buckets = drawables.length * FOG_CLASSES;
    bucketCount.fill(0, 0, buckets);
    let culled = 0;
    let culledBodies = 0;
    let unitCards = 0;
    let nearCards = 0;

    // Units and props: a mesh bucket, a card, or nothing.
    for (let i = 0; i < total; i++) {
      const inst = modelAt(i);
      const gpu = appearances.get(inst.appearance);
      unitChoice[i] = CULLED;
      if (!gpu) continue;
      const lying = inst.pose.kind === "corpse";
      const card = lying ? gpu.corpseCard : gpu.farCard;
      const tiers = tierCount(gpu.bundle);
      let tier: number;
      // A fitted prop's reach grows with its largest scale.
      const grow = inst.scale ? Math.max(inst.scale[0], inst.scale[1], inst.scale[2]) : 1;
      if (inst.tier !== undefined || !view) tier = Math.min(tiers - 1, Math.max(0, inst.tier ?? 0));
      else
        tier = modelDetail(
          detail,
          view,
          inst.x,
          inst.y,
          inst.z,
          (lying ? gpu.corpseSize : gpu.size) * grow,
          (lying ? gpu.corpseRadius : gpu.radius) * grow,
          card >= 0,
        );
      if (tier === CULLED) {
        culled++;
        if (inst.pose.kind === "skinned") culledBodies++;
        continue;
      }
      if (tier === IMPOSTOR) {
        unitChoice[i] = -2;
        unitCards++;
        continue;
      }
      const drawable = drawableOf(gpu, inst.pose, tier);
      if (!drawable) continue;
      const bucket = drawable.id * FOG_CLASSES + fogClassOf(inst.pose);
      unitChoice[i] = bucket;
      bucketCount[bucket]++;
    }

    // Corpses: far chunks as ranges of static cards, near ones per corpse.
    cardRunCount = 0;
    nearChunks.length = 0;
    const fixedRuns = cardRuns;
    let fixedCards = 0;
    if (corpses && view) {
      for (let k = 0; k < corpses.chunks.length; k++) {
        const chunk = corpses.chunks[k];
        if (!frustum.sidesIntersectsBox3(view.sides, chunk.box)) {
          culled += chunk.end - chunk.start;
          continue;
        }
        if (corpses.carded[k] && chunkIsFar(detail, view, chunk)) {
          const last = cardRunCount > 0 ? fixedRuns[cardRunCount - 1] : null;
          if (last && last.fixed && last.firstInstance + last.instances === chunk.start)
            last.instances += chunk.end - chunk.start;
          else pushCardRun(true, fogClassOf(CORPSE_POSE), chunk.start, chunk.end - chunk.start);
          fixedCards += chunk.end - chunk.start;
          continue;
        }
        nearChunks.push(k);
        for (let i = chunk.start; i < chunk.end; i++) {
          const gpu = corpses.appearance[i];
          corpseChoice[i] = CULLED;
          if (!gpu || !gpu.corpse.length) continue;
          const r = i * RECORD_FLOATS;
          const tier = modelDetail(
            detail,
            view,
            corpses.records[r],
            corpses.records[r + 1],
            corpses.records[r + 2],
            gpu.corpseSize,
            gpu.corpseRadius,
            gpu.corpseCard >= 0,
          );
          if (tier === CULLED) {
            culled++;
            continue;
          }
          if (tier === IMPOSTOR) {
            corpseChoice[i] = -2;
            nearCards++;
            continue;
          }
          const bucket = gpu.corpse[tier].id * FOG_CLASSES + fogClassOf(CORPSE_POSE);
          corpseChoice[i] = bucket;
          bucketCount[bucket]++;
        }
      }
    } else if (corpses) {
      // No view (a bake): every corpse at tier 0.
      for (let k = 0; k < corpses.chunks.length; k++) nearChunks.push(k);
      for (let i = 0; i < corpses.count; i++) {
        const gpu = corpses.appearance[i];
        corpseChoice[i] = CULLED;
        if (!gpu || !gpu.corpse.length) continue;
        const bucket = gpu.corpse[0].id * FOG_CLASSES + fogClassOf(CORPSE_POSE);
        corpseChoice[i] = bucket;
        bucketCount[bucket]++;
      }
    }

    // Buckets in drawable order: each run's records are contiguous.
    let drawn = 0;
    for (let b = 0; b < buckets; b++) {
      bucketCursor[b] = drawn;
      drawn += bucketCount[b];
    }
    growRecords(drawn);
    growCards(unitCards + nearCards);
    let matrices = 1;
    let skinned = 0;
    for (let i = 0; i < total; i++) {
      if (unitChoice[i] < 0) continue;
      const gpu = appearances.get(modelAt(i).appearance)!;
      const kind = modelAt(i).pose.kind;
      if (gpu.bundle.kind === "skinned" && kind === "skinned") {
        matrices += gpu.joints;
        skinned++;
      } else if (gpu.bundle.kind === "articulated") matrices += gpu.joints;
    }
    growPalette(matrices);
    growControls(skinned);

    mat4.identity(_world);
    paletteStaging.set(_world, IDENTITY_SLOT * 16);
    let cursor = 1;
    let control = 0;
    let card = 0;
    for (let i = 0; i < total; i++) {
      const choice = unitChoice[i];
      if (choice === CULLED) continue;
      const inst = modelAt(i);
      const gpu = appearances.get(inst.appearance)!;
      if (choice === -2) {
        writeRecord(
          cardStaging,
          card++,
          inst,
          IDENTITY_SLOT,
          0,
          0,
          inst.pose.kind === "corpse" ? gpu.corpseCard : gpu.farCard,
        );
        continue;
      }
      let base = IDENTITY_SLOT;
      let scrollL = 0;
      let scrollR = 0;
      const bundle = gpu.bundle;
      if (bundle.kind === "skinned" && inst.pose.kind === "skinned" && gpu.clips) {
        base = cursor;
        cursor += gpu.joints;
        const table = gpu.clips;
        const main = table.clips.get(inst.pose.clip) ?? table.clips.values().next().value!;
        const fade = inst.pose.blend ? table.clips.get(inst.pose.blend.clip) : undefined;
        clipFrames(_frames_a, main, inst.pose.phase);
        _frames_ref.offset = gpu.clipBase + main.offset;
        _frames_ref.f0 = _frames_a.f0;
        _frames_ref.f1 = _frames_a.f1;
        _frames_ref.w = _frames_a.w;
        let b = _frames_ref;
        if (fade) {
          clipFrames(_frames_b, fade, inst.pose.blend!.phase);
          _frames_fade.offset = gpu.clipBase + fade.offset;
          _frames_fade.f0 = _frames_b.f0;
          _frames_fade.f1 = _frames_b.f1;
          _frames_fade.w = _frames_b.w;
          b = _frames_fade;
        }
        writeControl(
          controlWords,
          controlFloats,
          control++,
          _frames_ref,
          b,
          fade ? inst.pose.blend!.weight : 0,
          gpu.joints,
          gpu.jointBase,
          base,
        );
      } else if (bundle.kind === "articulated") {
        base = cursor;
        cursor += gpu.joints;
        const input = inst.pose.kind === "articulated" ? inst.pose.articulation : REST_ARTICULATION;
        const nodes = (bundle as ArticulatedBundle).nodes;
        const locals = articulate(gpu.locals, nodes, gpu.rig!, input);
        // Node worlds in place (parents precede children), into the palette.
        const { parents, worlds } = gpu;
        for (let j = 0; j < parents.length; j++) {
          const w = worlds[j];
          mat4.fromRotationTranslationScale(w, locals[j].r, locals[j].t, locals[j].s);
          if (parents[j] >= 0) mat4.multiply(w, worlds[parents[j]], w);
          paletteStaging.set(w, (base + j) * 16);
        }
        const scroll = trackScroll(gpu.rig!, input);
        scrollL = scroll.left;
        scrollR = scroll.right;
      }
      writeRecord(recordStaging, bucketCursor[choice]++, inst, base, scrollL, scrollR, -1);
    }
    if (corpses)
      for (const k of nearChunks) {
        const chunk = corpses.chunks[k];
        for (let i = chunk.start; i < chunk.end; i++) {
          const choice = corpseChoice[i];
          if (choice === CULLED) continue;
          const at = choice === -2 ? card++ : bucketCursor[choice]++;
          (choice === -2 ? cardStaging : recordStaging).set(
            corpses.records.subarray(i * RECORD_FLOATS, (i + 1) * RECORD_FLOATS),
            at * RECORD_FLOATS,
          );
        }
      }

    // Runs: each non-empty bucket is one draw.
    runCount = 0;
    let triangles = 0;
    stats.tiers.fill(0);
    stats.bodyTiers.fill(0);
    let first = 0;
    for (let b = 0; b < buckets; b++) {
      const n = bucketCount[b];
      if (!n) continue;
      const drawable = drawables[Math.floor(b / FOG_CLASSES)];
      let run = runs[runCount];
      if (!run) runs[runCount] = run = { drawable, fog: 0, firstInstance: 0, instances: 0 };
      run.drawable = drawable;
      run.fog = b % FOG_CLASSES;
      run.firstInstance = first;
      run.instances = n;
      runCount++;
      first += n;
      triangles += (n * drawable.count) / 3;
      stats.tiers[drawable.tier] += n;
      if (drawable.body) stats.bodyTiers[drawable.tier] += n;
    }
    // Only skinned bodies carry atlases, so a unit's card is a standing soldier.
    if (unitCards) pushCardRun(false, UNITS, 0, unitCards);
    if (nearCards) pushCardRun(false, fogClassOf(CORPSE_POSE), unitCards, nearCards);

    paletteUsed = cursor;
    skinnedCount = control;
    drawnCount = drawn;
    device.queue.writeBuffer(palette.current!, 0, paletteStaging.buffer, 0, cursor * 64);
    if (drawn)
      device.queue.writeBuffer(
        records.current!,
        0,
        recordStaging.buffer,
        0,
        drawn * RECORD_FLOATS * 4,
      );
    if (card)
      device.queue.writeBuffer(
        cardRecords.current!,
        0,
        cardStaging.buffer,
        0,
        card * RECORD_FLOATS * 4,
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
    stats.instances = drawn;
    stats.triangles = triangles;
    stats.draws = runCount + cardRunCount;
    stats.paletteMatrices = cursor;
    stats.skinned = control;
    stats.cards = card + fixedCards;
    stats.culled = culled;
    stats.culledBodies = culledBodies;
  }

  function pushCardRun(fixed: boolean, fog: number, firstInstance: number, instances: number) {
    let run = cardRuns[cardRunCount];
    if (!run) cardRuns[cardRunCount] = run = { fixed, fog, firstInstance, instances };
    run.fixed = fixed;
    run.fog = fog;
    run.firstInstance = firstInstance;
    run.instances = instances;
    cardRunCount++;
  }

  function writeRecord(
    into: Float32Array,
    at: number,
    inst: ModelInstance,
    base: number,
    scrollL: number,
    scrollR: number,
    layer: number,
  ) {
    const r = at * RECORD_FLOATS;
    into[r] = inst.x;
    into[r + 1] = inst.y;
    into[r + 2] = inst.z;
    into[r + 3] = inst.yaw;
    into[r + 4] = base;
    into[r + 5] = scrollL - Math.floor(scrollL);
    into[r + 6] = scrollR - Math.floor(scrollR);
    into[r + 7] = inst.highlight ? 1 : 0;
    into.set(inst.tint ?? NO_TINT, r + 8);
    into[r + 11] = layer;
    into.set(inst.scale ?? UNIT_SCALE, r + 12);
    into[r + 15] = 0;
  }

  /** The pose an impostor of `name` shows, and its bounds in that pose. */
  function farPose(name: string): { pose: ModelPose; bounds: Bounds } | null {
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
  }

  let viewKey = "";

  /** Anything a bound pipeline can draw models through. */
  interface Drawable3 {
    with(layout: typeof modelVertexLayout, buffer: GPUBuffer): Drawable3;
    with(layout: typeof modelRecordLayout, buffer: GPUBuffer): Drawable3;
    with(layout: typeof modelLayout, group: GPUBindGroup): Drawable3;
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
  /** Anything a bound card pipeline can draw through. */
  interface CardDrawable {
    with(layout: typeof modelRecordLayout, buffer: GPUBuffer): CardDrawable;
    with(group: CardGroup): CardDrawable;
    draw(vertices: number, instances: number, firstVertex: number, firstInstance: number): void;
  }

  const fogIndex = (fog?: ModelFog) => (fog === undefined ? -1 : FOG_INDEX[fog]);

  return {
    /** Install a catalog generation's appearances (null clears them). */
    setAppearances: install,
    /** The posed models to draw this frame (soldiers, vehicles, a building). */
    setModels(list: readonly ModelInstance[]) {
      units = list;
      dirty = true;
    },
    /** The static props to draw (the map's, and what the side knows stands),
     *  replacing the last list. Call when it changes. */
    setStatics(list: readonly ModelInstance[]) {
      statics = list;
      dirty = true;
    },
    /** The corpses, a static population: call when the list changes. */
    setCorpses(list: readonly CorpseInstance[]) {
      corpseList = list;
      rechunk();
    },
    get models(): readonly ModelInstance[] {
      return units;
    },
    /** Choose this frame's draws for the camera (`view`), when anything changed. */
    prepare(view: DetailView, key: string) {
      if (!dirty && key === viewKey) return;
      viewKey = key;
      dirty = false;
      pack(view);
    },
    /** Pack `list` exactly as given (its tiers, nothing culled, no corpses) for
     *  an offline draw (the impostor bake); the next `prepare` packs again. */
    packExact(list: readonly ModelInstance[]) {
      const kept = units;
      const keptStatics = statics;
      const keptCorpses = corpses;
      units = list;
      statics = [];
      corpses = null;
      pack(null);
      units = kept;
      statics = keptStatics;
      corpses = keptCorpses;
      dirty = true;
    },
    /** An installed appearance's bundle, or null. */
    bundle(name: string): Exclude<Bundle, SkeletonClips> | null {
      return appearances.get(name)?.bundle ?? null;
    },
    /** The pose an impostor of `name` shows, and its bounds in that pose. */
    farPose,
    /** The impostor atlases the battle carries: every skinned appearance's
     *  far pose and corpse, in the order `setCards` takes them. */
    cardPoses(): {
      appearance: string;
      which: "far" | "corpse";
      pose: ModelPose;
      bounds: Bounds;
    }[] {
      const out: {
        appearance: string;
        which: "far" | "corpse";
        pose: ModelPose;
        bounds: Bounds;
      }[] = [];
      for (const gpu of appearances.values()) {
        if (gpu.bundle.kind !== "skinned" || !gpu.corpseBounds) continue;
        const far = farPose(gpu.name)!;
        out.push({ appearance: gpu.name, which: "far", ...far });
        out.push({
          appearance: gpu.name,
          which: "corpse",
          pose: { kind: "corpse" },
          bounds: gpu.corpseBounds,
        });
      }
      return out;
    },
    setCards,
    /** Run the pose kernel for this frame's skinned models. */
    encodePose(raw: GPUCommandEncoder) {
      if (!skinnedCount || !kernel || !computeGroup) return;
      const pass = raw.beginComputePass({ label: "pose-kernel" });
      pass.setPipeline(kernel);
      pass.setBindGroup(0, computeGroup);
      pass.dispatchWorkgroups(Math.ceil(skinnedCount / 64));
      pass.end();
    },
    /** Every mesh run into one of the sun's cascades, each a tier coarser. */
    drawCasters(bound: Drawable3) {
      if (!runCount || !renderGroup || !records.current) return;
      const b = bound.with(modelLayout, renderGroup);
      for (let i = 0; i < runCount; i++) {
        const run = runs[i];
        const caster = run.drawable.caster;
        b.with(modelVertexLayout, caster.mesh.vertices)
          .with(modelRecordLayout, records.current)
          .withIndexBuffer(caster.mesh.indices, "uint32")
          .drawIndexed(caster.count, run.instances, caster.first, 0, run.firstInstance);
      }
    },
    /** Draw the mesh runs, all of them or one fog class's. */
    draw(bound: Drawable3, fog?: ModelFog) {
      if (!runCount || !renderGroup || !records.current) return;
      const only = fogIndex(fog);
      const b = bound.with(modelLayout, renderGroup);
      for (let i = 0; i < runCount; i++) {
        const run = runs[i];
        if (only >= 0 && run.fog !== only) continue;
        b.with(modelVertexLayout, run.drawable.mesh.vertices)
          .with(modelRecordLayout, records.current)
          .withIndexBuffer(run.drawable.mesh.indices, "uint32")
          .drawIndexed(run.drawable.count, run.instances, run.drawable.first, 0, run.firstInstance);
      }
    },
    /** Draw one fog class's impostor cards. */
    drawCards(bound: CardDrawable, fog: ModelFog) {
      if (!cardRunCount || !cards) return;
      const only = fogIndex(fog);
      const b = bound.with(cards);
      for (let i = 0; i < cardRunCount; i++) {
        const run = cardRuns[i];
        if (run.fog !== only) continue;
        const source = run.fixed ? corpseCards.current : cardRecords.current;
        if (!source) continue;
        b.with(modelRecordLayout, source).draw(6, run.instances, 0, run.firstInstance);
      }
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
    /** Lab probe (bounded, named): GPU time of one pose-kernel dispatch,
     *  averaged over `reps` dispatches in one timestamped pass, for this
     *  frame's posed bodies or, with `bodies` more than that, for as many
     *  copies of them (the kernel at a battle's scale). Null without
     *  `timestamp-query` or a body to pose. The next frame packs afresh. */
    async timeKernel(
      reps: number,
      bodies = skinnedCount,
    ): Promise<{ bodies: number; ms: number } | null> {
      if (!device.features.has("timestamp-query") || !skinnedCount || !kernel) return null;
      if (bodies > skinnedCount) {
        const posed = controlWords.slice(0, skinnedCount * CONTROL_WORDS);
        growControls(bodies);
        for (let i = 0; i < bodies; i++)
          controlWords.set(
            posed.subarray(
              (i % skinnedCount) * CONTROL_WORDS,
              ((i % skinnedCount) + 1) * CONTROL_WORDS,
            ),
            i * CONTROL_WORDS,
          );
        device.queue.writeBuffer(
          controls.current!,
          0,
          controlWords.buffer,
          0,
          bodies * CONTROL_WORDS * 4,
        );
      }
      device.queue.writeBuffer(dispatch, 0, Uint32Array.of(bodies, 0, 0, 0));
      dirty = true;
      if (!computeGroup) return null;
      const querySet = device.createQuerySet({ type: "timestamp", count: 2 });
      const resolved = device.createBuffer({
        label: "kernel-timer",
        size: 16,
        usage: 0x200 | 0x04,
      });
      const read = device.createBuffer({
        label: "kernel-timer-read",
        size: 16,
        usage: 0x01 | 0x08,
      });
      try {
        const encoder = device.createCommandEncoder({ label: "kernel-timer" });
        const pass = encoder.beginComputePass({
          label: "pose-kernel-timed",
          timestampWrites: { querySet, beginningOfPassWriteIndex: 0, endOfPassWriteIndex: 1 },
        });
        pass.setPipeline(kernel);
        pass.setBindGroup(0, computeGroup);
        for (let r = 0; r < reps; r++) pass.dispatchWorkgroups(Math.ceil(bodies / 64));
        pass.end();
        encoder.resolveQuerySet(querySet, 0, 2, resolved, 0);
        encoder.copyBufferToBuffer(resolved, 0, read, 0, 16);
        device.queue.submit([encoder.finish()]);
        await read.mapAsync(1);
        const [t0, t1] = new BigUint64Array(read.getMappedRange().slice(0));
        return { bodies, ms: Number(t1 - t0) / 1e6 / reps };
      } finally {
        querySet.destroy();
        resolved.destroy();
        read.destroy();
      }
    },
    /** Each mesh-drawn model's palette start, in draw order, for the debug readback. */
    paletteBases(): number[] {
      const out: number[] = [];
      for (let i = 0; i < drawnCount; i++) out.push(recordStaging[i * RECORD_FLOATS + 4]);
      return out;
    },
    stats: (): ModelStats => ({
      ...stats,
      installed: [...stats.installed],
      tiers: [...stats.tiers],
      bodyTiers: [...stats.bodyTiers],
    }),
    dispose() {
      cardScope?.release();
      scope?.release();
    },
  };
}
export type ModelLayer = Awaited<ReturnType<typeof createModelLayer>>;
