// The scenery layer: every placed tree, hedgerow shrub and piece of
// forest-floor dressing, instanced from its appearance's tiers, in the
// frame's own passes. The trees and shrubs are chunked, culled and tiered by
// the static chunk owner (`staticChunks.ts`); the buffers, meshes and
// materials here are the layer's own. Opaque, so all of it is in the depth
// prepass (FogVisibility's tile cull reads it like any surface).
//
// - The forest (the simulation's trunks, one tree each) casts sun shadows
//   into every cascade (the trees in view, and those whose shadow can land in
//   it), receives them, and takes FogTerm through the faces group. A tree is
//   seen or unseen whole: every fragment probes fog at its crown's
//   heart with no facing test, as ground probes do. A crown is a porous
//   volume inside the simulation's foliage, so the sweep's rule (sight into
//   a forest fades with the foliage crossed) decides it, and fog never
//   splits a crown along its sunlit and shaded halves.
// - Scenery past the map is drawn like the backdrop it stands on: lit and
//   hazed, never shadowed, casting nothing, and fogged as a forest tree is:
//   the fog runs on past the playable area.
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
// A tree whose trunk stands on ground the side has seen cleared (a lane a
// vehicle knocked through) is not drawn: `setCleared` rebuilds the forest
// without it, and the dressing is laid again without what stood on that
// ground.
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
import type { GpuRegistry, GpuSlot } from "./registry";
import type { GroundMarks } from "./scarTexture";
import { TREE_FIELD, TREE_FLOATS } from "../scenery/placement";

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
  backdrop: SceneryPopulationStats;
  /** The forest floor's dressing: the pieces laid (`placed`) in `cells`
   *  cells of a pool of `bytes` bytes, the pieces drawn per tier, and
   *  whether wanted cells are still to be laid. */
  dressing: SceneryPopulationStats & { cells: number; bytes: number; pending: boolean };
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
  await Promise.all(
    [prepass, caster, forestColour, backdropColour, dressingPrepass, dressingColour].map(
      (pipeline) => pipeline.initAsync(),
    ),
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
  interface Loaded {
    scope: GpuRegistry;
    dressing: Dressing;
    forest: Population;
    /** The forest's own scope, rebuilt when trees fall. */
    forestScope: GpuRegistry;
    /** Every placed forest tree, and how many of them are drawn. */
    placedForest: Float32Array;
    standing: number;
    kept: number[] | null;
    sizes: ReturnType<typeof kindSize>[];
    backdrop: Population;
  }
  let loaded: Loaded | null = null;
  let viewKey = "";
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

  function upload(pop: Population) {
    for (let k = 0; k < pop.chunks.kinds; k++)
      for (let t = 0; t < TIER_COUNT; t++) {
        const count = pop.staged.counts[k][t];
        if (count === 0) continue;
        const near = pop.near[k][t];
        if (near.capacity < count) {
          near.capacity = Math.max(64, count * 2);
          near.slot.set(instanceBuffer(root, near.capacity));
        }
        near.slot.current!.write(pop.staged.records[k][t].buffer, {
          size: count * INSTANCE_FLOATS * 4,
        } as never);
      }
  }

  /** Anything a bound pipeline can draw instances through. */
  interface Drawable {
    with(layout: typeof vertexLayout, buffer: VertexBuffer): Drawable;
    with(layout: typeof placedLayout, buffer: InstanceBuffer): Drawable;
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

  /** Lab diagnostics: the trees draw and cast, or do neither; the dressing
   *  draws or does not. */
  let treesShown = true;
  let dressingShown = true;
  /** The trees drawn: the forest's, and the backdrop's. */
  const trees = () => (treesShown ? loaded : null);
  /** Every population drawn into the view. */
  const populations = () => (trees() ? [loaded!.forest, loaded!.backdrop] : []);
  const dressing = () => (dressingShown && loaded ? loaded.dressing : null);

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
      const forestScope = scope.scope();
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
        forest: trees(forestScope, next.placement.forest, true),
        forestScope,
        placedForest: next.placement.forest,
        standing: next.placement.forest.length / TREE_FLOATS,
        kept: null,
        sizes,
        backdrop: trees(scope, next.placement.backdrop, false),
      };
    },
    /** Draw only the trees whose trunk stands on ground `ground`'s side has
     *  not seen cleared; rebuilds the forest when that count changes. */
    setCleared(ground: GroundMarks | null) {
      if (!loaded) return;
      const all = loaded.placedForest;
      /** Whether the side has seen the ground at (x, y) cleared. */
      const cleared =
        ground &&
        ((x: number, y: number) => {
          const [i, j] = [Math.floor(x / ground.cellM), Math.floor(y / ground.cellM)];
          return i >= 0 && j >= 0 && i < ground.cols && j < ground.rows && ground.isCleared(i, j);
        });
      const kept: number[] = [];
      for (let o = 0; o < all.length; o += TREE_FLOATS)
        if (!cleared?.(all[o + TREE_FIELD.x], all[o + TREE_FIELD.y])) kept.push(o);
      if (
        kept.length === loaded.standing &&
        (loaded.kept === null || kept.every((o, k) => o === loaded!.kept![k]))
      )
        return;
      const standing = new Float32Array(kept.length * TREE_FLOATS);
      kept.forEach((o, k) => standing.set(all.subarray(o, o + TREE_FLOATS), k * TREE_FLOATS));
      loaded.forestScope.release();
      loaded.forestScope = loaded.scope.scope();
      loaded.forest = population(
        loaded.forestScope,
        treeInstances(standing, loaded.sizes),
        loaded.forest.meshes,
        loaded.forest.lodPx,
        true,
      );
      loaded.standing = kept.length;
      loaded.kept = kept;
      // Ground is cleared where a tree is knocked down, and only there.
      setDressingCleared(loaded.dressing.cache, cleared);
      viewKey = "";
    },
    setTreesShown(on: boolean) {
      treesShown = on;
    },
    setDressingShown(on: boolean) {
      dressingShown = on;
    },
    /** Choose this frame's tiers for `camera` at a viewport `height` pixels
     *  tall, and lay the dressing's next cells while any are waiting. */
    prepare(camera: Camera3DParams, height: number) {
      const key = detailKey(camera, height);
      const moved = key !== viewKey;
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
      const drawn = trees();
      if (drawn) drawPopulation(drawn.forest, bound, true, CASTER_COARSER);
    },
    encodeDepth(pass: TgpuRenderPass, cameraGroup: CameraGroup) {
      const bound = prepass.with(pass).with(cameraGroup) as unknown as Drawable;
      for (const pop of populations()) drawPopulation(pop, bound);
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
      const drawn = trees();
      if (drawn) {
        drawPopulation(drawn.forest, colour(forestColour));
        drawPopulation(drawn.backdrop, colour(backdropColour));
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
        kinds: loaded ? loaded.forest.meshes.length : 0,
        forest: population(loaded?.forest),
        backdrop: population(loaded?.backdrop),
        dressing: dressing(loaded?.dressing),
        draws: lastDraws,
      };
    },
  };
}
export type SceneryLayer = Awaited<ReturnType<typeof createSceneryLayer>>;
