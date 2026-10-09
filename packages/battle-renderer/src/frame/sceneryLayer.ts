// The scenery layer: every placed tree, hedgerow shrub and piece of
// forest-floor dressing, instanced from its appearance's tiers, in the
// frame's own passes. The trees and shrubs are chunked, culled and tiered by
// the static chunk owner (`staticChunks.ts`); the buffers, meshes and
// materials here are the layer's own. Opaque, so all of it is in the depth
// prepass (FogVisibility's tile cull reads it like any surface).
//
// - The forest (the simulation's trunks, one tree each) casts sun shadows
//   into every cascade (the trees in view, and those whose shadow can land in
//   it), receives them, and takes FogTerm through the faces group. The
//   shrubs under a tree line (the understorey) are drawn the same way. A tree is
//   seen or unseen whole: every fragment probes fog at its crown's
//   heart with no facing test, as ground probes do. A crown is a porous
//   volume inside the simulation's foliage, so the sweep's rule (sight into
//   a forest fades with the foliage crossed) decides it, and fog never
//   splits a crown along its sunlit and shaded halves.
// - Scenery past the map is drawn like the backdrop it stands on: lit and
//   hazed, never shadowed, casting nothing, and permanently fogged outside the
//   playable rectangle.
// - The forest floor's dressing is drawn as a forest tree is (shadowed,
//   fogged whole at its own heart) and casts nothing: it is small and stands
//   under the crowns. There is a great deal of it, so it is kept a cell of
//   ground at a time in one bounded pool (`scenery/dressing.ts`): a cell
//   draws whole from its slot at one tier, and the vertex stage shrinks each
//   piece to nothing as it nears a few pixels. A piece is drawn from both
//   sides: a frond is one sheet.
//
// Foliage adds leaf clumps per pixel (3D value noise bending the normal and
// darkening the gaps, in the tree's own space so it never swims), fading to
// the plain crown as a pixel grows past a clump.
//
// A tree the side knows has fallen (`setFelled`) leaves the forest and is
// drawn apart, few as they are: tipped about its stump's top on the clock,
// then lying, pressed flat (`scenery/felled.ts`), with its stump standing
// where it grew. It casts, fogs and lights as a standing tree does.
//
// A shrub whose foot stands on ground the side has seen cleared (a lane a
// vehicle knocked through) is not drawn: `setCleared` rebuilds the
// understorey without it, and the dressing is laid again without what stood
// on that ground. A forest tree leaves only by its fall (`setFelled`).
//
// On a view change `prepare` sorts trees into tiers by projected height
// (`scenery/lod.ts`) and uploads the near trees' per-tier lists; far chunks
// draw at tier 3 straight from a static buffer.
import { tgpu, d, std, type TgpuRenderPass } from "typegpu";
import { pcgHash } from "../shaders/pcgHash";
import type { StaticBundle } from "@packages/scene-assets/src/schema";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { createDetailView, detailKey, setDetailView } from "./detailView";
import { VERTEX_FLOATS } from "../mesh";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { WorldScenery } from "../scene";
import { kindSize, tierMesh } from "../scenery/appearance";
import {
  createDressingCache,
  DRESSING_GONE,
  dressingCounts,
  selectDressing,
  setDressingCleared,
  type DressingCache,
} from "../scenery/dressing";
import {
  chunkTier,
  INSTANCE_FLOATS,
  sceneryChunks,
  tierFor,
  TIER_COUNT,
  treeInstances,
  type PlacedInstances,
  type TierView,
} from "../scenery/lod";
import {
  createStagedLevels,
  selectChunks,
  stageNear,
  sunShadow,
  type StagedLevels,
  type StaticChunks,
} from "./staticChunks";
import type { EnvironmentFrame } from "./environmentFrame";
import type { FogVisibility } from "./fogVisibility";
import { fogCoverage, fogTerm } from "./fogTerm";
import { type CameraGroup, vertexBuffer, vertexLayout, type VertexBuffer } from "./geometry";
import { FRAME_MSAA, WORLD_OUT, worldTargets } from "./targets";
import type { GpuRegistry, GpuSlot, Releasable } from "./registry";
import type { GroundMarks } from "./scarTexture";
import { TREE_FIELD, TREE_FLOATS } from "../scenery/placement";
import {
  FELLED_FLOATS,
  felledRecords,
  stumpMesh,
  stumpRecords,
  treeIndex,
  groupByKey,
  trunkAt,
  crownBase,
  KICK_RADII,
  type FelledKind,
  type FelledTree,
} from "../scenery/felled";
import type { FelledRules } from "../terrain/biome";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** One population's instances: placed, drawn per tier this frame (tier 3
 *  counts the far chunks' too), the triangles those draw per view pass, and
 *  how many are drawn into each of the sun's cascades. */
export interface SceneryPopulationStats {
  placed: number;
  tiers: number[];
  triangles: number;
  casters: number;
}
export interface SceneryStats {
  /** Appearances installed. */
  kinds: number;
  forest: SceneryPopulationStats;
  /** The shrubs under tree lines. */
  understorey: SceneryPopulationStats;
  backdrop: SceneryPopulationStats;
  /** The forest floor's dressing: the pieces laid (`placed`) in `cells`
   *  cells of a pool of `bytes` bytes, the pieces drawn per tier, and
   *  whether wanted cells are still to be laid. */
  dressing: SceneryPopulationStats & { cells: number; bytes: number; pending: boolean };
  /** The felled trees drawn (falling or lying), whether any still falls,
   *  and their stumps. */
  felled: { trees: number; falling: boolean; stumps: number };
  /** Draw calls in the last whole frame, over every pass. */
  draws: number;
}

const PlacedInstance = d.unstruct({ pose: d.float32x4, shape: d.float32x4, tint: d.float32x4 });
const placedLayout = tgpu.vertexLayout(d.disarrayOf(PlacedInstance), "instance");
const placedAttribs = { ...vertexLayout.attrib, ...placedLayout.attrib };
type InstanceBuffer = ReturnType<typeof instanceBuffer>;
function instanceBuffer(root: Root, capacity: number) {
  return root.createBuffer(placedLayout.schemaForCount(Math.max(1, capacity))).$usage("vertex");
}
/** A falling or lying tree (`scenery/felled.ts`): a placed instance, then
 *  its `FellPose`: (toward x, y, angle, hinge), (squash, spread, trunk,
 *  landed), (crown, 0, 0, 0). */
const FelledInstance = d.unstruct({
  pose: d.float32x4,
  shape: d.float32x4,
  tint: d.float32x4,
  fall: d.float32x4,
  rest: d.float32x4,
  lie: d.float32x4,
});
const felledLayout = tgpu.vertexLayout(d.disarrayOf(FelledInstance), "instance");
const felledAttribs = { ...vertexLayout.attrib, ...felledLayout.attrib };
type FelledBuffer = ReturnType<typeof felledBuffer>;
function felledBuffer(root: Root, capacity: number) {
  return root.createBuffer(felledLayout.schemaForCount(Math.max(1, capacity))).$usage("vertex");
}

/** Square chunks instances are bucketed in for tier selection, metres. */
const CHUNK_M = 128;
/** The most cells of dressing kept on the GPU at once: more than a play
 *  camera sees pieces in. A wider view draws its nearest. */
const DRESSING_SLOTS = 128;
/** The cells laid a view: each costs the CPU a fraction of a millisecond, so
 *  a cut's cells arrive over the frames after it. */
const DRESSING_CELLS_PER_VIEW = 2;
/** Foliage roughness: leaves scatter; the environment's specular stays small. */
const LEAF_ROUGHNESS = 0.85;
const BARK_ROUGHNESS = 0.9;
/** Shadow casters draw this many tiers coarser than the view: four cascades
 *  of the finest crowns cost more than the crowns themselves. */
const CASTER_COARSER = 1;

const placedVaryings = {
  clip: d.builtin.position,
  world: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  local: d.vec3f,
  heart: d.vec3f,
};

const placedIn = {
  position: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  pose: d.vec4f,
  shape: d.vec4f,
  tint: d.vec4f,
};
const placedOut = {
  // Invariant, so the depth prepass and the colour pass agree exactly.
  clip: d.invariant(d.builtin.position) as unknown as typeof d.builtin.position,
  world: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  local: d.vec3f,
  heart: d.vec3f,
};

/** A point of the appearance, already scaled, turned by the instance's yaw
 *  about +Z and stood on its foot. */
const placedPoint = tgpu.fn(
  [d.vec3f, d.vec4f],
  d.vec3f,
)((local, pose) => {
  "use gpu";
  const c = std.cos(pose.w);
  const s = std.sin(pose.w);
  return d.vec3f(
    local.x * c - local.y * s + pose.x,
    local.x * s + local.y * c + pose.y,
    local.z + pose.z,
  );
});

/** The appearance's normal under the instance's scale (inverted) and yaw. */
const placedNormal = tgpu.fn(
  [d.vec3f, d.vec4f, d.f32],
  d.vec3f,
)((normal, shape, yaw) => {
  "use gpu";
  const c = std.cos(yaw);
  const s = std.sin(yaw);
  const nx = normal.x / shape.x;
  const ny = normal.y / shape.y;
  return std.normalize(d.vec3f(nx * c - ny * s, nx * s + ny * c, normal.z / shape.z));
});

/** Places the appearance: scale per axis, yaw about +Z, then the instance's
 *  foot. Normals take the inverse scale. */
const placedVertex = tgpu.vertexFn({ in: placedIn, out: placedOut })((v) => {
  "use gpu";
  const local = d.vec3f(
    v.position.x * v.shape.x,
    v.position.y * v.shape.y,
    v.position.z * v.shape.z,
  );
  const world = placedPoint(local, v.pose);
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal: placedNormal(v.normal, v.shape, v.pose.w),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    local: d.vec3f(local.x + v.tint.w, local.y, local.z),
    heart: d.vec3f(v.pose.x, v.pose.y, v.pose.z + v.shape.w),
  };
});

/** Places a piece of dressing as a tree is placed, shrunk about its foot as
 *  it nears a few pixels: its record's last float is its height over the
 *  projected height it starts to shrink at (`scenery/dressing.ts`), so its
 *  size on screen over that height is that float times the pixels a metre
 *  covers at its distance. */
const dressingVertex = tgpu.vertexFn({ in: placedIn, out: placedOut })((v) => {
  "use gpu";
  const cam = typegpuCameraLayout.$.cam;
  // The projection's vertical scale is the length of the view-projection's
  // second row over the world's axes; times half the viewport's height it is
  // the pixels a metre covers a metre away.
  const row = d.vec3f(
    std.mul(cam.viewProj, d.vec4f(1, 0, 0, 0)).y,
    std.mul(cam.viewProj, d.vec4f(0, 1, 0, 0)).y,
    std.mul(cam.viewProj, d.vec4f(0, 0, 1, 0)).y,
  );
  const pixelsPerMetre = 0.5 * cam.height * std.length(row);
  const size = (v.tint.w * pixelsPerMetre) / std.max(std.distance(v.pose.xyz, cam.eye), 0.001);
  const grow = std.smoothstep(DRESSING_GONE, 1, size);
  const local = d.vec3f(
    v.position.x * v.shape.x * grow,
    v.position.y * v.shape.y * grow,
    v.position.z * v.shape.z * grow,
  );
  const world = placedPoint(local, v.pose);
  return {
    clip: std.mul(cam.viewProj, d.vec4f(world, 1)),
    world,
    normal: placedNormal(v.normal, v.shape, v.pose.w),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    local,
    heart: d.vec3f(v.pose.x, v.pose.y, v.pose.z + v.shape.w * grow),
  };
});

/** `d` metres from a trunk's axis with what lies past its `radius` scaled
 *  by `k` (`scenery/felled.ts`'s `pastTrunk`). */
const pastTrunk = tgpu.fn(
  [d.f32, d.f32, d.f32],
  d.f32,
)(/* wgsl */ `(d: f32, radius: f32, k: f32) -> f32 {
  let m = abs(d);
  return sign(d) * (min(m, radius) + max(m - radius, 0.0) * k);
}`);

/** Places a felled tree as a standing one is placed, then poses it as
 *  `felledPoint` (`scenery/felled.ts`, its CPU mirror) does: tipped `fall.z`
 *  radians toward `fall.xy` about the hinge `fall.w` above its foot, kicked
 *  off its stump and lowered as it lands (`rest.w`); its leaves, and its
 *  bark above the crown's base (`lie.x`), pressed by `rest.x` up-and-down
 *  and `rest.y` across; its trunk below keeps its radius `rest.z`. */
const felledVertex = tgpu.vertexFn({
  in: { ...placedIn, fall: d.vec4f, rest: d.vec4f, lie: d.vec4f },
  out: placedOut,
})((v) => {
  "use gpu";
  const local = d.vec3f(
    v.position.x * v.shape.x,
    v.position.y * v.shape.y,
    v.position.z * v.shape.z,
  );
  const yc = std.cos(v.pose.w);
  const ys = std.sin(v.pose.w);
  const p = d.vec3f(local.x * yc - local.y * ys, local.x * ys + local.y * yc, local.z);
  const t = v.fall.xy;
  const c = std.cos(v.fall.z);
  const s = std.sin(v.fall.z);
  const hinge = v.fall.w;
  const trunk = v.rest.z;
  const landed = v.rest.w;
  const core = std.select(trunk, 0, v.color.w > 0.5 || local.z > v.lie.x);
  const a = std.dot(p.xy, t);
  const across = std.sub(p.xy, std.mul(t, a));
  const side = std.length(across);
  // Below the cut the stump stands in its place.
  const z = std.max(p.z - hinge, 0);
  const flat = std.add(
    std.mul(t, a * c + z * s + KICK_RADII * trunk * landed),
    std.mul(across, pastTrunk(side, core, v.rest.y) / std.max(side, 1e-6)),
  );
  const world = d.vec3f(
    v.pose.x + flat.x,
    v.pose.y + flat.y,
    v.pose.z + trunk * landed + hinge * (1 - landed) + z * c - pastTrunk(a, core, v.rest.x) * s,
  );
  // The normal turns with the tree; what was pressed takes its inverse.
  const n = placedNormal(v.normal, v.shape, v.pose.w);
  const na = std.dot(n.xy, t);
  const nAcross = std.sub(n.xy, std.mul(t, na));
  const pressAcross = std.select(1, v.rest.y, side > core);
  const pressUp = std.select(1, v.rest.x, std.abs(a) > core);
  const nFlat = std.add(std.mul(t, na * c + n.z * s), std.mul(nAcross, 1 / pressAcross));
  const heart = v.shape.w - hinge;
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal: std.normalize(d.vec3f(nFlat.x, nFlat.y, (n.z * c - na * s) / pressUp)),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    local: d.vec3f(local.x + v.tint.w, local.y, local.z),
    heart: d.vec3f(
      v.pose.x + t.x * (heart * s + KICK_RADII * trunk * landed),
      v.pose.y + t.y * (heart * s + KICK_RADII * trunk * landed),
      v.pose.z + trunk * landed + hinge * (1 - landed) + heart * c,
    ),
  };
});

/** Value noise on a unit 3D lattice, in [0, 1], with an integer hash per
 *  corner (no sin-hash). */
const valueNoise3 = tgpu
  .fn(
    [d.vec3f],
    d.f32,
  )(/* wgsl */ `(p: vec3f) -> f32 {
  let cell = floor(p);
  let f = p - cell;
  let u = f * f * (3.0 - 2.0 * f);
  let i = vec3u(bitcast<vec3u>(vec3i(cell)));
  var h = array<f32, 8>();
  for (var k = 0u; k < 8u; k++) {
    let q = i + vec3u(k & 1u, (k >> 1u) & 1u, k >> 2u);
    let s = pcgHash((q.x * 1597334677u) ^ (q.y * 3812015801u) ^ (q.z * 2798796415u));
    h[k] = f32(s) * (1.0 / 4294967295.0);
  }
  let x0 = mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
  let x1 = mix(mix(h[4], h[5], u.x), mix(h[6], h[7], u.x), u.y);
  return mix(x0, x1, u.z);
}`)
  .$uses({ pcgHash });

/** Leaf clumps about half a metre across: the normal bent toward each clump and the
 *  gaps between them darkened, faded out by `fade`. Returns (normal, shade). */
const leafClumps = tgpu
  .fn(
    [d.vec3f, d.vec3f, d.f32],
    d.vec4f,
  )(/* wgsl */ `(p: vec3f, n: vec3f, fade: f32) -> vec4f {
  let q = p * 2.1;
  let bend = vec3f(
    valueNoise3(q),
    valueNoise3(q + vec3f(17.3, 5.1, 9.7)),
    valueNoise3(q + vec3f(3.7, 29.3, 13.1)),
  ) - vec3f(0.5);
  let bent = normalize(n + bend * (0.7 * fade));
  let gap = valueNoise3(p * 3.4 + vec3f(7.0, 1.0, 3.0));
  let shade = mix(1.0, 0.72 + 0.4 * gap, fade);
  return vec4f(bent, shade);
}`)
  .$uses({ valueNoise3 });

export async function createSceneryLayer(
  root: Root,
  registry: GpuRegistry,
  environment: EnvironmentFrame,
) {
  /** Shading shared by both populations: albedo, leaf clumps and the light. */
  const surface = tgpu.fn(
    [d.vec3f, d.vec3f, d.vec4f, d.vec3f, d.f32],
    d.vec3f,
  )((world, normal, color, local, sun) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(normal);
    let shade = d.f32(1);
    let roughness = d.f32(BARK_ROUGHNESS);
    // Detail fades as a pixel grows past a clump (derivatives before any branch).
    const fade = 1 - std.smoothstep(0.08, 0.45, std.length(std.fwidth(world)));
    // Past the clumps' fade there is nothing to add: skip the noise.
    if (color.w > 0.5 && fade > 0) {
      const clumps = leafClumps(local, n, fade);
      n = clumps.xyz;
      shade = clumps.w;
      roughness = LEAF_ROUGHNESS;
    }
    const lit = environment.shade(
      std.mul(color.xyz, shade),
      d.vec3f(0),
      roughness,
      0,
      0,
      1,
      n,
      world,
      sun,
      eye,
    );
    return lit.xyz;
  });
  const forestFragment = tgpu.fragmentFn({ in: placedVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    const n = std.normalize(v.normal);
    // The whole tree probes at its heart through FogTerm (and so fogSeenSurface):
    // a zero normal means no facing test, and no roof rule, which needs an
    // upward face; unseen, the whole crown's mask says so, and the fog mask
    // pass gives it the style through fogLook.
    const seen = fogTerm(v.heart, d.vec3f(0), v.clip.xy, false);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const lit = surface(v.world, v.normal, v.color, v.local, sun);
    return { color: d.vec4f(lit, 1), fog: fogCoverage(seen, 1) };
  });
  /** Scenery past the map: unshadowed, but fogged as a forest tree is (the
   *  sight maps run on past the edge over open ground). */
  const backdropFragment = tgpu.fragmentFn({ in: placedVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    const seen = fogTerm(v.heart, d.vec3f(0), v.clip.xy, false);
    return {
      color: d.vec4f(surface(v.world, v.normal, v.color, v.local, 1), 1),
      fog: fogCoverage(seen, 1),
    };
  });

  const base = {
    attribs: placedAttribs,
    vertex: placedVertex,
    primitive: { topology: "triangle-list", cullMode: "back" },
  } as const;
  const prepass = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  const caster = root.createRenderPipeline({
    ...base,
    // Both faces cast: a crown's far side must shadow its own underside.
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: battleWorldDepth("read-write"),
  });
  const forestColour = root.createRenderPipeline({
    ...base,
    fragment: forestFragment,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  const backdropColour = root.createRenderPipeline({
    ...base,
    fragment: backdropFragment,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  const dressingBase = {
    attribs: placedAttribs,
    vertex: dressingVertex,
    primitive: { topology: "triangle-list", cullMode: "none" },
  } as const;
  const dressingPrepass = root.createRenderPipeline({
    ...dressingBase,
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  const dressingColour = root.createRenderPipeline({
    ...dressingBase,
    fragment: forestFragment,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  const felledBase = {
    attribs: felledAttribs,
    vertex: felledVertex,
    primitive: { topology: "triangle-list", cullMode: "back" },
  } as const;
  const felledPrepass = root.createRenderPipeline({
    ...felledBase,
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  const felledCaster = root.createRenderPipeline({
    ...felledBase,
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: battleWorldDepth("read-write"),
  });
  const felledColour = root.createRenderPipeline({
    ...felledBase,
    fragment: forestFragment,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  await Promise.all(
    [
      prepass,
      caster,
      forestColour,
      backdropColour,
      dressingPrepass,
      dressingColour,
      felledPrepass,
      felledCaster,
      felledColour,
    ].map((pipeline) => pipeline.initAsync()),
  );

  /** A kind's vertices per tier. */
  type TierMeshes = { buffer: VertexBuffer; vertices: number }[];
  interface Population {
    chunks: StaticChunks;
    /** The near chunks' instances, per kind and tier. */
    staged: StagedLevels;
    /** Whether the population casts sun shadows. */
    casts: boolean;
    /** Per kind and tier: the appearance's vertices. */
    meshes: TierMeshes[];
    /** Tier thresholds this population is sorted by, projected pixels. */
    lodPx: TierView["lodPx"];
    /** Per kind: the static far buffer (every instance, chunk order). */
    far: InstanceBuffer[];
    /** Per kind and tier: this frame's near instances. */
    near: { slot: GpuSlot<InstanceBuffer>; capacity: number }[][];
  }
  /** The forest floor's dressing: the cells laid, each in its slot of `pool`. */
  interface Dressing {
    cache: DressingCache;
    meshes: TierMeshes[];
    /** The cache's slots, of the field's `capacity` records each. */
    pool: InstanceBuffer;
    lodPx: TierView["lodPx"];
  }
  /** A population on the simulation's ground: what was placed, and of it
   *  what stands where the side has seen no ground cleared. */
  interface Standing {
    drawn: Population;
    /** The population's own scope, rebuilt when what stands changes. */
    scope: GpuRegistry;
    placed: Float32Array;
    /** Where each standing instance starts in `placed`; null for all. */
    kept: number[] | null;
  }
  /** The felled trees: the forest's trees by trunk id, each kind's stump,
   *  and the instances drawn this frame, grouped by kind and tier. */
  interface Felled {
    rules: FelledRules;
    /** Per forest tree: its trunk id; and where each id's tree is placed. */
    ids: Uint32Array;
    index: Map<number, number>;
    stumpMeshes: ({ buffer: VertexBuffer; vertices: number } | null)[];
    /** Per kind: what it lends its fall (zero for no tree). */
    kinds: FelledKind[];
    trees: { slot: GpuSlot<FelledBuffer>; capacity: number };
    stumps: { slot: GpuSlot<InstanceBuffer>; capacity: number };
    /** `[kind · TIER_COUNT + tier, first, count]` runs of this frame's trees. */
    treeRuns: number[];
    /** `[kind, first, count]` runs of the stumps. */
    stumpRuns: number[];
    /** Whether a tree still falls: the next frame packs them again. */
    moving: boolean;
  }
  interface Loaded {
    scope: GpuRegistry;
    dressing: Dressing;
    forest: Standing;
    understorey: Standing;
    sizes: ReturnType<typeof kindSize>[];
    backdrop: Population;
    felled: Felled;
  }
  let loaded: Loaded | null = null;
  let viewKey = "";
  /** What the side knows has fallen, by trunk id, and the frame's clock. */
  let felledList: readonly FelledTree[] = [];
  let felledIds = new Set<number>();
  let clock = 0;
  /** The felled trees need packing again (a new list, view or clock). */
  let felledDirty = true;
  // Where the sun's shadows fall: `prepare` sets it per view.
  const { sun_azimuth, sun_elevation, cascades } = environment.light;
  const sun = { azimuth: sun_azimuth, elevation: sun_elevation, maxFarM: cascades.max_far_m };
  const shadow = { fall: [0, 0] as [number, number], reach: 0 };
  const view: TierView = { ...createDetailView(), lodPx: [1, 1, 1], shadow };

  function population(
    scope: GpuRegistry,
    placed: PlacedInstances,
    meshes: TierMeshes[],
    lodPx: TierView["lodPx"],
    casts: boolean,
  ): Population {
    const chunks = sceneryChunks(placed, meshes.length, CHUNK_M);
    return {
      chunks,
      staged: createStagedLevels(chunks),
      casts,
      meshes,
      lodPx,
      far: chunks.sorted.map((instances) => {
        const buffer = scope.own(instanceBuffer(root, instances.length / INSTANCE_FLOATS));
        if (instances.length) buffer.write(instances.buffer);
        return buffer;
      }),
      near: chunks.sorted.map(() =>
        Array.from({ length: TIER_COUNT }, () => ({
          slot: scope.slot<InstanceBuffer>(),
          capacity: 0,
        })),
      ),
    };
  }

  /** Write the first `count` records (`stride` floats each) of `data` into
   *  `held`'s buffer, made anew by `make` at twice the need when too small. */
  function writeGrowing<
    B extends Releasable & { write: (data: ArrayBuffer, options: { endOffset: number }) => void },
  >(
    held: { slot: GpuSlot<B>; capacity: number },
    data: Float32Array,
    count: number,
    stride: number,
    make: (capacity: number) => B,
  ) {
    if (count === 0) return;
    if (held.capacity < count) {
      held.capacity = Math.max(64, count * 2);
      held.slot.set(make(held.capacity));
    }
    held.slot.current!.write(data.buffer as ArrayBuffer, { endOffset: count * stride * 4 });
  }

  function upload(pop: Population) {
    for (let k = 0; k < pop.chunks.kinds; k++)
      for (let t = 0; t < TIER_COUNT; t++)
        writeGrowing(
          pop.near[k][t],
          pop.staged.records[k][t],
          pop.staged.counts[k][t],
          INSTANCE_FLOATS,
          (n) => instanceBuffer(root, n),
        );
  }

  /** Anything a bound pipeline can draw instances through. */
  interface Drawable {
    with(layout: typeof vertexLayout, buffer: VertexBuffer): Drawable;
    with(layout: typeof placedLayout, buffer: InstanceBuffer): Drawable;
    with(layout: typeof felledLayout, buffer: FelledBuffer): Drawable;
    draw(vertices: number, instances: number, firstVertex?: number, firstInstance?: number): void;
  }
  let draws = 0;
  let lastDraws = 0;
  /** Draw a population into the view, or with `casting` into a cascade: its
   *  casters, each near tier with a mesh `coarser` tiers down (a cascade
   *  texel is larger than the leaf relief). */
  function drawPopulation(pop: Population, bound: Drawable, casting = false, coarser = 0) {
    for (let k = 0; k < pop.chunks.kinds; k++) {
      for (let t = 0; t < TIER_COUNT; t++) {
        const count = pop.staged.counts[k][t];
        if (count === 0) continue;
        const mesh = pop.meshes[k][Math.min(t + coarser, TIER_COUNT - 1)];
        bound
          .with(vertexLayout, mesh.buffer)
          .with(placedLayout, pop.near[k][t].slot.current!)
          .draw(mesh.vertices, count);
        draws++;
      }
      const far = casting ? pop.chunks.cast[k] : pop.chunks.ranges[k][TIER_COUNT - 1];
      if (far.length === 0) continue;
      const mesh = pop.meshes[k][TIER_COUNT - 1];
      const withMesh = bound.with(vertexLayout, mesh.buffer).with(placedLayout, pop.far[k]);
      for (let r = 0; r < far.length; r += 2) {
        withMesh.draw(mesh.vertices, far[r + 1], 0, far[r]);
        draws++;
      }
    }
  }

  /** Draw the dressing's cells in view: per tier and kind, each cell's
   *  records of that kind. The pipeline, its groups and buffers go through
   *  the typed path once per mesh; the cells after it change only the range,
   *  and go straight to the pass. */
  function drawDressing(dressing: Dressing, bound: Drawable, raw: GPURenderPassEncoder) {
    const { cache, meshes, pool } = dressing;
    const { capacity, kinds } = cache.field;
    for (let t = 0; t < TIER_COUNT; t++)
      for (let n = 0; n < kinds.length; n++) {
        const mesh = meshes[kinds[n]][t];
        let set = false;
        for (const cell of cache.drawn[t]) {
          const count = cell.starts[n + 1] - cell.starts[n];
          if (count === 0) continue;
          const first = cell.slot * capacity + cell.starts[n];
          if (set) raw.draw(mesh.vertices, count, 0, first);
          else
            bound
              .with(vertexLayout, mesh.buffer)
              .with(placedLayout, pool)
              .draw(mesh.vertices, count, 0, first);
          set = true;
          draws++;
        }
      }
  }

  /** Pack the felled trees at the frame's clock, each at the tier its
   *  standing height takes from the eye, grouped by kind and tier. */
  function packFelled(felled: Felled, forest: Standing, sizes: ReturnType<typeof kindSize>[]) {
    const now = felledRecords(
      felledList,
      forest.placed,
      felled.index,
      sizes,
      felled.kinds,
      felled.rules,
      clock,
    );
    felled.moving = now.moving;
    const keys = Array.from(now.kinds, (kind, i) => {
      const o = i * FELLED_FLOATS;
      const [x, y, z] = [now.records[o], now.records[o + 1], now.records[o + 2]];
      const distance = Math.hypot(x - view.eye[0], y - view.eye[1], z - view.eye[2]);
      return kind * TIER_COUNT + tierFor(now.heights[i], distance, view);
    });
    const grouped = groupByKey(now.records, FELLED_FLOATS, keys);
    felled.treeRuns = grouped.runs;
    writeGrowing(felled.trees, grouped.records, keys.length, FELLED_FLOATS, (n) =>
      felledBuffer(root, n),
    );
  }

  /** The stumps of what the side knows has fallen, grouped by kind. */
  function packStumps(felled: Felled, forest: Standing) {
    const now = stumpRecords(felledList, forest.placed, felled.index, felled.rules.stump_height_m);
    const grouped = groupByKey(now.records, INSTANCE_FLOATS, Array.from(now.kinds));
    felled.stumpRuns = grouped.runs;
    writeGrowing(felled.stumps, grouped.records, now.kinds.length, INSTANCE_FLOATS, (n) =>
      instanceBuffer(root, n),
    );
  }

  /** Draw the felled trees (a tier `coarser` for a cascade) through
   *  `trees`, and their stumps through `stumps`. */
  function drawFelled(felled: Felled, trees: Drawable, stumps: Drawable, coarser = 0) {
    const { treeRuns, stumpRuns } = felled;
    const meshes = loaded!.forest.drawn.meshes;
    for (let r = 0; r < treeRuns.length; r += 3) {
      const [kind, tier] = [Math.floor(treeRuns[r] / TIER_COUNT), treeRuns[r] % TIER_COUNT];
      const mesh = meshes[kind][Math.min(tier + coarser, TIER_COUNT - 1)];
      trees
        .with(vertexLayout, mesh.buffer)
        .with(felledLayout, felled.trees.slot.current!)
        .draw(mesh.vertices, treeRuns[r + 2], 0, treeRuns[r + 1]);
      draws++;
    }
    for (let r = 0; r < stumpRuns.length; r += 3) {
      const mesh = felled.stumpMeshes[stumpRuns[r]]!;
      stumps
        .with(vertexLayout, mesh.buffer)
        .with(placedLayout, felled.stumps.slot.current!)
        .draw(mesh.vertices, stumpRuns[r + 2], 0, stumpRuns[r + 1]);
      draws++;
    }
  }

  /** Lab diagnostics: the trees draw and cast, or do neither, and the shrubs
   *  under tree lines with them or not; the dressing draws or does not. */
  let treesShown = true;
  let understoreyShown = true;
  let dressingShown = true;
  /** The trees drawn: the forest's, and the backdrop's. */
  const trees = () => (treesShown ? loaded : null);
  /** The populations that stand on the simulation's ground, lit, shadowed
   *  and casting alike: the forest, and the shrubs under its tree lines. */
  const standing = () => {
    const drawn = trees();
    if (!drawn) return [];
    return understoreyShown ? [drawn.forest.drawn, drawn.understorey.drawn] : [drawn.forest.drawn];
  };
  /** Every population drawn into the view. */
  const populations = () => (trees() ? [...standing(), loaded!.backdrop] : []);
  const dressing = () => (dressingShown && loaded ? loaded.dressing : null);

  /** Rebuild `stand` without what `gone` names (by its offset in `placed`);
   *  false where the same instances stand as before. */
  function restand(stand: Standing, gone: (offset: number) => boolean) {
    const { scope, sizes } = loaded!;
    const all = stand.placed;
    const kept: number[] = [];
    for (let o = 0; o < all.length; o += TREE_FLOATS) if (!gone(o)) kept.push(o);
    const before = stand.kept;
    const same = before
      ? kept.length === before.length && kept.every((o, k) => o === before[k])
      : kept.length * TREE_FLOATS === all.length;
    if (same) return false;
    const left = new Float32Array(kept.length * TREE_FLOATS);
    kept.forEach((o, k) => left.set(all.subarray(o, o + TREE_FLOATS), k * TREE_FLOATS));
    stand.scope.release();
    stand.scope = scope.scope();
    const { meshes, lodPx } = stand.drawn;
    stand.drawn = population(stand.scope, treeInstances(left, sizes), meshes, lodPx, true);
    stand.kept = kept;
    return true;
  }
  /** The forest stands without the trees the side knows have fallen: a
   *  forest tree leaves only by its fall, so it never vanishes without one. */
  function restandForest() {
    const { ids } = loaded!.felled;
    return restand(loaded!.forest, (o) => felledIds.has(ids[o / TREE_FLOATS]));
  }

  return {
    /** The world's scenery (placement and appearances); `null` draws none. */
    set(next: WorldScenery | null) {
      loaded?.scope.release();
      loaded = null;
      viewKey = "";
      if (!next) return;
      const scope = registry.scope();
      const bundles: StaticBundle[] = next.placement.kinds.map((name) => {
        const bundle = next.appearances.get(name);
        if (!bundle) throw new Error(`scenery: no appearance "${name}" was handed to the frame`);
        return bundle;
      });
      const sizes = bundles.map(kindSize);
      const tiers = bundles.map((bundle) =>
        Array.from({ length: TIER_COUNT }, (_, t) => {
          const mesh = tierMesh(bundle, t);
          return {
            buffer: scope.own(vertexBuffer(root, mesh)),
            vertices: mesh.length / VERTEX_FLOATS,
          };
        }),
      );
      const trees = (within: GpuRegistry, placed: Float32Array, casts: boolean) =>
        population(within, treeInstances(placed, sizes), tiers, next.lodPx, casts);
      const stand = (placed: Float32Array): Standing => {
        const own = scope.scope();
        return { drawn: trees(own, placed, true), scope: own, placed, kept: null };
      };
      const stumpHeight = next.felled.stump_height_m;
      const forestKinds = new Set<number>();
      for (let o = 0; o < next.placement.forest.length; o += TREE_FLOATS)
        forestKinds.add(next.placement.forest[o + TREE_FIELD.kind]);
      const felled: Felled = {
        rules: next.felled,
        ids: next.placement.forestIds,
        index: treeIndex(next.placement.forestIds),
        // A stump for each kind the forest stands (hedges and dressing have
        // no bole to cut).
        kinds: bundles.map((bundle, kind) => {
          if (!forestKinds.has(kind)) return { trunk: 0, crown: 0 };
          const tier = tierMesh(bundle, 0);
          return { trunk: trunkAt(tier, stumpHeight).radius, crown: crownBase(tier) };
        }),
        stumpMeshes: bundles.map((bundle, kind) => {
          if (!forestKinds.has(kind)) return null;
          const mesh = stumpMesh(tierMesh(bundle, 0), stumpHeight, next.felled.cut);
          return {
            buffer: scope.own(vertexBuffer(root, mesh)),
            vertices: mesh.length / VERTEX_FLOATS,
          };
        }),
        trees: { slot: scope.slot<FelledBuffer>(), capacity: 0 },
        stumps: { slot: scope.slot<InstanceBuffer>(), capacity: 0 },
        treeRuns: [],
        stumpRuns: [],
        moving: false,
      };
      const field = next.placement.dressing;
      // A small map's forests are fewer cells than the pool would hold.
      const slots = Math.min(DRESSING_SLOTS, field.cells.length / 2);
      loaded = {
        scope,
        dressing: {
          cache: createDressingCache(field, sizes, next.dressing.fadePx, slots),
          meshes: tiers,
          pool: scope.own(instanceBuffer(root, slots * field.capacity)),
          lodPx: next.dressing.lodPx,
        },
        forest: stand(next.placement.forest),
        understorey: stand(next.placement.understorey),
        sizes,
        backdrop: trees(scope, next.placement.backdrop, false),
        felled,
      };
      felledDirty = true;
      restandForest();
      packStumps(felled, loaded.forest);
    },
    /** Draw only the shrubs and dressing whose foot stands on ground
     *  `ground`'s side has not seen cleared; rebuilds the understorey when
     *  what stands of it changes. (Trees leave by their falls, `setFelled`.) */
    setCleared(ground: GroundMarks | null) {
      if (!loaded) return;
      /** Whether the side has seen the ground at (x, y) cleared. */
      const cleared =
        ground &&
        ((x: number, y: number) => {
          const [i, j] = [Math.floor(x / ground.cellM), Math.floor(y / ground.cellM)];
          return i >= 0 && j >= 0 && i < ground.cols && j < ground.rows && ground.isCleared(i, j);
        });
      const all = loaded.understorey.placed;
      const shrubs = restand(
        loaded.understorey,
        (o) => !!cleared?.(all[o + TREE_FIELD.x], all[o + TREE_FIELD.y]),
      );
      // The dressing reads the ground as the side knows it from now on.
      loaded.dressing.cache.cleared = cleared;
      if (!shrubs) return;
      setDressingCleared(loaded.dressing.cache, cleared);
      viewKey = "";
    },
    /** The trees the side knows have fallen (`BattleFrame.setFelled`). */
    setFelled(next: readonly FelledTree[]) {
      if (next === felledList) return;
      felledList = next;
      felledIds = new Set(next.map((f) => f.prop));
      felledDirty = true;
      if (!loaded) return;
      packStumps(loaded.felled, loaded.forest);
      if (!restandForest()) return;
      // A fall clears ground round it: the dressing is laid again there.
      setDressingCleared(loaded.dressing.cache, loaded.dressing.cache.cleared);
      viewKey = "";
    },
    /** Presentation seconds: a falling tree moves on it. */
    setClock(seconds: number) {
      if (seconds === clock) return;
      clock = seconds;
      if (loaded?.felled.moving) felledDirty = true;
    },
    setTreesShown(on: boolean) {
      treesShown = on;
    },
    /** Lab diagnostics: draw the shrubs under tree lines or not (a paired
     *  cost measure). */
    setUnderstoreyShown(on: boolean) {
      understoreyShown = on;
      viewKey = "";
    },
    setDressingShown(on: boolean) {
      dressingShown = on;
    },
    /** Choose this frame's tiers for `camera` at a viewport `height` pixels
     *  tall, and lay the dressing's next cells while any are waiting. */
    prepare(camera: Camera3DParams, height: number) {
      const key = detailKey(camera, height);
      const moved = key !== viewKey;
      if (loaded && (moved || felledDirty)) {
        setDetailView(view, camera, height);
        view.lodPx = loaded.forest.drawn.lodPx;
        packFelled(loaded.felled, loaded.forest, loaded.sizes);
        felledDirty = false;
      }
      if (!moved && !loaded?.dressing.cache.pending) return;
      viewKey = key;
      if (moved) {
        setDetailView(view, camera, height);
        sunShadow(shadow, sun, camera);
        for (const pop of populations()) {
          view.lodPx = pop.lodPx;
          selectChunks(pop.chunks, view, chunkTier, pop.casts ? view.shadow : null);
          stageNear(pop.chunks, pop.staged, view, tierFor);
          upload(pop);
        }
      }
      if (!loaded) return;
      const { cache, pool, lodPx } = loaded.dressing;
      view.lodPx = lodPx;
      const slotBytes = cache.field.capacity * INSTANCE_FLOATS * 4;
      selectDressing(cache, view, DRESSING_CELLS_PER_VIEW, (slot, records) =>
        registry.device.queue.writeBuffer(root.unwrap(pool), slot * slotBytes, records),
      );
    },
    /** The forest into one cascade (`bound` carries the cascade's camera). */
    encodeShadows(pass: TgpuRenderPass, cameraGroup: CameraGroup) {
      const bound = caster.with(pass).with(cameraGroup) as unknown as Drawable;
      for (const pop of standing()) drawPopulation(pop, bound, true, CASTER_COARSER);
      const drawn = trees();
      if (drawn)
        drawFelled(
          drawn.felled,
          felledCaster.with(pass).with(cameraGroup) as unknown as Drawable,
          bound,
          CASTER_COARSER,
        );
    },
    encodeDepth(pass: TgpuRenderPass, cameraGroup: CameraGroup) {
      const bound = prepass.with(pass).with(cameraGroup) as unknown as Drawable;
      for (const pop of populations()) drawPopulation(pop, bound);
      const drawn = trees();
      if (drawn)
        drawFelled(
          drawn.felled,
          felledPrepass.with(pass).with(cameraGroup) as unknown as Drawable,
          bound,
        );
      const dressed = dressing();
      if (dressed)
        drawDressing(
          dressed,
          dressingPrepass.with(pass).with(cameraGroup) as unknown as Drawable,
          root.unwrap(pass),
        );
    },
    encode(
      pass: TgpuRenderPass,
      cameraGroup: CameraGroup,
      fogFaces: ReturnType<FogVisibility["groups"]>["faces"],
    ) {
      const colour = (pipeline: typeof forestColour) =>
        pipeline
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogFaces) as unknown as Drawable;
      for (const pop of standing()) drawPopulation(pop, colour(forestColour));
      const drawn = trees();
      if (drawn) {
        drawPopulation(drawn.backdrop, colour(backdropColour));
        const felledBound = felledColour
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogFaces) as unknown as Drawable;
        drawFelled(drawn.felled, felledBound, colour(forestColour));
      }
      const dressed = dressing();
      if (dressed) drawDressing(dressed, colour(dressingColour), root.unwrap(pass));
    },
    /** Starts a frame's draw count (the frame calls it before its shadows). */
    beginFrame() {
      lastDraws = draws;
      draws = 0;
    },
    stats(): SceneryStats {
      const felledStats = (felled: Felled | undefined) => {
        let trees = 0,
          stumps = 0;
        for (let r = 2; r < (felled?.treeRuns.length ?? 0); r += 3) trees += felled!.treeRuns[r];
        for (let r = 2; r < (felled?.stumpRuns.length ?? 0); r += 3) stumps += felled!.stumpRuns[r];
        return { trees, falling: felled?.moving ?? false, stumps };
      };
      const population = (pop: Population | undefined) => {
        const tiers = Array.from({ length: TIER_COUNT }, () => 0);
        let placed = 0,
          triangles = 0,
          casters = 0;
        if (pop)
          for (let k = 0; k < pop.chunks.kinds; k++) {
            placed += pop.chunks.sorted[k].length / INSTANCE_FLOATS;
            for (let t = 0; t < TIER_COUNT; t++) {
              let count = pop.staged.counts[k][t];
              const whole = pop.chunks.ranges[k][t];
              for (let r = 1; r < whole.length; r += 2) count += whole[r];
              tiers[t] += count;
              triangles += (count * pop.meshes[k][t].vertices) / 3;
              if (pop.casts && t < TIER_COUNT - 1) casters += pop.staged.counts[k][t];
            }
            for (let r = 1; r < pop.chunks.cast[k].length; r += 2) casters += pop.chunks.cast[k][r];
          }
        return { placed, tiers, triangles, casters };
      };
      const dressing = (drawn: Dressing | undefined) => {
        const out = {
          placed: 0,
          tiers: [0, 0, 0, 0],
          triangles: 0,
          casters: 0,
          cells: 0,
          bytes: 0,
          pending: false,
        };
        if (!drawn) return out;
        const { cache, meshes } = drawn;
        const counts = dressingCounts(cache);
        out.placed = counts.held;
        out.cells = cache.held.filter((cell) => cell !== null).length;
        out.bytes = cache.held.length * cache.field.capacity * INSTANCE_FLOATS * 4;
        out.pending = cache.pending;
        if (!dressingShown) return out;
        out.tiers = counts.tiers;
        cache.drawn.forEach((cells, t) => {
          for (const cell of cells)
            cache.field.kinds.forEach((kind, n) => {
              out.triangles +=
                ((cell.starts[n + 1] - cell.starts[n]) * meshes[kind][t].vertices) / 3;
            });
        });
        return out;
      };
      return {
        kinds: loaded ? loaded.backdrop.meshes.length : 0,
        forest: population(loaded?.forest.drawn),
        understorey: population(loaded?.understorey.drawn),
        backdrop: population(loaded?.backdrop),
        dressing: dressing(loaded?.dressing),
        felled: felledStats(loaded?.felled),
        draws: lastDraws,
      };
    },
  };
}
export type SceneryLayer = Awaited<ReturnType<typeof createSceneryLayer>>;
