// The scenery layer: every tree, hedgerow shrub and piece of forest-floor
// dressing the presentation's walk chose (`static_world::ground::walk`),
// instanced from its appearance's tiers in the frame's own passes. The walk
// is Rust's: what draws at which tier and what casts arrive as the frame's
// static batches, written only when they change; this layer holds a buffer
// per batch, the meshes and the materials. Opaque, so all of it is in the
// depth prepass (FogVisibility's tile cull reads it like any surface).
//
// - The forest (the simulation's trunks, one tree each) and the shrubs under
//   its tree lines cast sun shadows, receive them, and take FogTerm through
//   the faces group. A tree is seen or unseen whole: every fragment probes
//   fog at its crown's heart with no facing test, as ground probes do. A
//   crown is a porous volume inside the simulation's foliage, so the sweep's
//   rule (sight into a forest fades with the foliage crossed) decides it.
// - Scenery past the map is drawn like the backdrop it stands on: lit and
//   hazed, never shadowed, casting nothing.
// - The forest floor's dressing is drawn as a forest tree is (shadowed,
//   fogged whole at its own heart) and casts nothing; the vertex stage
//   shrinks each piece to nothing as it nears a few pixels. A piece is drawn
//   from both sides: a frond is one sheet.
// - A tree the side knows has fallen is drawn apart, tipped about its
//   stump's top on the clock, then lying, pressed flat, its stump (the
//   presentation's mesh, cut from its trunk) standing where it grew.
//
// Foliage adds leaf clumps per pixel (3D value noise bending the normal and
// darkening the gaps, in the tree's own space so it never swims), fading to
// the plain crown as a pixel grows past a clump.
import { recordDraw, type SubmittedDraws } from "./submitted";
import {
  casterVertex,
  dressingVertex,
  felledAttribs,
  felledLayout,
  felledVertex,
  placedAttribs,
  placedLayout,
  placedVaryings,
  placedVertex,
} from "./sceneryVertex";
import { tgpu, d, std, type TgpuRenderPass } from "typegpu";
import { pcgHash } from "../shaders/pcgHash";
import { VERTEX_FLOATS, type Mesh } from "../mesh";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { WorldScenery } from "../scene";
import { tierMesh } from "../scenery/appearance";
import type { EnvironmentFrame } from "./environmentFrame";
import type { FogVisibility } from "./fogVisibility";
import { fogCoverage, fogTerm } from "./fogTerm";
import { type CameraGroup, vertexBuffer, vertexLayout, type VertexBuffer } from "./geometry";
import { FRAME_MSAA, WORLD_OUT, worldTargets } from "./targets";
import type { GpuRegistry } from "./registry";
import type { WorldAppearance } from "../models/modelLayer";
import {
  Batch,
  BATCH_WORDS,
  CAST_PASS,
  INSTANCE_WORDS,
  NONE,
  VIEW_PASS,
  type frameViews,
} from "../records.gen";

/** What draws a scenery row (`Appearance.draws`). */
const FOREST = 1,
  UNDERSTOREY = 2,
  BACKDROP = 3,
  DRESSING = 4,
  FELLED = 5;
/** A felled tree's stump: one mesh, the presentation's (`stump_mesh`). */
export const STUMP = 6;
const POPULATIONS = ["forest", "understorey", "backdrop", "dressing", "felled", "stumps"] as const;
const TIER_COUNT = 4;

/** The instances a view drew, per population: `kind · 4 + tier`; the
 *  stumps of felled trees per kind alone (a stump has one mesh). */
export interface SceneryCounts {
  forest: number[];
  understorey: number[];
  backdrop: number[];
  dressing: number[];
  felled: number[];
  stumps: number[];
}

/** One population's instances: placed, drawn per tier this frame, the
 *  triangles those draw per view pass, and how many are cast. */
export interface SceneryPopulationStats {
  placed: number;
  tiers: number[];
  triangles: number;
  casters: number;
}
export interface FelledWorkStats {
  /** Cumulative startup envelope rows. */
  envelopeRowsTotal: number;
  /** Cumulative reconciled rows; excludes snapshot equality/replacement work. */
  snapshotRowsTotal: number;
  /** Standing/fallen membership tests in the last selection. */
  membershipChecks: number;
}

export interface SceneryStats {
  /** Bounds tested by the standing populations, including hierarchy branches. */
  visitedBounds: number;
  /** Appearances installed. */
  kinds: number;
  forest: SceneryPopulationStats;
  /** The shrubs under tree lines. */
  understorey: SceneryPopulationStats;
  backdrop: SceneryPopulationStats;
  /** The forest floor's dressing: the pieces drawn (`placed`) from `cells`
   *  cells in `bytes` of records, per tier, and whether wanted cells are
   *  still to be laid. */
  dressing: SceneryPopulationStats & { cells: number; bytes: number; pending: boolean };
  /** The felled trees drawn (falling or lying), whether any still falls,
   *  and their stumps. */
  felled: { trees: number; falling: boolean; stumps: number; work: FelledWorkStats };
  /** Draw calls in the last whole frame, over every pass. */
  draws: number;
}

/** What the presentation says of the scenery it placed and walked
 *  (`scenery_stats`). */
export interface SceneryWalkStats {
  visitedBounds: number;
  felledWork: FelledWorkStats;
  /** Instances placed: the forest's, the tree lines' shrubs', the backdrop's. */
  placed: [number, number, number];
  /** Dressing cells the last walk drew, and whether more are wanted. */
  cells: number;
  pending: boolean;
  falling: boolean;
}

export const NO_SCENERY_WALK: SceneryWalkStats = {
  visitedBounds: 0,
  felledWork: { envelopeRowsTotal: 0, snapshotRowsTotal: 0, membershipChecks: 0 },
  placed: [0, 0, 0],
  cells: 0,
  pending: false,
  falling: false,
};

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** Foliage roughness: leaves scatter; the environment's specular stays small. */
const LEAF_ROUGHNESS = 0.85;
const BARK_ROUGHNESS = 0.9;

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
  /** Scenery past the map: unshadowed, but fogged as a forest tree is (and
   *  so always unseen: nothing is seen past the playable area). */
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
  const prepass = root
    .createRenderPipeline({
      ...base,
      depthStencil: battleWorldDepth("read-write"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("sceneryPrepass");
  const caster = root
    .createRenderPipeline({
      ...base,
      vertex: casterVertex,
      // Both faces cast: a crown's far side must shadow its own underside.
      primitive: { topology: "triangle-list", cullMode: "none" },
      depthStencil: battleWorldDepth("read-write"),
    })
    .$name("sceneryCaster");
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

  /** A row's meshes per tier (one for a stump) and what draws it. */
  interface RowMeshes {
    draws: number;
    kind: number;
    meshes: { buffer: VertexBuffer; vertices: number }[];
  }
  /** One batch the last frame listed: its row, tier and pass, its buffer
   *  (records, or a felled tree's pairs: a power of two of records, in a
   *  scope of its own), and what it draws. */
  interface Held {
    scope: GpuRegistry | null;
    buffer: VertexBuffer | null;
    capacity: number;
    count: number;
    row: number;
    tier: number;
    pass: number;
    cascadeMask: number;
  }
  let scope: GpuRegistry | null = null;
  let given: WorldScenery | null = null;
  let rows: readonly WorldAppearance[] = [];
  /** Per world row: its meshes, when it is scenery the layer was given art for. */
  let meshes: (RowMeshes | null)[] = [];
  let kinds: string[] = [];
  const held = new Map<number, Held>();
  let listed: Held[] = [];
  let walk = NO_SCENERY_WALK;
  let treesShown = true;
  let draws = 0;
  let lastDraws = 0;

  /** The meshes of every scenery row the world names and the art covers. */
  function rebuild() {
    scope?.release();
    scope = registry.scope();
    held.clear();
    listed = [];
    kinds = [];
    meshes = rows.map((row) => {
      if (!row.draws || !given) return null;
      const bundle = given.appearances.get(row.name);
      if (!bundle) return null;
      if (!kinds.includes(row.name)) kinds.push(row.name);
      const own = (mesh: Mesh) => ({
        buffer: scope!.own(vertexBuffer(root, mesh)),
        vertices: mesh.length / VERTEX_FLOATS,
      });
      const tiers = row.stump
        ? [own(row.stump)]
        : Array.from({ length: TIER_COUNT }, (_, t) => own(tierMesh(bundle, t)));
      return { draws: row.draws, kind: kinds.indexOf(row.name), meshes: tiers };
    });
  }

  /** Anything a bound pipeline can draw instances through. */
  interface Drawable {
    with(layout: typeof vertexLayout, buffer: VertexBuffer): Drawable;
    with(layout: typeof placedLayout | typeof felledLayout, buffer: VertexBuffer): Drawable;
    draw(vertices: number, instances: number, firstVertex?: number, firstInstance?: number): void;
  }

  /** Draw the listed batches of `pass` that `which` takes, each through
   *  `bound(draws)` (null: not drawn). */
  function drawListed(
    pass: number,
    bound: (draws: number) => Drawable | null,
    submitted?: SubmittedDraws,
    cascade?: number,
  ) {
    for (const h of listed) {
      const row = meshes[h.row];
      if (h.pass !== pass || !row || !h.buffer) continue;
      if (cascade !== undefined && !(h.cascadeMask & (1 << cascade))) continue;
      if (!treesShown && row.draws !== DRESSING) continue;
      const pipeline = bound(row.draws);
      if (!pipeline) continue;
      const mesh = row.meshes[Math.min(h.tier, row.meshes.length - 1)];
      const layout = row.draws === FELLED ? felledLayout : placedLayout;
      const records = row.draws === FELLED ? h.count * 2 : h.count;
      if (records > h.capacity) continue;
      pipeline.with(vertexLayout, mesh.buffer).with(layout, h.buffer).draw(mesh.vertices, h.count);
      recordDraw(submitted, mesh.vertices, h.count);
      draws++;
    }
  }

  return {
    /** The scenery's art: the appearances its rows name; `null` draws none. */
    set(next: WorldScenery | null) {
      given = next;
      rebuild();
    },
    /** The drawables a new load's batches name. */
    setWorld(world: readonly WorldAppearance[]) {
      if (world === rows) return;
      rows = world;
      rebuild();
    },
    /** The static batches of a presentation frame (null: none) and what it
     *  says of its scenery: the scenery rows' batches are taken, their
     *  records copied where the frame wrote them, and those it no longer
     *  lists dropped (the frame writes one anew when it comes back), so the
     *  same view holds the same buffers however it was reached. True when
     *  what is drawn changed. Read at once: the views last only until the
     *  next call into the module. */
    setFrame(frame: ReturnType<typeof frameViews> | null, stats: SceneryWalkStats): boolean {
      walk = stats;
      let changed = false;
      const next: Held[] = [];
      const keys = new Set<number>();
      const words = frame?.batches.u32;
      for (let b = 0; b < (frame?.batches.count ?? 0); b++) {
        const o = b * BATCH_WORDS;
        const row = words![o + Batch.appearance];
        if (!meshes[row]) continue;
        const key = words![o + Batch.key];
        let h = held.get(key);
        if (!h) {
          h = {
            scope: null,
            buffer: null,
            capacity: 0,
            count: 0,
            row,
            tier: 0,
            pass: 0,
            cascadeMask: 0,
          };
          held.set(key, h);
        }
        h.row = row;
        h.tier = words![o + Batch.tier];
        h.pass = words![o + Batch.pass];
        h.cascadeMask = words![o + Batch.cascade_mask];
        const first = words![o + Batch.first];
        const records = words![o + Batch.count];
        const pairs = meshes[row]!.draws === FELLED;
        if (first !== NONE) {
          const capacity = 2 ** Math.ceil(Math.log2(Math.max(2, records)));
          if (h.capacity !== capacity) {
            h.capacity = capacity;
            h.scope?.release();
            h.scope = scope!.scope();
            const layout = pairs ? felledLayout : placedLayout;
            h.buffer = h.scope.own(
              root
                .createBuffer(layout.schemaForCount(pairs ? capacity / 2 : capacity))
                .$usage("vertex"),
            ) as unknown as VertexBuffer;
          }
          const view = frame!.instances.f32.subarray(
            first * INSTANCE_WORDS,
            (first + records) * INSTANCE_WORDS,
          );
          // TypeGPU copies a view's bytes as it does a buffer's, though its
          // types name only buffers.
          (h.buffer as unknown as { write(data: ArrayBuffer): void }).write(
            view as unknown as ArrayBuffer,
          );
          changed = true;
        }
        // `first === NONE` means the frame did not rewrite this batch. Its
        // count is part of the uploaded record contract too, so keep the
        // previous draw count until a rewritten batch supplies the new one.
        // Updating it here would let a stale, smaller buffer be drawn with a
        // larger instance range and invalidate the whole command buffer.
        if (first !== NONE) h.count = pairs ? records / 2 : records;
        keys.add(key);
        next.push(h);
      }
      for (const [key, h] of held)
        if (!keys.has(key)) {
          h.scope?.release();
          held.delete(key);
          changed = true;
        }
      listed = next;
      return changed;
    },
    setTreesShown(on: boolean) {
      treesShown = on;
    },
    /** The standing scenery and the felled into one cascade. */
    encodeShadows(
      pass: TgpuRenderPass,
      cameraGroup: CameraGroup,
      cascade: number,
      submitted?: SubmittedDraws,
    ) {
      const bound = caster.with(pass).with(cameraGroup) as unknown as Drawable;
      const felled = felledCaster.with(pass).with(cameraGroup) as unknown as Drawable;
      drawListed(CAST_PASS, (draws) => (draws === FELLED ? felled : bound), submitted, cascade);
    },
    encodeDepth(pass: TgpuRenderPass, cameraGroup: CameraGroup, submitted?: SubmittedDraws) {
      const bound = prepass.with(pass).with(cameraGroup) as unknown as Drawable;
      const felled = felledPrepass.with(pass).with(cameraGroup) as unknown as Drawable;
      const dressing = dressingPrepass.with(pass).with(cameraGroup) as unknown as Drawable;
      drawListed(
        VIEW_PASS,
        (draws) => (draws === FELLED ? felled : draws === DRESSING ? dressing : bound),
        submitted,
      );
    },
    encode(
      pass: TgpuRenderPass,
      cameraGroup: CameraGroup,
      fogFaces: ReturnType<FogVisibility["groups"]>["faces"],
      submitted?: SubmittedDraws,
    ) {
      const colour = (pipeline: typeof forestColour) =>
        pipeline
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogFaces) as unknown as Drawable;
      const [forest, backdrop, felled, dressing] = [
        colour(forestColour),
        colour(backdropColour),
        colour(felledColour as unknown as typeof forestColour),
        colour(dressingColour),
      ];
      drawListed(
        VIEW_PASS,
        (draws) =>
          draws === BACKDROP
            ? backdrop
            : draws === FELLED
              ? felled
              : draws === DRESSING
                ? dressing
                : forest,
        submitted,
      );
    },
    /** The instances the last view drew, per population, kind and tier
     *  (`SceneryCounts`); the stumps per kind alone. */
    drawn(): SceneryCounts {
      const zero = () => Array.from({ length: kinds.length * TIER_COUNT }, () => 0);
      const out: SceneryCounts = {
        forest: zero(),
        understorey: zero(),
        backdrop: zero(),
        dressing: zero(),
        felled: zero(),
        stumps: Array.from({ length: kinds.length }, () => 0),
      };
      for (const h of listed) {
        const row = meshes[h.row];
        if (!row || h.pass !== VIEW_PASS) continue;
        if (!treesShown && row.draws !== DRESSING) continue;
        if (row.draws === STUMP) out.stumps[row.kind] += h.count;
        else out[POPULATIONS[row.draws - 1]][row.kind * TIER_COUNT + h.tier] += h.count;
      }
      return out;
    },
    /** Starts a frame's draw count (the frame calls it before its shadows). */
    beginFrame() {
      lastDraws = draws;
      draws = 0;
    },
    stats(): SceneryStats {
      const population = (draws: number, placed: number) => {
        const tiers = [0, 0, 0, 0];
        let triangles = 0,
          casters = 0;
        for (const h of listed) {
          const row = meshes[h.row];
          if (row?.draws !== draws) continue;
          if (h.pass === CAST_PASS) casters += h.count;
          else {
            tiers[h.tier] += h.count;
            triangles += (h.count * row.meshes[h.tier].vertices) / 3;
          }
        }
        return { placed, tiers, triangles, casters };
      };
      const dressing = population(DRESSING, 0);
      dressing.placed = dressing.tiers.reduce((a, b) => a + b, 0);
      const felled = population(FELLED, 0);
      const stumps = population(STUMP, 0);
      return {
        kinds: kinds.length,
        visitedBounds: walk.visitedBounds,
        forest: population(FOREST, walk.placed[0]),
        understorey: population(UNDERSTOREY, walk.placed[1]),
        backdrop: population(BACKDROP, walk.placed[2]),
        dressing: {
          ...dressing,
          cells: walk.cells,
          bytes: dressing.placed * INSTANCE_WORDS * 4,
          pending: walk.pending,
        },
        felled: {
          trees: felled.tiers.reduce((a, b) => a + b, 0),
          falling: walk.falling,
          work: walk.felledWork,
          stumps: stumps.tiers.reduce((a, b) => a + b, 0),
        },
        draws: lastDraws,
      };
    },
  };
}
