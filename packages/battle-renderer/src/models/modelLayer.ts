// The models layer: real appearance bundles in the battle frame. One vertex
// path draws all three bundle kinds, because each is a palette-skinned mesh:
//
// - skinned bodies and articulated vehicles take the joint palettes the
//   presentation crate builds (`crates/presentation`), one per soldier and
//   one matrix per vehicle node;
// - static models (a prop, a kit's module), and every corpse, use the
//   palette's identity slot; a static piece in motion (a wreck's, cooking
//   off) takes a slot of its own. A corpse is its body posed once at one of the
//   bundle's `corpse_poses` at install (`posedMesh`), so the fallen (up to their
//   presentation cap) are never skinned.
//
// Rewritten from reading ~/dev/game battle-renderer/src/world/crowd.ts:
// per-appearance, per-tier vertex and index
// buffers, instances packed per draw, a palette storage buffer read by the
// vertex stage, and a caster variant for the sun's cascades. The layer is lit,
// shadowed and fogged exactly like the rest of the world (environment `shade`,
// `sampleSunShadow`, `FogTerm`) and draws in the frame's own passes.
//
// The battle's units, corpses, ghosts and cook-off pieces arrive as the
// presentation frame's batches (`setFrame`), already culled and tiered: each
// batch is one drawable at one tier, or its impostor card, and draws as one
// run of the frame's instance records. The map's props and what the side
// knows of them arrive as its static batches (`setStaticFrame`), each held
// in a buffer of its own until the frame writes it again. A lab's models
// are tiered each frame by Rust's rule for the frame's camera
// (`view::model_detail`, through `model_tiers`): models off screen are
// skipped, the rest take a mesh tier by projected height or, below
// `impostor_px`, their impostor card (`impostorCards.ts`). Each draw's fog
// group and the tier it casts with are Rust's too, a lab's as a batch's
// (`view::model_fog`, `view::batch_caster`).
//
// A mesh's surfaces are not all drawn alike: a material may be a cutout,
// glass or a room behind a window. Its indices are ordered by kind at install
// (`surfaceParts.ts`), every draw takes the kind it draws, and the stages of
// the kinds that are not plain opaque are `surfaceFragments.ts`.
//
// A town's buildings are drawn here too, as instances of their kits' modules
// (`buildingLayer.ts`): the same meshes, records, material and pipelines as
// any static model, with their own record buffers and draw ranges.

import { staticCardPose } from "@packages/scene-assets/src/propCards";
import {
  createPropBatches,
  clearPropBatches,
  setPropBatches,
  drawnProps,
  setPropCardLayer,
  type HeldProps,
} from "./propBatches";
import { recordDraw, type SubmittedDraws } from "../frame/submitted";
import type { Drawable3, CardDrawable } from "./modelDraw";
import { tgpu, d, std } from "typegpu";
import type {
  Bounds,
  Bundle,
  Material,
  MeshData,
  SkeletonClips,
  Texture,
  TextureChannel,
} from "@packages/scene-assets/src/schema";
import { TEXTURE_CHANNELS } from "@packages/scene-assets/src/schema";
import { textureBytes } from "@packages/scene-assets/src/texture";
import { TextureLayers, arrayBytes, uploadTextureArray } from "./modelTextures";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  boundsSize,
  farPoseBounds,
  lyingBounds,
  poseWorlds,
  posedMesh,
} from "@packages/scene-assets/src/pose";
import type { Mesh } from "../mesh";
import { typegpuCameraLayout } from "../world/camera";
import type { EnvironmentFrame } from "../frame/environmentFrame";
import { fogCoverage, groundPaint, paintedAlbedo, paintedSeen, paintGlow } from "../frame/fogTerm";
import { WORLD_OUT } from "../frame/targets";
import {
  SURFACE_CLASSES,
  orderSurfaces,
  partRange,
  surfaceFlags,
  surfaceClass,
  type SurfaceClass,
  type SurfacePart,
  type SurfaceParts,
} from "./surfaceParts";
import { FOG_CLASSES, FOG_INDEX, modelSeen, type ModelFog } from "./modelFog";
import type { GpuRegistry, GpuSlot } from "../frame/registry";
import {
  writeJoint,
  writeRecord,
  restingModelPose,
  type ModelInstance,
  type ModelPose,
} from "./modelInstances";
import {
  BATCH_WORDS,
  CAST_PASS,
  Batch,
  CULLED,
  Counts,
  FOG_UNITS,
  GHOST_PASS,
  HIGH_PASS,
  IMPOSTOR,
  INSTANCE_WORDS,
  Instance,
  JOINT_WORDS,
  Joint,
  NONE,
  VIEW_PASS,
  type frameViews,
} from "../records.gen";
import { batch_caster, model_fog, model_tiers } from "@wasm/presentation/presentation_wasm.js";
import { uploadCardAtlases, type CardGroup } from "./impostorCards";
import type { CardAtlas } from "./impostorCards";
import { createBuildingLayer, type BuildingStats, type BuildingWalkStats } from "./buildingLayer";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** 48 bytes: position, normal, uv, colour, four joints and weights, the
 *  tangent (xyz, w the bitangent's sign; zero without one), and (global
 *  material index, track side: 0 none, 1 left, 2 right). */
export const ModelVertex = d.unstruct({
  position: d.float32x3,
  normal: d.snorm16x4,
  uv: d.float32x2,
  color: d.unorm8x4,
  joints: d.uint8x4,
  weights: d.unorm8x4,
  tangent: d.snorm8x4,
  material: d.uint16x2,
});
export const VERTEX_BYTES = 48;
/** Per model: the presentation crate's `Instance` (`records.gen.ts`). */
export const ModelRecord = Instance;
// Each drawable has ordinary and ghost runs, each with the same fog classes.
const DRAW_CLASSES = FOG_CLASSES * 2;
const RECORD_FLOATS = INSTANCE_WORDS;
/** The words `model_tiers` reads per model. */
const TIER_INPUT = 6;
export const modelVertexLayout = tgpu.vertexLayout(d.disarrayOf(ModelVertex));
export const modelRecordLayout = tgpu.vertexLayout(d.disarrayOf(ModelRecord), "instance");
export const modelAttribs = { ...modelVertexLayout.attrib, ...modelRecordLayout.attrib };

export const modelLayout = tgpu.bindGroupLayout({
  palette: {
    storage: (n: number) => d.arrayOf(Joint, n),
    access: "readonly",
    visibility: ["vertex"],
  },
  /** `MATERIAL_ROWS` vec4s per material (`materialRows`). */
  materials: {
    storage: (n: number) => d.arrayOf(d.vec4f, n),
    access: "readonly",
    visibility: ["fragment"],
  },
  /** Every installed albedo texture, one layer each (sRGB). */
  albedo: { texture: d.texture2dArray(), visibility: ["fragment"] },
  /** Every installed normal and ORM texture, one layer each (linear). */
  surface: { texture: d.texture2dArray(), visibility: ["fragment"] },
  tiled: { sampler: "filtering", visibility: ["fragment"] },
});

/** Per material: base colour, with the coverage value's factor in alpha;
 *  (metallic, roughness, tint, vertex colour scale); texture layers (albedo,
 *  normal, orm; −1 without) and whether it wears; the worn surface (linear
 *  rgb, roughness); a cutout's cutoff, and the surface layer whose alpha is
 *  the coverage value's image (the normal texture's; −1 without). */
export const MATERIAL_ROWS = 5;
/** The rows of one material, given each channel's texture layer. */
function materialRow(m: Material, layer: (channel: TextureChannel) => number): number[] {
  const normal = layer("normal");
  const cutoff = m.coverage.kind === "cutout" ? m.coverage.cutoff : 0;
  return [
    m.base_color,
    [m.metallic, m.roughness, m.tint, m.colour_scale ?? 1],
    [layer("albedo"), normal, layer("orm"), m.wear ? 1 : 0],
    m.wear ?? [0, 0, 0, 1],
    [cutoff, normal, 0, 0],
  ].flat();
}
/** An opaque white material: what a generation with none installs. */
const BLANK_MATERIAL: Material = {
  name: "",
  base_color: [1, 1, 1, 1],
  metallic: 0,
  roughness: 1,
  tint: 0,
  coverage: { kind: "opaque" },
};
/** Anisotropic filtering on material textures: surfaces are seen at grazing
 *  battle angles. */
const TEXTURE_ANISOTROPY = 8;
/** Half the width of the wear edge, in threshold units: a crisp chip. */
const WEAR_EDGE = 0.04;
/** An albedo layer at or below this is switched off (the workbench): the
 *  row holds `ALBEDO_MEAN_ONLY − layer`, and the surface takes its mean colour. */
const ALBEDO_MEAN_ONLY = -2;

/** Palette slot 0 is the identity every static model and corpse uses. */
const IDENTITY_SLOT = 0;
const IDENTITY_JOINT = Float32Array.of(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0);
/** One drawable a presentation batch names (`World.appearances`): an
 *  appearance's posed body or first state (`state` empty), one of its
 *  states, or a way it lies dead (`lies`, else `NONE`). */
export interface WorldAppearance {
  name: string;
  state: string;
  lies: number;
  /** The static world's (`Appearance.placed`): its batches are written
   *  only when they change, the buildings layer's, the scenery's or the props'. */
  placed: boolean;
  /** What draws it (`Appearance.draws`): 0 a model or a building's kit
   *  module; a prop (`PROP`, `PROP_GROUND`); the scenery layer's otherwise. */
  draws: number;
  /** A felled tree's stump row's mesh (`Presentation::stump_mesh`). */
  stump?: Mesh;
}

/** A posed body's or vehicle's drawable and fog class. */
const POSED: ModelPose = { kind: "posed", palette: new Float32Array(0) };

/** Track links: the dark half of each link's pitch. */
const TRACK_LINK_SHADE = 0.55;

/** The fog group a lab's model binds, as Rust decides it (`view::model_fog`). */
const fogOf = (pose: ModelPose) =>
  model_fog(
    pose.kind === "corpse",
    pose.kind === "static",
    pose.kind === "static" && !!pose.ground,
  );

/** `p` (w 1 a point, 0 a direction) moved by a palette joint's 3×4 rows. */
const moved = tgpu.fn(
  [Joint, d.vec4f],
  d.vec4f,
)((j, p) => {
  "use gpu";
  return d.vec4f(std.dot(j.x, p), std.dot(j.y, p), std.dot(j.z, p), p.w);
});

export const modelVertex = tgpu.vertexFn({
  in: {
    position: d.vec3f,
    normal: d.vec4f,
    uv: d.vec2f,
    color: d.vec4f,
    joints: d.vec4u,
    weights: d.vec4f,
    tangent: d.vec4f,
    material: d.vec2u,
    instance: d.builtin.instanceIndex,
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
    tangent: d.vec4f,
    color: d.vec4f,
    uv: d.vec2f,
    material: d.interpolate("flat", d.u32),
    track: d.f32,
    tint: d.vec3f,
    // The model's own position, the same at every fragment to the bit: what
    // a room behind a window is chosen by.
    anchor: d.interpolate("flat", d.vec3f),
    xray: d.interpolate("flat", d.vec4f),
    instance: d.interpolate("flat", d.u32),
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
    std.add(std.mul(moved(m0, p), v.weights.x), std.mul(moved(m1, p), v.weights.y)),
    std.add(std.mul(moved(m2, p), v.weights.z), std.mul(moved(m3, p), v.weights.w)),
  );
  const skinnedNormal = std.add(
    std.add(std.mul(moved(m0, n), v.weights.x), std.mul(moved(m1, n), v.weights.y)),
    std.add(std.mul(moved(m2, n), v.weights.z), std.mul(moved(m3, n), v.weights.w)),
  );
  const t = d.vec4f(v.tangent.xyz, 0);
  const skinnedTangent = std.add(
    std.add(std.mul(moved(m0, t), v.weights.x), std.mul(moved(m1, t), v.weights.y)),
    std.add(std.mul(moved(m2, t), v.weights.z), std.mul(moved(m3, t), v.weights.w)),
  );
  const c = std.cos(v.placement.w);
  const s = std.sin(v.placement.w);
  // A fitted prop's scale, in its own frame; normals take the inverse,
  // tangents (surface directions) the scale itself.
  const local = std.mul(skinned.xyz, v.scale.xyz);
  const localNormal = std.div(skinnedNormal.xyz, v.scale.xyz);
  const localTangent = std.mul(skinnedTangent.xyz, v.scale.xyz);
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
  const tangent = d.vec4f(
    localTangent.x * c - localTangent.y * s,
    localTangent.x * s + localTangent.y * c,
    localTangent.z,
    v.tangent.w,
  );
  // A track's links run along u: its texture scrolls with the links.
  let scroll = d.f32(0);
  let track = d.f32(0);
  if (v.material.y === 1) {
    scroll = v.data.y;
    track = 1 + std.fract(v.uv.x - scroll);
  }
  if (v.material.y === 2) {
    scroll = v.data.z;
    track = 1 + std.fract(v.uv.x - scroll);
  }
  // The x-ray colour: rgb as one 24-bit integer (exact in f32), alpha apart.
  const packed = v.data.w;
  const red = std.floor(packed / 65536);
  const green = std.floor(packed / 256) - red * 256;
  const blue = packed - std.floor(packed / 256) * 256;
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal,
    tangent,
    color: v.color,
    uv: d.vec2f(v.uv.x - scroll, v.uv.y),
    material: v.material.x,
    track,
    tint: v.tint.xyz,
    anchor: v.placement.xyz,
    xray: d.vec4f(red / 255, green / 255, blue / 255, v.scale.w),
    instance: v.instance,
  };
});

export const modelVaryings = {
  clip: d.builtin.position,
  world: d.vec3f,
  normal: d.vec3f,
  tangent: d.vec4f,
  color: d.vec4f,
  uv: d.vec2f,
  material: d.interpolate("flat", d.u32),
  track: d.f32,
  tint: d.vec3f,
  anchor: d.interpolate("flat", d.vec3f),
  xray: d.interpolate("flat", d.vec4f),
  instance: d.interpolate("flat", d.u32),
};

/** What a model's surface is at one fragment, before light and the side's tint. */
const ModelSurface = d.struct({
  albedo: d.vec3f,
  normal: d.vec3f,
  roughness: d.f32,
  metallic: d.f32,
  occlusion: d.f32,
  /** How much of the side's tint it takes. */
  tint: d.f32,
});

/**
 * The one material function every model path shades through. Linear albedo:
 * the vertex colour (glTF COLOR_0 is linear) times the material's base colour
 * times its albedo texture; where the vertex colour's alpha (how worn) rises
 * past the albedo texture's alpha (where it breaks first), the worn surface
 * shows instead. The normal map bends `normal` (already facing the eye)
 * through the vertex tangent frame; ORM scales roughness and metalness and
 * gives occlusion, and its alpha the tint mask. A material without a texture
 * in a channel keeps its factors there. Without an albedo texture a track's
 * link gaps are shaded as before.
 */
export const modelSurface = tgpu.fn(
  [d.vec4f, d.u32, d.f32, d.vec2f, d.vec3f, d.vec4f],
  ModelSurface,
)((color, material, track, uv, normal, tangent) => {
  "use gpu";
  const row = material * MATERIAL_ROWS;
  const base = modelLayout.$.materials[row];
  const factors = modelLayout.$.materials[row + 1];
  const layers = modelLayout.$.materials[row + 2];
  const worn = modelLayout.$.materials[row + 3];
  // Every channel samples unconditionally: sampling needs uniform control flow.
  const a = std.textureSample(
    modelLayout.$.albedo,
    modelLayout.$.tiled,
    uv,
    d.i32(std.max(layers.x, 0)),
  );
  const nm = std.textureSample(
    modelLayout.$.surface,
    modelLayout.$.tiled,
    uv,
    d.i32(std.max(layers.y, 0)),
  );
  const orm = std.textureSample(
    modelLayout.$.surface,
    modelLayout.$.tiled,
    uv,
    d.i32(std.max(layers.z, 0)),
  );
  let albedo = d.vec3f(std.mul(std.mul(color.xyz, factors.w), base.xyz));
  let threshold = d.f32(0.5);
  if (layers.x >= 0) {
    albedo = std.mul(albedo, a.xyz);
    threshold = a.w;
  } else if (layers.x <= ALBEDO_MEAN_ONLY) {
    // The workbench's albedo switch: the texture's mean colour, its 1×1 level.
    const mean = std.textureSampleLevel(
      modelLayout.$.albedo,
      modelLayout.$.tiled,
      d.vec2f(0.5),
      d.i32(ALBEDO_MEAN_ONLY - layers.x),
      16,
    );
    albedo = std.mul(albedo, mean.xyz);
  } else if (track >= 1 && track - 1 < 0.5) {
    // A track's scroll arrives as 1 + fract(u − offset): links split light and dark.
    albedo = std.mul(albedo, TRACK_LINK_SHADE);
  }
  let roughness = factors.y;
  let metallic = factors.x;
  let occlusion = d.f32(1);
  let tint = factors.z;
  if (layers.z >= 0) {
    occlusion = orm.x;
    roughness = roughness * orm.y;
    metallic = metallic * orm.z;
    tint = tint * orm.w;
  }
  let n = d.vec3f(normal);
  const tangentLength = std.length(tangent.xyz);
  if (layers.y >= 0 && tangentLength > 0.5) {
    const t = std.normalize(std.sub(tangent.xyz, std.mul(n, std.dot(n, tangent.xyz))));
    const b = std.mul(std.cross(n, t), tangent.w);
    const m = std.sub(std.mul(nm.xyz, 2), d.vec3f(1));
    n = std.normalize(std.add(std.add(std.mul(t, m.x), std.mul(b, m.y)), std.mul(n, m.z)));
  }
  if (layers.w > 0) {
    const w = std.smoothstep(threshold - WEAR_EDGE, threshold + WEAR_EDGE, color.w);
    albedo = std.mix(albedo, worn.xyz, w);
    roughness = std.mix(roughness, worn.w, w);
    metallic = std.mix(metallic, 0, w);
  }
  return ModelSurface({ albedo, normal: n, roughness, metallic, occlusion, tint });
});

export function createModelFragments(environment: EnvironmentFrame) {
  const lit = tgpu.fragmentFn({ in: modelVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    const surface = modelSurface(v.color, v.material, v.track, v.uv, n, v.tangent);
    const albedo = std.mul(surface.albedo, std.mix(d.vec3f(1), v.tint, surface.tint));
    // The ground paint, on a surface movers stand on (a bridge deck: its
    // fog layer is painted; none on any other) and only on its faces that
    // look up, as paint sprayed from above lands.
    const paint = std.mul(groundPaint(v.world), std.smoothstep(0.5, 0.8, n.z));
    // Shadow and fog read the geometric normal; light reads the bent one.
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const shaded = environment.shade(
      paintedAlbedo(albedo, paint),
      d.vec3f(0),
      surface.roughness,
      0,
      surface.metallic,
      surface.occlusion,
      surface.normal,
      v.world,
      sun,
      eye,
    );
    const seen = paintedSeen(modelSeen(v.world, n, v.anchor, v.clip.xy), paint);
    return { color: d.vec4f(std.add(shaded.xyz, paintGlow(paint)), 1), fog: fogCoverage(seen, 1) };
  });
  /** The impostor bake's targets, each with coverage in alpha: display-encoded
   *  albedo before the side's tint (the battle tints cards itself), with its
   *  occlusion, the model-space normal (bent by the normal map) mapped to 0..1,
   *  and the tint mask. Unlit: the battle relights impostors itself. */
  const impostor = tgpu.fragmentFn({
    in: modelVaryings,
    out: { albedo: d.vec4f, normal: d.vec4f, mask: d.vec4f },
  })((v) => {
    "use gpu";
    const surface = modelSurface(
      v.color,
      v.material,
      v.track,
      v.uv,
      std.normalize(v.normal),
      v.tangent,
    );
    const albedo = std.mul(surface.albedo, surface.occlusion);
    return {
      albedo: d.vec4f(std.pow(std.max(albedo, d.vec3f(0)), d.vec3f(1 / 2.2)), 1),
      normal: d.vec4f(std.add(std.mul(surface.normal, 0.5), d.vec3f(0.5)), 1),
      mask: d.vec4f(surface.tint, 0, 0, 1),
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
  /** Its indices, by how their surface is drawn (`surfaceParts.ts`). */
  parts: SurfaceParts;
  /** All of its indices. */
  count: number;
  /** Its pose's tiers, finest first: a batch's `caster` names one. */
  tiers: Drawable[];
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
  /** Each way the body lies dead (skinned only), as `corpse_poses`. */
  corpses: GpuCorpse[];
  /** Standing height and reach from the foot (far pose), metres: detail and culling. */
  size: number;
  radius: number;
  /** Impostor atlas layer, or -1 without one. */
  farCard: number;
  staticCard: ReturnType<typeof staticCardPose>;
}

/** One way a body lies dead: its posed tiers, length and reach, bounds (for
 *  its impostor bake) and impostor layer (-1 without one). */
interface GpuCorpse {
  tiers: Drawable[];
  size: number;
  radius: number;
  bounds: Bounds;
  card: number;
}

/** Fold a MeshData into the 48-byte vertex layout. */
export function packVertices(
  mesh: MeshData,
  materialBase: number,
  rigid: number | null,
  trackSide: (vertex: number) => number,
): ArrayBuffer {
  const count = mesh.positions.length / 3;
  const bytes = new ArrayBuffer(count * VERTEX_BYTES);
  const f32 = new Float32Array(bytes);
  const i16 = new Int16Array(bytes);
  const i8 = new Int8Array(bytes);
  const u8 = new Uint8Array(bytes);
  const u16 = new Uint16Array(bytes);
  const weights = [0, 0, 0, 0];
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
    for (let c = 0; c < 4; c++)
      u8[o + 32 + c] = rigid ?? (mesh.joints ? mesh.joints[v * 4 + c] : 0);
    // Weights narrow to unorm8, still summing to exactly one.
    if (rigid !== null || !mesh.weights) weights.fill(0).fill(255, 0, 1);
    else {
      let heaviest = 0;
      for (let c = 0; c < 4; c++) {
        weights[c] = Math.round((mesh.weights[v * 4 + c] / 65535) * 255);
        if (mesh.weights[v * 4 + c] > mesh.weights[v * 4 + heaviest]) heaviest = c;
      }
      weights[heaviest] += 255 - (weights[0] + weights[1] + weights[2] + weights[3]);
    }
    for (let c = 0; c < 4; c++) u8[o + 36 + c] = weights[c];
    if (mesh.tangents)
      for (let c = 0; c < 4; c++) i8[o + 40 + c] = Math.round(mesh.tangents[v * 4 + c] / 258);
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

/** A mesh draw: one drawable over a run of records, in one fog class. */
interface DrawRun {
  ghost: boolean;
  /** A high-air model (`HIGH_PASS`): drawn on its own plane when the frame
   *  has one (`highApart`), with the world otherwise. */
  high: boolean;
  drawable: Drawable;
  /** What it casts into the sun's cascades in its place, if anything. */
  caster: Drawable | null;
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
  /** Prepared mesh population; actual pass submissions live in SubmittedFrame. */
  instances: number;
  triangles: number;
  /** Prepared runs, before surface/pass filtering. */
  draws: number;
  paletteMatrices: number;
  /** Bodies drawn posed by their palettes this frame. */
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
  /** Material texture layers installed (albedo and surface arrays together),
   *  the bytes the two arrays take on the GPU, and each installed
   *  appearance's own textures' bytes (shared textures count in each). */
  textureLayers: number;
  textureBytes: number;
  appearanceTextureBytes: Record<string, number>;
}

export async function createModelLayer(
  root: Root,
  registry: GpuRegistry,
  /** `presentation.models` as JSON, handed to Rust's tier rule unread. */
  detail: string,
) {
  const device = root.device;
  const buildings = createBuildingLayer(device, registry);
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
  /** Whether any installed mesh has a surface of each class. */
  let installedSurfaces = surfaceFlags();
  /** Lab diagnostics: the classes not drawn (`setSurfaceShown`). */
  const hiddenSurfaces = surfaceFlags();
  let skeletons = new Map<string, SkeletonClips>();
  let materials: GPUBuffer | null = null;
  /** The generation's material rows as authored; `writeMaterials` applies the
   *  channel switches on the way to the GPU. */
  let materialRows = new Float32Array(MATERIAL_ROWS * 4);
  const channels: Record<TextureChannel, boolean> = { albedo: true, normal: true, orm: true };
  let textureViews: { albedo: GPUTextureView; surface: GPUTextureView } | null = null;
  const tiled = device.createSampler({
    label: "model-textures",
    addressModeU: "repeat",
    addressModeV: "repeat",
    magFilter: "linear",
    minFilter: "linear",
    mipmapFilter: "linear",
    maxAnisotropy: TEXTURE_ANISOTROPY,
  });
  let cardScope: GpuRegistry | null = null;
  let cards: CardGroup | null = null;
  let atlasLayers = 0;

  const palette: GpuSlot<GPUBuffer> = registry.slot();
  const records: GpuSlot<GPUBuffer> = registry.slot();
  const cardRecords: GpuSlot<GPUBuffer> = registry.slot();
  const frameRecords: GpuSlot<GPUBuffer> = registry.slot();
  let paletteCapacity = 0;
  let recordCapacity = 0;
  let cardCapacity = 0;
  let frameCapacity = 0;
  let renderGroup: GPUBindGroup | null = null;

  let paletteStaging = new Float32Array(JOINT_WORDS);
  let recordStaging = new Float32Array(RECORD_FLOATS);
  let cardStaging = new Float32Array(RECORD_FLOATS);
  const runs: DrawRun[] = [];
  let runCount = 0;
  const cardRuns: CardRun[] = [];
  let cardRunCount = 0;
  let drawnCount = 0;
  let hasXrayMeshes = false;
  /** The presentation frame's batches as runs over `frameRecords`, and its
   *  cards (layer set) appended to each pack's card records. */
  const frameRuns: DrawRun[] = [];
  let frameRunCount = 0;
  let frameCards = new Float32Array(RECORD_FLOATS);
  let frameCardCount = 0;
  const frameCardRuns: { fog: number; first: number; count: number }[] = [];
  let frameXray = false;
  /** The high-air models draw on their own plane (`setHighApart`). */
  let highApart = false;
  /** The frame's drawables by `World.appearances` row (`setWorld`). */
  let worldRows: readonly WorldAppearance[] = [];
  const frameStats = {
    instances: 0,
    triangles: 0,
    palette: 0,
    skinned: 0,
    cards: 0,
    culled: 0,
    corpses: 0,
    tiers: [0, 0, 0, 0],
    bodyTiers: [0, 0, 0, 0],
  };

  let units: readonly ModelInstance[] = [];
  /** The props' static batches by key, each its records as the frame last
   *  wrote them; those the last frame listed, and their runs. */
  const propBatches = createPropBatches();
  const propRuns: { run: DrawRun; buffer: GPUBuffer; pass: number }[] = [];
  /** Something changed since the last pack (models, appearances, a bake). */
  let dirty = true;
  // Grow-once scratch: each unit's choice (bucket, or −1 culled, −2 card).
  let unitChoice = new Int32Array(64);
  /** Each lab model's place and reach, as `model_tiers` reads them. */
  let tierInput = new Float64Array(64 * TIER_INPUT);
  let bucketCount = new Int32Array(8);
  let bucketCursor = new Int32Array(8);

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
    textureLayers: 0,
    textureBytes: 0,
    appearanceTextureBytes: {},
  };

  /** Upload the material rows, with switched-off channels' layers at −1. */
  const writeMaterials = () => {
    if (!materials) return;
    const rows = materialRows.slice();
    for (let r = 0; r < rows.length; r += MATERIAL_ROWS * 4)
      TEXTURE_CHANNELS.forEach((channel, c) => {
        const layer = rows[r + 8 + c];
        // Albedo off keeps its layer as its mean (the vertex colour is relative
        // to it); the others fall back to the factors.
        if (!channels[channel] && layer >= 0)
          rows[r + 8 + c] = channel === "albedo" ? ALBEDO_MEAN_ONLY - layer : -1;
      });
    device.queue.writeBuffer(materials, 0, rows);
  };

  const rebind = () => {
    if (!materials || !palette.current || !textureViews) return;
    renderGroup = device.createBindGroup({
      label: "models",
      layout: root.unwrap(modelLayout),
      entries: [
        { binding: 0, resource: { buffer: palette.current } },
        { binding: 1, resource: { buffer: materials } },
        { binding: 2, resource: textureViews.albedo },
        { binding: 3, resource: textureViews.surface },
        { binding: 4, resource: tiled },
      ],
    });
  };

  const growPalette = (joints: number) => {
    if (joints <= paletteCapacity) return;
    paletteCapacity = Math.max(64, joints * 2);
    palette.set(buffer("model-palette", paletteCapacity * JOINT_WORDS * 4, PALETTE_USAGE));
    paletteStaging = new Float32Array(paletteCapacity * JOINT_WORDS);
    paletteStaging.set(IDENTITY_JOINT);
    device.queue.writeBuffer(palette.current!, 0, IDENTITY_JOINT);
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
  growRecords(1);
  growCards(1);

  async function install(installed: InstalledAppearances | null) {
    // Build the new generation beside the old one and swap at the end, so a
    // frame drawn meanwhile still binds live buffers.
    const next = registry.scope();
    const built = new Map<string, GpuAppearance>();
    const nextDrawables: Drawable[] = [];
    const own = <T extends GPUBuffer | GPUTexture>(b: T) => next.own(b);
    const rows: number[] = [];
    const layers = new TextureLayers();
    const appearanceTextureBytes: Record<string, number> = {};
    for (const [name, appearance] of installed?.appearances ?? []) {
      const { bundle } = appearance;
      const materialBase = rows.length / (MATERIAL_ROWS * 4);
      for (const m of bundle.materials) {
        const layer = (channel: TextureChannel) => {
          const index = m.textures?.[channel];
          return layers.layer(index === undefined ? undefined : bundle.textures[index]);
        };
        rows.push(...materialRow(m, layer));
      }
      const classOf = (material: number) => surfaceClass(bundle.materials[material]);
      appearanceTextureBytes[name] = bundle.textures.reduce((n, t) => n + textureBytes(t), 0);
      const skeleton =
        bundle.kind === "skinned" ? installed!.skeletons.get(bundle.skeleton)! : null;
      const far = boundsSize(farPoseBounds(bundle, skeleton));
      const gpu: GpuAppearance = {
        name,
        bundle,
        body: [],
        states: new Map(),
        corpses: [],
        size: far.size,
        radius: far.radius,
        farCard: -1,
        staticCard: staticCardPose(appearance),
      };
      /** One tier's buffers from parts; returns a drawable per part key. */
      const tierMesh = (
        tier: number,
        tag: string,
        parts: { key: string; meshes: { mesh: MeshData; pack: ArrayBuffer }[] }[],
      ): Map<string, Drawable> => {
        const vertexParts: ArrayBuffer[] = [];
        const indexParts: Uint32Array[] = [];
        const ranges: { key: string; parts: SurfaceParts; count: number }[] = [];
        let base = 0;
        let first = 0;
        for (const part of parts) {
          const meshes = part.meshes.map(({ mesh }) => ({
            vertices: mesh.positions.length / 3,
            indices: mesh.indices,
            draws: mesh.draws,
          }));
          // Each surface class one range of the part's indices.
          const ordered = orderSurfaces(meshes, classOf, base, first);
          for (const { pack } of part.meshes) vertexParts.push(pack);
          indexParts.push(ordered.indices);
          base += meshes.reduce((n, m) => n + m.vertices, 0);
          first += ordered.indices.length;
          ranges.push({ key: part.key, parts: ordered.parts, count: ordered.indices.length });
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
            parts: r.parts,
            count: r.count,
          } as Drawable;
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
      if (bundle.kind === "skinned" && skeleton)
        for (const [k, pose] of bundle.corpse_poses.entries()) {
          const worlds = poseWorlds(bundle, skeleton, pose);
          const bounds = lyingBounds(bundle, skeleton, pose);
          gpu.corpses.push({
            ...boundsSize(bounds),
            bounds,
            card: -1,
            tiers: bundle.tiers.map((mesh, t) =>
              single(t, `corpse${k}-t${t}`, posedMesh(mesh, bundle.joints, worlds), 0),
            ),
          });
        }
      for (let t = 0; t < tierCount(bundle); t++) {
        if (bundle.kind === "skinned") {
          gpu.body.push(single(t, `t${t}`, bundle.tiers[t], null));
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
      if (bundle.kind === "skinned") for (const d of gpu.body) d.body = true;
      for (const list of [gpu.body, ...gpu.corpses.map((c) => c.tiers), ...gpu.states.values()])
        for (const drawable of list) drawable.tiers = list;
      built.set(name, gpu);
    }
    const nextRows = Float32Array.from(rows.length ? rows : materialRow(BLANK_MATERIAL, () => -1));
    const nextMaterials = own(buffer("model-materials", nextRows.byteLength, STORAGE_USAGE));
    const albedoArray = own(
      uploadTextureArray(device, "model-albedo", "rgba8unorm-srgb", layers.albedo),
    );
    const surfaceArray = own(
      uploadTextureArray(device, "model-surface", "rgba8unorm", layers.surface),
    );
    const nextViews = {
      albedo: albedoArray.createView({ dimension: "2d-array" }),
      surface: surfaceArray.createView({ dimension: "2d-array" }),
    };
    const size = (list: Texture[]) => Math.max(1, ...list.map((t) => t.width));
    if (registry.isReleased) {
      next.release();
      return;
    }
    const old = scope;
    scope = next;
    appearances = built;
    drawables = nextDrawables;
    // The frame's runs name the old generation's drawables: drawn again
    // only once the next presentation frame names the new ones.
    frameRunCount = 0;
    frameCardCount = 0;
    frameCardRuns.length = 0;
    installedSurfaces = surfaceFlags();
    for (const drawable of drawables)
      for (const c of SURFACE_CLASSES) installedSurfaces[c] ||= drawable.parts[c].count > 0;
    skeletons = new Map(installed?.skeletons ?? []);
    materials = nextMaterials;
    materialRows = nextRows;
    writeMaterials();
    textureViews = nextViews;
    stats.appearances = appearances.size;
    stats.installed = [...appearances.keys()];
    stats.textureLayers = layers.albedo.length + layers.surface.length;
    stats.textureBytes =
      arrayBytes(size(layers.albedo), Math.max(1, layers.albedo.length)) +
      arrayBytes(size(layers.surface), Math.max(1, layers.surface.length));
    stats.appearanceTextureBytes = appearanceTextureBytes;
    if (bucketCount.length < drawables.length * DRAW_CLASSES) {
      bucketCount = new Int32Array(drawables.length * DRAW_CLASSES);
      bucketCursor = new Int32Array(drawables.length * DRAW_CLASSES);
    }
    setCards([]);
    rebind();
    // The kits a template art library places are appearances like any other:
    // its modules are their states, and the buildings draw those meshes.
    buildings.setSource((kit, module, tier) => {
      const gpu = appearances.get(kit);
      if (gpu?.bundle.kind !== "static") return null;
      const drawable = gpu.states.get(module)?.[tier];
      return drawable ? { ...drawable.mesh, parts: drawable.parts, count: drawable.count } : null;
    });
    relistProps();
    old?.release();
  }

  /** Swap a complete atlas generation; failure preserves the installed one. */
  function setCards(atlases: readonly CardAtlas[], bakeMs = 0) {
    const next = registry.scope();
    let group: CardGroup | null = null;
    try {
      if (atlases.length)
        group = uploadCardAtlases(
          root,
          next,
          atlases.map((a) => a.atlas),
        );
    } catch (error) {
      next.release();
      throw error;
    }
    cardScope?.release();
    cardScope = next;
    cards = group;
    for (const gpu of appearances.values()) {
      gpu.farCard = -1;
      for (const c of gpu.corpses) c.card = -1;
    }
    atlasLayers = atlases.length;
    if (atlases.length) {
      atlases.forEach(({ pose, atlas }, layer) => {
        const gpu = appearances.get(atlas.appearance);
        if (!gpu) return;
        if (pose.kind !== "corpse") gpu.farCard = layer;
        else if (gpu.corpses[pose.lies]) gpu.corpses[pose.lies].card = layer;
      });
    }
    relistProps();
    stats.atlasLayers = atlasLayers;
    stats.atlasBakeMs = bakeMs;
    dirty = true;
  }

  /** The drawable an instance's pose takes at `tier`, or null. */
  function drawableOf(gpu: GpuAppearance, pose: ModelPose, tier: number): Drawable | null {
    if (pose.kind === "corpse") return gpu.corpses[pose.lies]?.tiers[tier] ?? null;
    if (pose.kind === "static") return gpu.states.get(pose.state)?.[tier] ?? null;
    return gpu.body[tier] ?? null;
  }

  /** The drawable a batch's `caster` tier names, or null for none. */
  const casterOf = (drawable: Drawable, casts: number) =>
    casts === NONE ? null : (drawable.tiers[casts] ?? null);

  /** The props' runs: each listed batch's drawable at its tier, against
   *  the meshes installed now. */
  const propCards: HeldProps[] = [];
  function relistProps() {
    propRuns.length = 0;
    propCards.length = 0;
    for (const h of propBatches.listed) {
      const row = worldRows[h.row];
      const gpu = row && appearances.get(row.name);
      const tiers = gpu?.states.get(row.state);
      const layer =
        h.pass !== CAST_PASS && h.tier === IMPOSTOR && gpu?.staticCard?.pose.state === row.state
          ? gpu.farCard
          : -1;
      setPropCardLayer(h, layer, device);
      if (layer >= 0 && h.buffer && h.count) {
        propCards.push(h);
        continue;
      }
      const drawable = tiers?.[Math.min(h.tier, 3)];
      if (!drawable || !h.buffer || !h.count) continue;
      propRuns.push({
        pass: h.pass,
        run: {
          drawable,
          ghost: false,
          high: false,
          caster: casterOf(drawable, h.caster),
          fog: h.fog,
          firstInstance: 0,
          instances: h.count,
        },
        buffer: h.buffer,
      });
    }
    dirty = true;
  }

  function setProps(frame: ReturnType<typeof frameViews> | null): boolean {
    const changed = setPropBatches(propBatches, frame, worldRows, registry, device);
    if (changed) relistProps();
    return changed;
  }

  /** Each lab model's tier by Rust's rule for the frame's `View` record
   *  (`CULLED`, a mesh tier or `IMPOSTOR`), in `units`' order. A model whose
   *  appearance is not installed is sized nothing; its tier goes unread. */
  function cardLayer(gpu: GpuAppearance, inst: ModelInstance): number {
    if (inst.ghost) return -1;
    const pose = inst.pose;
    if (
      pose.kind === "static" &&
      (pose.motion || gpu.bundle.kind !== "static" || pose.state !== gpu.bundle.states[0]?.name)
    )
      return -1;
    return pose.kind === "corpse" ? (gpu.corpses[pose.lies]?.card ?? -1) : gpu.farCard;
  }

  function labTiers(view: Float32Array): Int32Array {
    if (tierInput.length < units.length * TIER_INPUT)
      tierInput = new Float64Array(units.length * TIER_INPUT * 2);
    for (let i = 0; i < units.length; i++) {
      const inst = units[i];
      const gpu = appearances.get(inst.appearance);
      const o = i * TIER_INPUT;
      tierInput.fill(0, o, o + TIER_INPUT);
      if (!gpu) continue;
      const lying = inst.pose.kind === "corpse" ? (gpu.corpses[inst.pose.lies] ?? null) : null;
      // A fitted prop's reach grows with its largest scale, and a moving
      // piece's by how far it has moved.
      const grow = inst.scale ? Math.max(inst.scale[0], inst.scale[1], inst.scale[2]) : 1;
      const motion = inst.pose.kind === "static" ? inst.pose.motion : undefined;
      const moved = motion ? grow * Math.hypot(motion[12], motion[13], motion[14]) : 0;
      tierInput[o] = inst.x;
      tierInput[o + 1] = inst.y;
      tierInput[o + 2] = inst.z;
      tierInput[o + 3] = (lying ?? gpu).size * grow;
      tierInput[o + 4] = (lying ?? gpu).radius * grow + moved;
      tierInput[o + 5] = cardLayer(gpu, inst) >= 0 ? 1 : 0;
    }
    return model_tiers(view, detail, tierInput.subarray(0, units.length * TIER_INPUT));
  }

  /** Choose, sort and upload a lab's models' draws. With `view` null every
   *  model draws at its own tier (0 unless given) and nothing is culled.
   *  The presentation frame's runs are `setFrame`'s, the props' `setProps`'. */
  // eslint-disable-next-line complexity -- debt: its passes (choose buckets, size palette, write records, build runs, upload) share the layer's staging arrays and cursors; split needs a per-pack state those passes take, not loose closure writes.
  function pack(view: Float32Array | null) {
    hasXrayMeshes = frameXray;
    const total = units.length;
    const modelAt = (i: number) => units[i];
    if (unitChoice.length < total) unitChoice = new Int32Array(total * 2);
    const buckets = drawables.length * DRAW_CLASSES;
    bucketCount.fill(0, 0, buckets);
    let culled = 0;
    let unitCards = 0;
    cardRunCount = 0;
    const chosen = view ? labTiers(view) : null;

    // A mesh bucket, a card, or nothing.
    for (let i = 0; i < total; i++) {
      const inst = modelAt(i);
      const gpu = appearances.get(inst.appearance);
      unitChoice[i] = CULLED;
      if (!gpu) continue;
      const tier =
        inst.tier === IMPOSTOR && cardLayer(gpu, inst) >= 0
          ? IMPOSTOR
          : inst.tier !== undefined || !chosen
            ? Math.min(tierCount(gpu.bundle) - 1, Math.max(0, inst.tier ?? 0))
            : chosen[i];
      if (tier === CULLED) {
        culled++;
        continue;
      }
      if (tier === IMPOSTOR) {
        unitChoice[i] = -2;
        unitCards++;
        continue;
      }
      const drawable = drawableOf(gpu, inst.pose, tier);
      if (!drawable) continue;
      const bucket = drawable.id * DRAW_CLASSES + fogOf(inst.pose) + (inst.ghost ? FOG_CLASSES : 0);
      unitChoice[i] = bucket;
      bucketCount[bucket]++;
    }

    // Buckets in drawable order: each run's records are contiguous.
    let drawn = 0;
    for (let b = 0; b < buckets; b++) {
      bucketCursor[b] = drawn;
      drawn += bucketCount[b];
    }
    growRecords(drawn);
    growCards(unitCards + frameCardCount);
    // A lab's posed models and moving pieces take palette slots after the
    // identity; with none, the frame's palette stands.
    let joints = 1;
    for (let i = 0; i < total; i++) {
      if (unitChoice[i] < 0) continue;
      const pose = modelAt(i).pose;
      if (pose.kind === "posed") joints += pose.palette.length / JOINT_WORDS;
      else if (pose.kind === "static" && pose.motion) joints++;
    }
    if (joints > 1) growPalette(joints);
    let cursor = 1;
    let card = 0;
    for (let i = 0; i < total; i++) {
      const choice = unitChoice[i];
      if (choice === CULLED) continue;
      const inst = modelAt(i);
      const gpu = appearances.get(inst.appearance)!;
      if (choice === -2) {
        pushCardRun(false, fogOf(inst.pose), card, 1);
        writeRecord(
          cardStaging,
          card++,
          inst,
          IDENTITY_SLOT,
          0,
          0,
          inst.pose.kind === "corpse" ? (gpu.corpses[inst.pose.lies]?.card ?? -1) : gpu.farCard,
        );
        continue;
      }
      let base = IDENTITY_SLOT;
      let [scrollL, scrollR] = [0, 0];
      const pose = inst.pose;
      if (pose.kind === "posed") {
        base = cursor;
        paletteStaging.set(pose.palette, cursor * JOINT_WORDS);
        cursor += pose.palette.length / JOINT_WORDS;
        [scrollL, scrollR] = pose.scroll ?? [0, 0];
      } else if (pose.kind === "static" && pose.motion) {
        base = cursor++;
        writeJoint(paletteStaging, base, pose.motion);
      }
      if (choice % DRAW_CLASSES === FOG_UNITS && (inst.xray?.[3] ?? 0) > 0) hasXrayMeshes = true;
      writeRecord(recordStaging, bucketCursor[choice]++, inst, base, scrollL, scrollR, -1);
    }

    // Runs: each non-empty bucket is one draw.
    runCount = 0;
    let triangles = frameStats.triangles;
    stats.tiers = [...frameStats.tiers];
    stats.bodyTiers = [...frameStats.bodyTiers];
    let first = 0;
    for (let b = 0; b < buckets; b++) {
      const n = bucketCount[b];
      if (!n) continue;
      const drawable = drawables[Math.floor(b / DRAW_CLASSES)];
      let run = runs[runCount];
      if (!run)
        runs[runCount] = run = {
          drawable,
          ghost: false,
          high: false,
          caster: null,
          fog: 0,
          firstInstance: 0,
          instances: 0,
        };
      run.drawable = drawable;
      run.fog = b % FOG_CLASSES;
      run.ghost = b % DRAW_CLASSES >= FOG_CLASSES;
      run.caster = casterOf(
        drawable,
        batch_caster(drawable.tier, run.ghost ? GHOST_PASS : VIEW_PASS),
      );
      run.firstInstance = first;
      run.instances = n;
      runCount++;
      first += n;
      triangles += (n * drawable.count) / 3;
      stats.tiers[drawable.tier] += n;
      if (drawable.body) stats.bodyTiers[drawable.tier] += n;
    }
    cardStaging.set(frameCards.subarray(0, frameCardCount * RECORD_FLOATS), card * RECORD_FLOATS);
    for (const r of frameCardRuns) pushCardRun(false, r.fog, card + r.first, r.count);
    card += frameCardCount;

    drawnCount = drawn;
    if (cursor > 1)
      device.queue.writeBuffer(
        palette.current!,
        0,
        paletteStaging.buffer,
        0,
        cursor * JOINT_WORDS * 4,
      );
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
    let props = 0;
    let propDraws = 0;
    for (const { run, pass } of propRuns) {
      if (pass === CAST_PASS) continue;
      propDraws++;
      props += run.instances;
      triangles += (run.instances * run.drawable.count) / 3;
      stats.tiers[run.drawable.tier] += run.instances;
    }
    stats.instances = drawn + props + frameStats.instances;
    stats.triangles = triangles;
    stats.draws = runCount + propDraws + frameRunCount + cardRunCount;
    stats.paletteMatrices = cursor > 1 ? cursor : frameStats.palette;
    stats.skinned = frameStats.skinned;
    const propCardCount = propCards.reduce((n, h) => n + h.count, 0);
    stats.cards = card + propCardCount;
    stats.instances += propCardCount;
    stats.triangles += 2 * propCardCount;
    stats.draws += propCards.length;
    stats.culled = culled + frameStats.culled;
    stats.culledBodies = frameStats.culled;
    stats.corpses = frameStats.corpses;
  }

  /** The presentation frame's units, corpses, ghosts and moving pieces
   *  (`crates/presentation`): its palettes and their instance records are
   *  uploaded, each batch draws as one run, and a card batch's records are
   *  kept for the next pack with their atlas layer set. The static world's
   *  batches are the buildings layer's (`setStaticFrame`). */
  function setFrame(frame: ReturnType<typeof frameViews> | null) {
    frameRunCount = 0;
    frameCardCount = 0;
    frameCardRuns.length = 0;
    frameXray = false;
    Object.assign(frameStats, {
      instances: 0,
      triangles: 0,
      palette: 0,
      skinned: 0,
      cards: 0,
      culled: 0,
      corpses: 0,
      tiers: [0, 0, 0, 0],
      bodyTiers: [0, 0, 0, 0],
    });
    dirty = true;
    if (!frame) return;
    const { batches, instances, palettes } = frame;
    frameStats.culled = frame.counts.u32[Counts.culled] ?? 0;
    frameStats.corpses = frame.counts.u32[Counts.corpses] ?? 0;
    growPalette(palettes.count);
    device.queue.writeBuffer(palette.current!, 0, palettes.f32);
    frameStats.palette = palettes.count;
    const words = batches.u32;
    // The posed records follow the static world's: only theirs are uploaded.
    let base = instances.count;
    let end = base;
    for (let b = 0; b < batches.count; b++) {
      const o = b * BATCH_WORDS;
      const first = words[o + Batch.first];
      if (first !== NONE && !worldRows[words[o + Batch.appearance]]?.placed) {
        base = Math.min(base, first);
        end = Math.max(end, first + words[o + Batch.count]);
      }
    }
    // Draw ranges are the contract the GPU validates. Size from their actual
    // end as well as the mixed frame's record count; a batch may outlive the
    // dynamic tail after static records were split out of the upload.
    const posed = Math.max(instances.count, end) - base;
    if (posed > frameCapacity) {
      frameCapacity = Math.max(64, posed * 2);
      frameRecords.set(
        buffer("model-frame-records", frameCapacity * RECORD_FLOATS * 4, VERTEX_USAGE),
      );
    }
    if (posed)
      device.queue.writeBuffer(
        frameRecords.current!,
        0,
        instances.f32.subarray(base * RECORD_FLOATS) as Float32Array<ArrayBuffer>,
      );
    if (frameCards.length < posed * RECORD_FLOATS)
      frameCards = new Float32Array(posed * RECORD_FLOATS);
    for (let b = 0; b < batches.count; b++) {
      const o = b * BATCH_WORDS;
      const row = worldRows[words[o + Batch.appearance]];
      const gpu = row && !row.placed && appearances.get(row.name);
      const [first, count] = [words[o + Batch.first], words[o + Batch.count]];
      if (!gpu || first === NONE || !count) continue;
      const lying = row.lies === NONE ? null : (gpu.corpses[row.lies] ?? null);
      const pose: ModelPose = lying
        ? { kind: "corpse", lies: row.lies }
        : row.state
          ? { kind: "static", state: row.state }
          : POSED;
      const fog = words[o + Batch.fog];
      const casts = words[o + Batch.caster];
      let tier = words[o + Batch.tier];
      if (tier === IMPOSTOR) {
        const layer = lying ? lying.card : gpu.farCard;
        if (layer >= 0) {
          const at = frameCardCount * RECORD_FLOATS;
          frameCards.set(
            instances.f32.subarray(first * RECORD_FLOATS, (first + count) * RECORD_FLOATS),
            at,
          );
          for (let i = 0; i < count; i++) frameCards[at + i * RECORD_FLOATS + 11] = layer;
          frameCardRuns.push({ fog, first: frameCardCount, count });
          frameCardCount += count;
          frameStats.cards += count;
          continue;
        }
        // No card baked (yet): the coarsest mesh, as the tier rule gives without one.
        tier = 3;
      }
      const drawable = drawableOf(gpu, pose, tier);
      if (!drawable) continue;
      const ghost = words[o + Batch.pass] === GHOST_PASS;
      const high = words[o + Batch.pass] === HIGH_PASS;
      const caster = casterOf(drawable, casts);
      let run = frameRuns[frameRunCount];
      if (!run)
        frameRuns[frameRunCount] = run = {
          drawable,
          ghost,
          high,
          caster,
          fog,
          firstInstance: 0,
          instances: 0,
        };
      run.drawable = drawable;
      run.ghost = ghost;
      run.high = high;
      run.caster = caster;
      run.fog = fog;
      run.firstInstance = first - base;
      run.instances = count;
      frameRunCount++;
      frameStats.instances += count;
      frameStats.triangles += (count * drawable.count) / 3;
      frameStats.tiers[drawable.tier] += count;
      if (drawable.body) {
        frameStats.bodyTiers[drawable.tier] += count;
        frameStats.skinned += count;
      }
      if (fog === FOG_UNITS && !ghost)
        for (let i = first; i < first + count; i++)
          if (instances.f32[i * RECORD_FLOATS + 15] > 0) frameXray = true;
    }
  }

  /** The drawables the frame's batches name, by `World.appearances` row. */
  function setWorld(world: readonly WorldAppearance[]) {
    if (world !== worldRows) {
      clearPropBatches(propBatches);
      propRuns.length = 0;
      propCards.length = 0;
    }
    worldRows = world;
    buildings.setWorld(world);
    dirty = true;
  }

  function pushCardRun(fixed: boolean, fog: number, firstInstance: number, instances: number) {
    const last = cardRuns[cardRunCount - 1];
    if (
      last &&
      last.fixed === fixed &&
      last.fog === fog &&
      last.firstInstance + last.instances === firstInstance
    ) {
      last.instances += instances;
      return;
    }
    let run = cardRuns[cardRunCount];
    if (!run) cardRuns[cardRunCount] = run = { fixed, fog, firstInstance, instances };
    run.fixed = fixed;
    run.fog = fog;
    run.firstInstance = firstInstance;
    run.instances = instances;
    cardRunCount++;
  }

  /** The pose an impostor of `name` shows, and its bounds in that pose. */
  function farPose(name: string): { pose: ModelPose; bounds: Bounds } | null {
    const gpu = appearances.get(name);
    if (!gpu) return null;
    const bundle = gpu.bundle;
    const skeleton = bundle.kind === "skinned" ? (skeletons.get(bundle.skeleton) ?? null) : null;
    const pose = restingModelPose(bundle, skeleton);
    return { pose, bounds: farPoseBounds(bundle, skeleton) };
  }

  let viewKey = "";

  /** Every mesh run with the record buffer it draws from: the packed lab
   *  models', the props', then the presentation frame's. */
  function eachRun(visit: (run: DrawRun, source: GPUBuffer) => void, casting = false) {
    if (records.current) for (let i = 0; i < runCount; i++) visit(runs[i], records.current);
    for (const { run, buffer, pass } of propRuns)
      if (casting || pass !== CAST_PASS) visit(run, buffer);
    if (frameRecords.current)
      for (let i = 0; i < frameRunCount; i++) visit(frameRuns[i], frameRecords.current);
  }

  const fogIndex = (fog?: ModelFog) => (fog === undefined ? -1 : FOG_INDEX[fog]);
  /** Whether a pipeline's part of the installed meshes has anything to draw
   *  that is not switched off. */
  const drawnClass = (c: SurfaceClass) => installedSurfaces[c] && !hiddenSurfaces[c];
  const drawn = (part: SurfacePart) =>
    part === "solid" ? drawnClass("opaque") || drawnClass("room") : drawnClass(part);

  return {
    /** Install a catalog generation's appearances (null clears them). */
    setAppearances: install,
    /** The posed models to draw this frame (soldiers, vehicles, a building). */
    setModels(list: readonly ModelInstance[]) {
      units = list;
      dirty = true;
    },
    setFrame,
    setWorld,
    /** Whether the packed unit mesh draws contain an eligible x-ray subject. */
    get hasXrayMeshes(): boolean {
      return hasXrayMeshes;
    },
    /** Mesh record count, including every instance-index address drawn. */
    get drawnInstances(): number {
      return drawnCount;
    },
    get models(): readonly ModelInstance[] {
      return units;
    },
    /** The static world's batches of the presentation frame: the buildings'
     *  (`buildingLayer.ts`) and the props'; whether what is drawn changed. */
    setStaticFrame(frame: ReturnType<typeof frameViews> | null, stats: BuildingWalkStats): boolean {
      const props = setProps(frame);
      return buildings.setFrame(frame, stats) || props;
    },
    setBuildingsShown(on: boolean) {
      buildings.setShown(on);
    },
    /** Lab diagnostics: draw one surface class of every model and building,
     *  with its depth and its shadow, or none of it. */
    setSurfaceShown(surface: SurfaceClass, on: boolean) {
      hiddenSurfaces[surface] = !on;
    },
    /** Choose a lab's models' draws for the frame's `View` record, when anything changed. */
    prepare(view: Float32Array, key: string) {
      if (!dirty && key === viewKey) return;
      viewKey = key;
      dirty = false;
      pack(view);
    },
    /** Pack `list` exactly as given (its tiers, nothing culled, no corpses) for
     *  an offline draw (the impostor bake); the next `prepare` packs again. */
    packExact(list: readonly ModelInstance[]) {
      const kept = units;
      const keptRuns = frameRunCount;
      const keptCards = frameCardCount;
      const keptProps = propRuns.splice(0);
      units = list;
      [frameRunCount, frameCardCount] = [0, 0];
      pack(null);
      units = kept;
      [frameRunCount, frameCardCount] = [keptRuns, keptCards];
      propRuns.push(...keptProps);
      dirty = true;
    },
    /** An installed appearance's bundle, or null. */
    bundle(name: string): Exclude<Bundle, SkeletonClips> | null {
      return appearances.get(name)?.bundle ?? null;
    },
    /** The pose an impostor of `name` shows, and its bounds in that pose. */
    farPose,
    /** Supported poses for card installation. */
    cardPoses(): { appearance: string; pose: ModelPose; bounds: Bounds }[] {
      const out: { appearance: string; pose: ModelPose; bounds: Bounds }[] = [];
      for (const gpu of appearances.values()) {
        if (gpu.staticCard) out.push({ appearance: gpu.name, ...gpu.staticCard });
        if (gpu.bundle.kind !== "skinned" || !gpu.corpses.length) continue;
        out.push({ appearance: gpu.name, ...farPose(gpu.name)! });
        gpu.corpses.forEach(({ bounds }, lies) =>
          out.push({ appearance: gpu.name, pose: { kind: "corpse", lies }, bounds }),
        );
      }
      return out;
    },
    setCards,
    /** One surface class of every mesh run into one of the sun's cascades,
     *  each at the tier its batch casts with. */
    drawCasters(bound: Drawable3, surface: SurfacePart, submitted?: SubmittedDraws) {
      if (!renderGroup || !drawn(surface)) return;
      const b = bound.with(modelLayout, renderGroup);
      eachRun((run, source) => {
        const caster = run.caster;
        if (!caster) return;
        const part = partRange(caster.parts, surface, hiddenSurfaces);
        if (!part.count) return;
        b.with(modelVertexLayout, caster.mesh.vertices)
          .with(modelRecordLayout, source)
          .withIndexBuffer(caster.mesh.indices, "uint32")
          .drawIndexed(part.count, run.instances, part.first, 0, run.firstInstance);
        recordDraw(submitted, part.count, run.instances);
      }, true);
    },
    /** Whether the high-air models draw on their own plane this frame
     *  (`draw` with `high`), not with the world. */
    setHighApart(on: boolean) {
      highApart = on;
    },
    /** Whether the current mesh runs hold a high-air model. */
    get hasHighMeshes(): boolean {
      let high = false;
      eachRun((run) => (high ||= run.high));
      return high;
    },
    /** Draw one surface class of the mesh runs, all of them or one fog
     *  class's: the world's, or with `high` the high-air plane's. */
    draw(
      bound: Drawable3,
      surface: SurfacePart,
      fog?: ModelFog,
      high = false,
      submitted?: SubmittedDraws,
    ) {
      if (!renderGroup || !drawn(surface)) return;
      const only = fogIndex(fog);
      const b = bound.with(modelLayout, renderGroup);
      eachRun((run, source) => {
        if (run.ghost || (only >= 0 && run.fog !== only)) return;
        if (high ? !run.high : run.high && highApart) return;
        const part = partRange(run.drawable.parts, surface, hiddenSurfaces);
        if (!part.count) return;
        b.with(modelVertexLayout, run.drawable.mesh.vertices)
          .with(modelRecordLayout, source)
          .withIndexBuffer(run.drawable.mesh.indices, "uint32")
          .drawIndexed(part.count, run.instances, part.first, 0, run.firstInstance);
        recordDraw(submitted, part.count, run.instances);
      });
    },
    /** Whether the current mesh runs contain a placement preview. */
    get hasGhostMeshes(): boolean {
      let ghost = false;
      eachRun((run) => (ghost ||= run.ghost));
      return ghost;
    },
    /** Placement previews draw actual mesh geometry, never cards or casters. */
    drawGhosts(bound: Drawable3) {
      if (!renderGroup) return;
      const b = bound.with(modelLayout, renderGroup);
      eachRun((run, source) => {
        if (!run.ghost) return;
        b.with(modelVertexLayout, run.drawable.mesh.vertices)
          .with(modelRecordLayout, source)
          .withIndexBuffer(run.drawable.mesh.indices, "uint32")
          .drawIndexed(
            run.drawable.count,
            run.instances,
            run.drawable.parts.cutout.first,
            0,
            run.firstInstance,
          );
      });
    },
    /** One surface class of every building's modules into the view: the
     *  world's faces. `raw` is the pass `bound` draws into. */
    drawBuildings(
      bound: Drawable3,
      raw: GPURenderPassEncoder,
      surface: SurfacePart,
      submitted?: SubmittedDraws,
    ) {
      if (!renderGroup || !drawn(surface)) return;
      const b = bound.with(modelLayout, renderGroup);
      buildings.draw(
        (vertices, instances, indices) =>
          b
            .with(modelVertexLayout, vertices)
            .with(modelRecordLayout, instances)
            .withIndexBuffer(indices, "uint32"),
        raw,
        surface,
        hiddenSurfaces,
        submitted,
      );
    },
    /** One surface class of the buildings that cast, into one of the sun's cascades. */
    drawBuildingCasters(
      bound: Drawable3,
      raw: GPURenderPassEncoder,
      surface: SurfacePart,
      submitted?: SubmittedDraws,
    ) {
      if (!renderGroup || !drawn(surface)) return;
      const b = bound.with(modelLayout, renderGroup);
      buildings.drawCasters(
        (vertices, instances, indices) =>
          b
            .with(modelVertexLayout, vertices)
            .with(modelRecordLayout, instances)
            .withIndexBuffer(indices, "uint32"),
        raw,
        surface,
        hiddenSurfaces,
        submitted,
      );
    },
    /** Draw one fog class's impostor cards. */
    drawCards(bound: CardDrawable, fog: ModelFog, submitted?: SubmittedDraws) {
      if (!cards) return;
      const only = fogIndex(fog);
      const b = bound.with(cards);
      for (const h of propCards) {
        if (h.fog !== only || !h.buffer) continue;
        b.with(modelRecordLayout, h.buffer).draw(6, h.count, 0, 0);
        recordDraw(submitted, 6, h.count);
      }
      for (let i = 0; i < cardRunCount; i++) {
        const run = cardRuns[i];
        if (run.fog !== only) continue;
        if (!cardRecords.current) continue;
        b.with(modelRecordLayout, cardRecords.current).draw(6, run.instances, 0, run.firstInstance);
        recordDraw(submitted, 6, run.instances);
      }
    },
    /** Switch texture channels on or off for every material (the workbench's
     *  per-channel toggles); a switched-off channel draws its factors. */
    setTextureChannels(next: Partial<Record<TextureChannel, boolean>>) {
      Object.assign(channels, next);
      writeMaterials();
    },
    buildingStats: (): BuildingStats => buildings.stats(),
    /** The buildings as tables for a second client, and the module instances
     *  the last view drew (`buildingLayer.ts`). */
    drawnBuildings: () => buildings.drawn(),
    /** The props the last frame drew, by appearance in name order, each per
     *  tier 0–3. */
    drawnProps: () => drawnProps(propBatches, worldRows),
    stats: (): ModelStats => ({
      ...stats,
      appearanceTextureBytes: { ...stats.appearanceTextureBytes },
      installed: [...stats.installed],
      tiers: [...stats.tiers],
      bodyTiers: [...stats.bodyTiers],
    }),
    dispose() {
      clearPropBatches(propBatches);
      buildings.dispose();
      cardScope?.release();
      scope?.release();
    },
  };
}
export type ModelLayer = Awaited<ReturnType<typeof createModelLayer>>;
