// The grass pass: dense 3D grass grown on the GPU whenever the view moves,
// after the Ghost of Tsushima technique (research.md), and drawn as clumps,
// each a grass kind (a scenery appearance the workbench can show).
//
// Build (compute, one workgroup per 4 m world tile in the window the CPU
// fits to the view):
// - cull the tile by the view's side planes and the grass's reach;
// - walk the tile's candidate clumps in rank order. Clump j stands at the
//   j-th point of an R2 low-discrepancy sequence over the tile (jittered), so
//   any first k are spread evenly, and exists where the field's density (clumps per
//   square metre, set so a clump covers about `pixels_per_clump` pixels)
//   exceeds its rank. As density falls with distance a clump shrinks away
//   before it goes, so nothing pops; as the camera moves, a clump never
//   changes place;
// - per clump, the ground under it says what grows (the plot's kind, the
//   verge) or that nothing does (road, forest, water, props), seats it on
//   the simulation's triangle, and gives its colour (the ground's own albedo
//   there, so near grass and the painted ground beyond agree);
// - append it to the near or far tier by its height in pixels. Far clumps'
//   tint and lighting give way to the ground's, so they never speckle.
// Draw: one indexed indirect draw per tier over every kind's canonical blade
// strips, the vertex stage bending each blade in the one wind; the fragment
// stage is lit and sun-shadowed like the ground, and takes FogTerm as the
// ground beneath each fragment, so grass is seen exactly where that ground is.
//
// The side's learned scars (`groundScars`, the terrain's own sample) shape
// the field too: craters, scorch and tracks leave clumps out, scorch and
// tracks colour them, and tracks and trampling lay them over (the clump's
// colour alpha carries how far; the vertex stage leans and sinks it). The
// field regrows when the scars change, as when the view moves.
//
// Grass casts no sun shadow (cost); it receives the cascades.
import { tgpu, d, std, type TgpuBindGroup, type TgpuRenderPass } from "typegpu";
import { pcgHash } from "../shaders/pcgHash";
import { vec3, type Mat4 } from "math";
import { frustum } from "math/shapes";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { eyePosition } from "@packages/renderer-core/src/camera3d";
import {
  GRASS_SEGMENTS,
  grassBladeVertices,
  grassStripIndices,
} from "@packages/scene-assets/src/grass.ts";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { TerrainSurface } from "../terrain/terrainSurface";
import {
  createGrassWindow,
  FIELD_LODS,
  GRASS_GROWTH_ROWS,
  GRASS_MAX_KINDS,
  GRASS_TILE_M,
  grassKinds,
  grassSides,
  grassWindow,
  metresPerPixel,
  packGrassShapes,
  type GrassAppearances,
  GRASS_EDGE_M,
} from "../terrain/grassField";
import type { GrassRules } from "../terrain/biome";
import {
  fogCoverage,
  fogIsGround,
  fogTerm,
  grassPaintGlow,
  groundPaint,
  groundPaintAtPixel,
} from "./fogTerm";
import {
  forestVergeInside,
  groundColour,
  groundScars,
  groundCell,
  groundShore,
  groundSite,
  groundVerge,
  groundWater,
  scarredSurface,
  terrainLayout,
  valueNoise,
} from "./terrainMaterial";
import type { TerrainSource } from "./terrainMaterial";
import { triangleRuleHeight } from "./triangleRule";
import { terrainSample, type TerrainHeights } from "./terrainHeights";
import type { EnvironmentFrame } from "./environmentFrame";
import type { CameraGroup } from "./geometry";
import { FRAME_MSAA, WORLD_OUT, worldTargets } from "./targets";
import type { GpuRegistry, GpuSlot } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

const GrassParams = d
  .struct({
    /** The view's side planes: inward normal, offset. */
    planes: d.arrayOf(d.vec4f, 4),
    /** The eye, and metres one pixel spans per metre of distance. */
    eye: d.vec4f,
    /** Tile (0, 0)'s corner, the tile's side, the tallest clump. */
    window: d.vec4f,
    /** Tiles across and up; the near and far tiers' capacities. */
    grid: d.vec4u,
    /** Pixels per clump, the most clumps a square metre, the fade's footprints. */
    density: d.vec4f,
    /** Near tier's height in pixels, a blade's least width in pixels, 0, 0. */
    tiers: d.vec4f,
    /** Bare margins: road, prop, forest and water; 0. */
    clear: d.vec4f,
    /** The height grid's size, the prop count, 0. */
    counts: d.vec4u,
    /** The height grid's spacing, then 0. */
    ground: d.vec4f,
    /** Wind heading (unit), steady lean, gust lean. */
    wind: d.vec4f,
    /** 1 / gust spacing, gust speed, flutter, flutter radians a second. */
    gusts: d.vec4f,
    /** Per plot kind (the verge last): density, height scale, kind, 0. */
    growth: d.arrayOf(d.vec4f, GRASS_GROWTH_ROWS),
    /** Per kind: height, blades, a blade's root width, 0. */
    kinds: d.arrayOf(d.vec4f, GRASS_MAX_KINDS),
    /** Per kind: the near and far tiers' first shape vertex. */
    bases: d.arrayOf(d.vec4u, GRASS_MAX_KINDS),
  })
  .$name("GrassParams");

/** One clump as the build leaves it for the draw. */
const Clump = d
  .struct({
    root: d.vec3f,
    height: d.f32,
    /** The ground's albedo under it, sqrt-encoded rgb8. */
    colour: d.u32,
    kind: d.u32,
    yaw: d.f32,
    /** Blade width multiplier: far blades are held to a least pixel width. */
    width: d.f32,
  })
  .$name("GrassClump");
const CLUMP_BYTES = 32;

const ShapeVertex = d
  .struct({ spine: d.vec4f, side: d.vec4f, normal: d.vec4f, tint: d.vec4f })
  .$name("GrassShapeVertex");

/** Two indexed-indirect draws' arguments (5 words each), then each tier's
 *  raw count before clamping to capacity. */
const ARGS_WORDS = 12;

export const grassBuildLayout = tgpu.bindGroupLayout({
  params: { uniform: GrassParams, visibility: ["compute"] },
  heights: {
    storage: (n: number) => d.arrayOf(d.u32, n),
    access: "readonly",
    visibility: ["compute"],
  },
  /** Per prop: (x, y, cos yaw, sin yaw), (half extents with the margin, 0, 0). */
  props: {
    storage: (n: number) => d.arrayOf(d.vec4f, n),
    access: "readonly",
    visibility: ["compute"],
  },
  clumps: {
    storage: (n: number) => d.arrayOf(Clump, n),
    access: "mutable",
    visibility: ["compute"],
  },
  args: {
    storage: d.arrayOf(d.atomic(d.u32), ARGS_WORDS),
    access: "mutable",
    visibility: ["compute"],
  },
});

export const grassDrawLayout = tgpu.bindGroupLayout({
  params: { uniform: GrassParams, visibility: ["vertex"] },
  /** Tier, its first clump, vertices per blade, 0. */
  tier: { uniform: d.vec4u, visibility: ["vertex"] },
  clumps: {
    storage: (n: number) => d.arrayOf(Clump, n),
    access: "readonly",
    visibility: ["vertex"],
  },
  shapes: {
    storage: (n: number) => d.arrayOf(ShapeVertex, n),
    access: "readonly",
    visibility: ["vertex"],
  },
});

const BUILD_WORKGROUP = 64;
/** A clump laid flat (tracks, trampling) leans this far over, as a fraction
 *  of its height, and sinks by this much of it. */
const FLAT_LEAN = 1.1;
const FLAT_SINK = 0.6;
/** The length scale of patches of taller and lower grass. */
const GRASS_PATCH_M = 7;
/** How far a clump strays from its sequence point, either way. */
const GRASS_JITTER_M = 0.4;
/** R2's generator: the plastic number's reciprocals. */
const R2 = [0.7548776662466927, 0.5698402909980532] as const;

const grassSample = terrainSample(
  tgpu
    .fn(
      [d.u32],
      d.u32,
    )(/* wgsl */ `(at: u32) -> u32 {
  return grassBuildLayout.$.heights[at];
}`)
    .$uses({ grassBuildLayout }),
);

/** Ground height at `q`: the simulation's triangle rule over the grid. */
const grassGround = tgpu
  .fn(
    [d.vec2f],
    d.f32,
  )(/* wgsl */ `(q: vec2f) -> f32 {
  let P = grassBuildLayout.$.params;
  let nx = P.counts.x;
  let ny = P.counts.y;
  let fx = clamp(q.x / P.ground.x, 0.0, f32(nx - 1u));
  let fy = clamp(q.y / P.ground.x, 0.0, f32(ny - 1u));
  let i = min(u32(fx), nx - 2u);
  let j = min(u32(fy), ny - 2u);
  return triangleRuleHeight(
    grassSample(i, j), grassSample(i + 1u, j),
    grassSample(i, j + 1u), grassSample(i + 1u, j + 1u),
    fx - f32(i), fy - f32(j));
}`)
  .$uses({ grassBuildLayout, triangleRuleHeight, grassSample });

/** Clumps a square metre holds `dist` metres from the eye and `drop` below it:
 *  about one per `pixels_per_clump` pixels of that ground on screen. */
const grassDensity = tgpu
  .fn(
    [d.f32, d.f32],
    d.f32,
  )(/* wgsl */ `(dist: f32, drop: f32) -> f32 {
  let P = grassBuildLayout.$.params;
  let facing = clamp(drop / dist, 0.15, 1.0);
  let footprint = P.eye.w * dist;
  return min(P.density.y, facing / (P.density.x * footprint * footprint));
}`)
  .$uses({ grassBuildLayout });

/** Whether `p` lies on a prop's footprint (widened by its margin). */
const grassUnderProp = tgpu
  .fn(
    [d.vec2f],
    d.bool,
  )(/* wgsl */ `(p: vec2f) -> bool {
  for (var i = 0u; i < grassBuildLayout.$.params.counts.z; i++) {
    let a = grassBuildLayout.$.props[2u * i];
    let b = grassBuildLayout.$.props[2u * i + 1u];
    let o = p - a.xy;
    if (abs(dot(o, a.zw)) < b.x && abs(dot(o, vec2f(-a.w, a.z))) < b.y) { return true; }
  }
  return false;
}`)
  .$uses({ grassBuildLayout });

/** Three hashes in [0, 1) of two integers: PCG over a stream seeded by both. */
const grassHash = tgpu
  .fn(
    [d.u32, d.u32],
    d.vec3f,
  )(/* wgsl */ `(a: u32, b: u32) -> vec3f {
  var s = a * 747796405u + b * 2891336453u + 12345u;
  var out = vec3f(0.0);
  for (var k = 0u; k < 3u; k++) {
    out[k] = f32(pcgHash(s) >> 8u) * (1.0 / 16777216.0);
    s = s * 747796405u + 2891336453u;
  }
  return out;
}`)
  .$uses({ pcgHash });

const buildFn = tgpu
  .computeFn({
    in: { wg: d.builtin.workgroupId, li: d.builtin.localInvocationIndex },
    workgroupSize: [BUILD_WORKGROUP],
  })(/* wgsl */ `{
  let P = grassBuildLayout.$.params;
  if (wg.x >= P.grid.x || wg.y >= P.grid.y) { return; }
  let T = P.window.z;
  let lo = P.window.xy + vec2f(f32(wg.x), f32(wg.y)) * T;
  let hi = lo + vec2f(T);
  // The tile's box: its ground at the corners and centre, a metre either
  // way, and the tallest clump above.
  var zlo = 1e9;
  var zhi = -1e9;
  for (var k = 0u; k < 5u; k++) {
    var q = (lo + hi) * 0.5;
    if (k < 4u) { q = vec2f(select(lo.x, hi.x, (k & 1u) == 1u), select(lo.y, hi.y, k >= 2u)); }
    let z = grassGround(q);
    zlo = min(zlo, z);
    zhi = max(zhi, z);
  }
  zlo -= 1.0;
  zhi += 1.0 + P.window.w;
  let lo2 = lo - vec2f(${GRASS_JITTER_M});
  let hi2 = hi + vec2f(${GRASS_JITTER_M});
  for (var i = 0u; i < 4u; i++) {
    let pl = P.planes[i];
    let pv = vec3f(select(lo2.x, hi2.x, pl.x >= 0.0), select(lo2.y, hi2.y, pl.y >= 0.0), select(zlo, zhi, pl.z >= 0.0));
    if (dot(pl.xyz, pv) + pl.w < 0.0) { return; }
  }
  let eye = P.eye.xyz;
  let nearest = clamp(eye, vec3f(lo2, zlo), vec3f(hi2, zhi));
  let dNear = max(distance(eye, nearest), 0.5);
  if (P.eye.w * dNear >= P.density.w) { return; }
  let n = u32(ceil(T * T * grassDensity(dNear, max(eye.z - zlo, 0.0))));
  // The tile's own offset of the sequence, from its place on the world grid.
  let cell = vec2i(floor(lo / T + 0.5));
  let offset = grassHash(bitcast<u32>(cell.x), bitcast<u32>(cell.y)).xy;
  for (var j = li; j < n; j += ${BUILD_WORKGROUP}u) {
    // The sequence's point, jittered by a fixed amount so a sparse field
    // shows no lattice; both depend on the tile and j alone.
    let h = grassHash(bitcast<u32>(cell.x) ^ (j * 2654435761u), bitcast<u32>(cell.y) + j);
    let jitter = (vec2f(fract(h.z * 7.13), fract(h.x * 5.31)) - 0.5) * ${2 * GRASS_JITTER_M};
    let p = lo + T * fract(offset + f32(j + 1u) * vec2f(${R2[0]}, ${R2[1]})) + jitter;
    let z = grassGround(p);
    let root = vec3f(p, z);
    let dist = max(distance(eye, root), 0.5);
    let footprint = P.eye.w * dist;
    if (footprint >= P.density.w) { continue; }
    // Clump j stands once the density passes (j + 1/2) per tile.
    let rank = (f32(j) + 0.5) / (T * T);
    let rho = grassDensity(dist, max(eye.z - z, 0.0));
    if (rank >= rho) { continue; }
    let cell = groundCell(p, footprint);
    let site = groundSite(p, cell);
    let water = groundWater(p, cell);
    // Bare within the margins; thinner and lower for a metre beyond them, so
    // a field meets a road or a wood without a wall of blades.
    // A wood's edge is its rect or its floor's ragged verge, whichever lies
    // farther out.
    let wood = max(site.w, forestVergeInside(p, site.w));
    let margin = min(-site.z - P.clear.x, min(-wood, -water) - P.clear.z);
    if (margin < 0.0) { continue; }
    // Bare on the wet banks round water.
    if (groundShore(water) > 0.35) { continue; }
    let edge = smoothstep(0.0, ${GRASS_EDGE_M}, margin);
    let kindOfPlot = u32(terrainLayout.$.plots[i32(site.x)].detail.y);
    var g = P.growth[min(kindOfPlot, ${GRASS_GROWTH_ROWS - 2}u)];
    if (groundVerge(site, footprint) > 0.5) { g = P.growth[${GRASS_GROWTH_ROWS - 1}u]; }
    // The side's learned scars: craters, scorch and tracks leave clumps out.
    let scar = groundScars(p, footprint);
    let S = terrainLayout.$.scarParams.grass;
    let bare = max(max(scar.weights.x, scar.weights.y), scar.weights.z * S.y) * S.x;
    let keep = rho * g.x * mix(0.5, 1.0, edge) * (1.0 - bare);
    if (rank >= keep) { continue; }
    if (grassUnderProp(p)) { continue; }
    // A clump nearing its rank's threshold is small: it grows in as the
    // density passes it, and shrinks away as the fade takes it.
    let grow = 1.0 - smoothstep(0.7 * keep, keep, rank);
    let fade = 1.0 - smoothstep(P.density.z, P.density.w, footprint);
    let kind = u32(g.z);
    let row = P.kinds[kind];
    // Patches a few metres across stand taller or lower, as uneven grass does.
    let tall = valueNoise(p * ${1 / GRASS_PATCH_M});
    let height = row.x * g.y * grow * fade * mix(0.8, 1.2, h.x) * mix(0.7, 1.2, tall) * mix(0.35, 1.0, edge);
    if (height < 0.02) { continue; }
    let centre = root + vec3f(0.0, 0.0, height * 0.5);
    let radius = height * 0.8 + 0.4;
    var shown = true;
    for (var i = 0u; i < 4u; i++) {
      let pl = P.planes[i];
      if (dot(pl.xyz, centre) + pl.w < -radius) { shown = false; }
    }
    if (!shown) { continue; }
    var tier = 1u;
    if (height / footprint > P.tiers.x * mix(0.85, 1.15, h.y)) { tier = 0u; }
    let width = max(1.0, P.tiers.y * footprint / max(row.z, 1e-4));
    let colour = scarredSurface(groundColour(p, footprint, site, water), scar).xyz;
    // Tracks and trampling lay the clump over (carried in the colour's alpha).
    let flat = max(scar.weights.z, scar.weights.w) * S.z;
    let slot = atomicAdd(&grassBuildLayout.$.args[tier * 5u + 1u], 1u);
    let cap = select(P.grid.w, P.grid.z, tier == 0u);
    if (slot >= cap) { continue; }
    let base = select(P.grid.z, 0u, tier == 0u);
    grassBuildLayout.$.clumps[base + slot] = GrassClump(
      root, height, pack4x8unorm(vec4f(sqrt(max(colour, vec3f(0.0))), 1.0 - flat)), kind, h.z * 6.2831853, width);
  }
}`)
  .$uses({
    grassBuildLayout,
    terrainLayout,
    grassGround,
    grassDensity,
    grassUnderProp,
    grassHash,
    groundCell,
    groundSite,
    groundWater,
    groundShore,
    groundVerge,
    groundColour,
    forestVergeInside,
    groundScars,
    scarredSurface,
    valueNoise,
    Clump,
  });

/** Clamp each tier's count to its capacity, keeping the raw count. */
const finishFn = tgpu
  .computeFn({ workgroupSize: [1] })(/* wgsl */ `{
  let P = grassBuildLayout.$.params;
  for (var t = 0u; t < 2u; t++) {
    let raw = atomicLoad(&grassBuildLayout.$.args[t * 5u + 1u]);
    atomicStore(&grassBuildLayout.$.args[10u + t], raw);
    atomicStore(&grassBuildLayout.$.args[t * 5u + 1u], min(raw, select(P.grid.w, P.grid.z, t == 0u)));
  }
}`)
  .$uses({ grassBuildLayout });

const GrassVertex = d
  .struct({
    clip: d.vec4f,
    world: d.vec3f,
    root: d.vec3f,
    normal: d.vec3f,
    albedo: d.vec3f,
    plain: d.f32,
  })
  .$name("GrassVertex");

/** One blade vertex of clump `iid` of the bound tier: placed, scaled, turned,
 *  and bent in the wind by the square of its height fraction. */
const grassVertexOf = tgpu
  .fn(
    [d.u32, d.u32],
    GrassVertex,
  )(/* wgsl */ `(vid: u32, iid: u32) -> GrassVertex {
  let P = grassDrawLayout.$.params;
  let tier = grassDrawLayout.$.tier;
  let c = grassDrawLayout.$.clumps[tier.y + iid];
  let row = P.kinds[c.kind];
  var out: GrassVertex;
  if (vid / tier.z >= u32(row.y)) {
    // A blade this kind does not have: outside the depth range, clipped.
    out.clip = vec4f(0.0, 0.0, 2.0, 1.0);
    return out;
  }
  let base = select(P.bases[c.kind].y, P.bases[c.kind].x, tier.x == 0u);
  let s = grassDrawLayout.$.shapes[base + vid];
  let scale = c.height / max(row.x, 1e-4);
  let cs = cos(c.yaw);
  let sn = sin(c.yaw);
  let sp = s.spine.xyz * scale;
  let sd = s.side.xyz * scale * c.width;
  let spine = vec3f(sp.x * cs - sp.y * sn, sp.x * sn + sp.y * cs, sp.z);
  let side = vec3f(sd.x * cs - sd.y * sn, sd.x * sn + sd.y * cs, sd.z);
  // The one wind: a steady lean, gust fronts rolling downwind, a flutter.
  let t = typegpuCameraLayout.$.cam.time;
  let dir = P.wind.xy;
  let along = dot(c.root.xy, dir);
  let front = sin(6.2831853 * (along - t * P.gusts.y) * P.gusts.x);
  let gust = smoothstep(0.2, 1.0, front);
  let flutter = P.gusts.z * sin(P.gusts.w * t + s.side.w * 6.2831853 + along * 0.35);
  let bend = s.spine.w * s.spine.w;
  let packed = unpack4x8unorm(c.colour);
  // Laid over by tracks or trampling: the clump sinks and its blades lean
  // out its own way, still in the wind but less.
  let flat = 1.0 - packed.w;
  let lean = (P.wind.z + P.wind.w * gust + flutter) * (1.0 - flat);
  let push = dir * lean + vec2f(-dir.y, dir.x) * flutter * 0.5 * (1.0 - flat);
  let laid = vec2f(cs, sn) * flat * ${FLAT_LEAN};
  var world = c.root + vec3f(spine.xy, spine.z * (1.0 - flat * ${FLAT_SINK})) + side + vec3f((push + laid) * c.height * bend, 0.0);
  world.z -= 0.5 * dot(push, push) * c.height * bend;
  let n = s.normal.xyz;
  let colour = packed.xyz;
  out.clip = typegpuCameraLayout.$.cam.viewProj * vec4f(world, 1.0);
  // Blades cross each other: where two meet at (nearly) one depth the winner
  // is left to the hardware and changes frame to frame. A per-blade nudge of
  // up to a two-thousandth of the depth (5 cm at 100 m, a few millimetres
  // near the eye) separates every such pair the same way each frame.
  let tie = ((bitcast<u32>(c.root.x) * 2654435761u) ^ (bitcast<u32>(c.root.y) * 2246822519u)) + (vid / tier.z) * 374761393u;
  out.clip.z *= 1.0 + 1e-3 * (f32(tie >> 8u) / 16777216.0 - 0.5);
  out.world = world;
  out.root = c.root;
  out.normal = vec3f(n.x * cs - n.y * sn, n.x * sn + n.y * cs, n.z);
  // Far clumps are a few pixels: their dark roots and gaps would read as
  // speckle, so their tint and their lighting give way to the ground's.
  let footprint = P.eye.w * distance(typegpuCameraLayout.$.cam.eye, c.root);
  let plain = smoothstep(0.6 * P.density.z, 1.1 * P.density.z, footprint);
  out.albedo = colour * colour * mix(s.tint.xyz, vec3f(1.0), plain);
  out.plain = plain;
  return out;
}`)
  .$uses({ grassDrawLayout, typegpuCameraLayout, GrassVertex });

const grassVertex = tgpu.vertexFn({
  in: { vid: d.builtin.vertexIndex, iid: d.builtin.instanceIndex },
  out: {
    clip: d.builtin.position,
    world: d.vec3f,
    root: d.vec3f,
    normal: d.vec3f,
    albedo: d.vec3f,
    plain: d.f32,
  },
})((v) => {
  "use gpu";
  const g = grassVertexOf(v.vid, v.iid);
  return {
    clip: g.clip,
    world: g.world,
    root: g.root,
    normal: g.normal,
    albedo: g.albedo,
    plain: g.plain,
  };
});

/** How far a blade's shading normal leans to the ground's: mostly, so the
 *  field lights like the ground it grows on and never glitters. */
const GROUND_NORMAL_WEIGHT = 0.55;
const BLADE_ROUGHNESS = 0.9;

export interface GrassStats {
  /** Whether grass is drawn: the terrain grows it and its kinds are installed. */
  enabled: boolean;
  /** World tiles the last frame's window held. */
  tiles: number;
  capacity: [number, number];
  clumpBytes: number;
  kinds: string[];
}

export interface GrassCounts {
  /** Clumps drawn per tier, and what the build found before capacity. */
  near: number;
  far: number;
  nearFound: number;
  farFound: number;
}

/** One clump as the probe reads it back. */
export interface GrassClumpRow {
  root: [number, number, number];
  height: number;
  kind: string;
  tier: 0 | 1;
  /** How far tracks or trampling laid it over, 0 upright to 1 flat. */
  laid: number;
}

export async function createGrassPass(
  root: Root,
  registry: GpuRegistry,
  environment: EnvironmentFrame,
  terrain: TerrainSource,
  terrainHeights: TerrainHeights,
) {
  const device = registry.device;
  const fragment = tgpu.fragmentFn({
    in: {
      clip: d.builtin.position,
      world: d.vec3f,
      root: d.vec3f,
      normal: d.vec3f,
      albedo: d.vec3f,
      plain: d.f32,
    },
    out: WORLD_OUT,
  })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const up = d.vec3f(0, 0, 1);
    // Fog as the ground under each blade fragment, at the root's height: the
    // sight edge cuts through a clump as sharply as through the ground, and
    // a blade is seen exactly where the ground beneath it is.
    const seen = fogTerm(d.vec3f(v.world.x, v.world.y, v.root.z), up, v.clip.xy, fogIsGround());
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    // Far clumps light as the ground does, so they never speckle.
    const toGround = std.mix(GROUND_NORMAL_WEIGHT, 1, v.plain);
    const shading = std.normalize(std.mix(n, up, toGround));
    const sun = environment.sampleSunShadow(v.world, up, v.clip.xy);
    // The ground paint is a light. The paint on the ground under this bit of
    // blade lights it from below, only its bottom few centimetres (the
    // falloff), so no fringe of coloured blades rises out of a stroke. Where
    // the blade stands over the stroke as drawn (its own pixel), it carries
    // the stroke's glow, so the line runs on unbroken through the grass
    // without spreading past its edges.
    const under = groundPaint(d.vec3f(v.world.x, v.world.y, v.root.z));
    const here = groundPaintAtPixel(v.clip.xy);
    const lit = environment.shade(
      v.albedo,
      d.vec3f(0),
      BLADE_ROUGHNESS,
      0,
      0,
      1,
      shading,
      v.world,
      sun,
      eye,
    );
    // Bound with fog's ground group, so the mask marks blades as ground: the
    // soft edge and the rim cut through grass as through the ground under it.
    return {
      color: d.vec4f(
        std.add(
          lit.xyz,
          std.max(grassPaintGlow(under, v.world.z - v.root.z), grassPaintGlow(here, 0)),
        ),
        1,
      ),
      fog: fogCoverage(seen, 1),
    };
  });

  const drawPipeline = root.createRenderPipeline({
    vertex: grassVertex,
    fragment,
    targets: worldTargets(),
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  const build = root.createComputePipeline({ compute: buildFn });
  const finish = root.createComputePipeline({ compute: finishFn });
  await Promise.all([drawPipeline.initAsync(), build.initAsync(), finish.initAsync()]);

  const { STORAGE, COPY_DST, COPY_SRC, INDIRECT, INDEX, MAP_READ } = GPUBufferUsage;
  const params = registry.own(root.createBuffer(GrassParams).$usage("uniform"));
  const tierUniforms = FIELD_LODS.map(() =>
    registry.own(root.createBuffer(d.vec4u).$usage("uniform")),
  );
  const args = registry.buffer({
    label: "grass-args",
    size: ARGS_WORDS * 4,
    usage: STORAGE | INDIRECT | COPY_DST | COPY_SRC,
  });
  const storage = (label: string, bytes: number, extra = 0) =>
    device.createBuffer({ label, size: Math.max(16, bytes), usage: STORAGE | COPY_DST | extra });
  const slots = {
    props: registry.slot<GPUBuffer>(),
    clumps: registry.slot<GPUBuffer>(),
    shapes: registry.slot<GPUBuffer>(),
    indices: [registry.slot<GPUBuffer>(), registry.slot<GPUBuffer>()] as GpuSlot<GPUBuffer>[],
  };

  let surface: TerrainSurface | null = null;
  let appearances: GrassAppearances | null = null;
  let rules: GrassRules | null = null;
  let kindNames: string[] = [];
  let capacity: [number, number] = [0, 0];
  let indexCounts: [number, number] = [0, 0];
  let ready = false;
  let lowest = 0;
  let tiles = 0;
  let buildGroup: ReturnType<typeof makeBuildGroup> | null = null;
  let drawGroups: ReturnType<typeof makeDrawGroup>[] = [];
  const window = createGrassWindow();
  const sides = frustum.create();
  const eye = vec3.create();
  let frameParams: d.Infer<typeof GrassParams> | null = null;

  function makeBuildGroup() {
    return root.createBindGroup(grassBuildLayout, {
      params,
      heights: terrainHeights.forGrid(surface!.grid!),
      props: slots.props.current!,
      clumps: slots.clumps.current!,
      args,
    });
  }
  function makeDrawGroup(t: number) {
    return root.createBindGroup(grassDrawLayout, {
      params,
      tier: tierUniforms[t],
      clumps: slots.clumps.current!,
      shapes: slots.shapes.current!,
    });
  }

  /** Rebuild every table from the terrain and the installed kinds. */
  function rebuild() {
    ready = false;
    const grid = surface?.grid;
    if (!surface || !grid || !appearances) return;
    const kinds = grassKinds(surface.biome, appearances);
    if (!kinds) return;
    rules = surface.biome.grass;
    const packed = packGrassShapes(kinds);
    kindNames = kinds.appearances.map((a) => a.name);
    lowest = grid.minHeight;
    const footprints = surface.site.footprints;
    const propCount = footprints.length / 5;
    const props = new Float32Array(Math.max(1, propCount) * 8);
    const margin = rules.clear_m.prop;
    for (let i = 0; i < propCount; i++) {
      const [x, y, yaw, hx, hy] = footprints.subarray(i * 5, i * 5 + 5);
      props.set([x, y, Math.cos(yaw), Math.sin(yaw), hx + margin, hy + margin, 0, 0], i * 8);
    }
    slots.props.set(storage("grass-props", props.byteLength));
    device.queue.writeBuffer(slots.props.current!, 0, props);
    capacity = [rules.capacity[0], rules.capacity[1]];
    slots.clumps.set(storage("grass-clumps", (capacity[0] + capacity[1]) * CLUMP_BYTES, COPY_SRC));
    slots.shapes.set(storage("grass-shapes", packed.shapes.byteLength));
    device.queue.writeBuffer(slots.shapes.current!, 0, packed.shapes);
    FIELD_LODS.forEach((lod, t) => {
      const segments = GRASS_SEGMENTS[lod];
      const indices = grassStripIndices(packed.maxBlades, segments);
      indexCounts[t] = indices.length;
      const buffer = device.createBuffer({
        label: `grass-indices-${t}`,
        size: Math.ceil(indices.byteLength / 4) * 4,
        usage: INDEX | COPY_DST,
      });
      slots.indices[t].set(buffer);
      const padded = new Uint16Array(buffer.size / 2);
      padded.set(indices);
      device.queue.writeBuffer(buffer, 0, padded);
      tierUniforms[t].write(d.vec4u(t, t === 0 ? 0 : capacity[0], grassBladeVertices(segments), 0));
    });
    const tallest = kinds.appearances.reduce((m, a) => {
      const scales = Object.values(rules!.growth)
        .filter((g) => g.appearance === a.name)
        .map((g) => g.height);
      return Math.max(m, a.bundle.bounds.max[2] * Math.max(...scales) * 1.2);
    }, 0);
    const wind = rules.wind;
    const heading = (wind.heading_deg * Math.PI) / 180;
    frameParams = {
      planes: [d.vec4f(), d.vec4f(), d.vec4f(), d.vec4f()],
      eye: d.vec4f(),
      window: d.vec4f(0, 0, GRASS_TILE_M, tallest),
      grid: d.vec4u(0, 0, capacity[0], capacity[1]),
      density: d.vec4f(
        rules.pixels_per_clump,
        rules.max_clumps_m2,
        rules.fade_m_per_px[0],
        rules.fade_m_per_px[1],
      ),
      tiers: d.vec4f(rules.near_tier_px, rules.min_blade_px, 0, 0),
      clear: d.vec4f(rules.clear_m.road, rules.clear_m.prop, rules.clear_m.area, 0),
      counts: d.vec4u(grid.nx, grid.ny, propCount, 0),
      ground: d.vec4f(grid.spacing, 0, 0, 0),
      wind: d.vec4f(Math.cos(heading), Math.sin(heading), wind.lean, wind.gust),
      gusts: d.vec4f(1 / wind.gust_m, wind.gust_mps, wind.flutter, wind.flutter_hz * 2 * Math.PI),
      growth: Array.from({ length: GRASS_GROWTH_ROWS }, (_, k) =>
        d.vec4f(
          ...(Array.from(kinds.growth.subarray(k * 4, k * 4 + 4)) as [
            number,
            number,
            number,
            number,
          ]),
        ),
      ),
      kinds: Array.from({ length: GRASS_MAX_KINDS }, (_, k) =>
        d.vec4f(
          ...(Array.from(packed.rows.subarray(k * 4, k * 4 + 4)) as [
            number,
            number,
            number,
            number,
          ]),
        ),
      ),
      bases: Array.from({ length: GRASS_MAX_KINDS }, (_, k) =>
        d.vec4u(
          ...(Array.from(packed.bases.subarray(k * 4, k * 4 + 4)) as [
            number,
            number,
            number,
            number,
          ]),
        ),
      ),
    };
    buildGroup = makeBuildGroup();
    drawGroups = FIELD_LODS.map((_, t) => makeDrawGroup(t));
    grownFor.fill(NaN);
    ready = true;
  }

  const resetArgs = new Uint32Array(ARGS_WORDS);
  /** The view the clumps were last grown for (view-projection, pixel scale):
   *  a frame at the same view draws them again without regrowing, so a still
   *  camera draws the very same clumps in the same order. */
  const grownFor = new Float32Array(17).fill(NaN);
  let drawn = false;
  let regrow = false;
  let suppressed = false;

  return {
    /** The terrain the grass grows on, and the grass kinds; null kinds grow none. */
    setWorld(terrainSurface: TerrainSurface, kinds: GrassAppearances | null) {
      surface = terrainSurface;
      appearances = kinds;
      rebuild();
    },
    /** Fit this frame's window and write its parameters. */
    prepare(camera: Camera3DParams, viewProj: Mat4, heightPx: number) {
      drawn = false;
      regrow = false;
      if (!ready || suppressed || !frameParams || !rules) return;
      eyePosition(eye, camera);
      const scale = metresPerPixel(camera.fovY, heightPx);
      grassWindow(window, eye, lowest, scale, rules);
      tiles = window.tilesX * window.tilesY;
      if (!tiles) return;
      drawn = true;
      let same = grownFor[16] === Math.fround(scale);
      for (let i = 0; i < 16 && same; i++) same = grownFor[i] === Math.fround(viewProj[i]);
      if (same) return;
      grownFor.set(viewProj);
      grownFor[16] = scale;
      regrow = true;
      grassSides(sides, viewProj);
      // The side planes are the frustum's first four.
      frameParams.planes = [0, 1, 2, 3].map((i) => {
        const p = sides[i];
        return d.vec4f(p.normal[0], p.normal[1], p.normal[2], p.constant);
      });
      frameParams.eye = d.vec4f(eye[0], eye[1], eye[2], scale);
      frameParams.window = d.vec4f(window.x0, window.y0, GRASS_TILE_M, frameParams.window.w);
      frameParams.grid = d.vec4u(window.tilesX, window.tilesY, capacity[0], capacity[1]);
      params.write(frameParams);
      resetArgs.fill(0);
      resetArgs[0] = indexCounts[0];
      resetArgs[5] = indexCounts[1];
      device.queue.writeBuffer(args, 0, resetArgs);
    },
    /** Grow the clumps again next frame, though the view has not moved (the
     *  scars under them changed). */
    regrow() {
      grownFor.fill(NaN);
    },
    /** The exact window sampled while this frame's clumps are rebuilt. */
    scarBounds(): readonly [number, number, number, number] | undefined {
      return regrow
        ? [
            window.x0,
            window.y0,
            window.x0 + window.tilesX * GRASS_TILE_M,
            window.y0 + window.tilesY * GRASS_TILE_M,
          ]
        : undefined;
    },
    /** Grow this frame's clumps (compute, before the colour pass). */
    encodeBuild(encoder: GPUCommandEncoder) {
      if (!regrow || !buildGroup) return;
      build
        .with(buildGroup)
        .with(terrain.group)
        .with(encoder)
        .dispatchWorkgroups(window.tilesX, window.tilesY);
      finish.with(buildGroup).with(encoder).dispatchWorkgroups(1);
    },
    /** Draw both tiers into the world pass. */
    draw(pass: TgpuRenderPass, cameraGroup: CameraGroup, fogGround: TgpuBindGroup) {
      if (!drawn) return;
      FIELD_LODS.forEach((_, t) => {
        drawPipeline
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogGround)
          .with(drawGroups[t])
          .withIndexBuffer(slots.indices[t].current!, "uint16")
          .drawIndexedIndirect(args, t * 20);
      });
    },
    stats(): GrassStats {
      return {
        enabled: ready,
        tiles,
        capacity,
        clumpBytes: ready ? (capacity[0] + capacity[1]) * CLUMP_BYTES : 0,
        kinds: kindNames,
      };
    },
    /** Lab probes (debug readbacks, never in a frame). */
    probes: {
      /** Draw no grass while on (a paired cost measure). */
      suppress(on: boolean) {
        suppressed = on;
      },
      async counts(): Promise<GrassCounts> {
        if (!drawn) return { near: 0, far: 0, nearFound: 0, farFound: 0 };
        const words = new Uint32Array(await readback(args, ARGS_WORDS * 4));
        return { near: words[1], far: words[6], nearFound: words[10], farFound: words[11] };
      },
      /** The last frame's clumps, both tiers. */
      async clumps(): Promise<GrassClumpRow[]> {
        if (!drawn) return [];
        const counts = new Uint32Array(await readback(args, ARGS_WORDS * 4));
        if (!slots.clumps.current) return [];
        const bytes = await readback(
          slots.clumps.current,
          (capacity[0] + capacity[1]) * CLUMP_BYTES,
        );
        const f = new Float32Array(bytes);
        const u = new Uint32Array(bytes);
        const rows: GrassClumpRow[] = [];
        for (const [tier, start, count] of [
          [0, 0, counts[1]],
          [1, capacity[0], counts[6]],
        ] as const)
          for (let i = 0; i < count; i++) {
            const o = ((start + i) * CLUMP_BYTES) / 4;
            rows.push({
              root: [f[o], f[o + 1], f[o + 2]],
              height: f[o + 3],
              kind: kindNames[u[o + 5]] ?? "?",
              tier,
              laid: 1 - (u[o + 4] >>> 24) / 255,
            });
          }
        return rows;
      },
    },
  };

  async function readback(source: GPUBuffer, bytes: number): Promise<ArrayBuffer> {
    const read = device.createBuffer({ size: bytes, usage: MAP_READ | COPY_DST });
    try {
      const encoder = device.createCommandEncoder({ label: "grass-readback" });
      encoder.copyBufferToBuffer(source, 0, read, 0, bytes);
      device.queue.submit([encoder.finish()]);
      await read.mapAsync(GPUMapMode.READ);
      return read.getMappedRange().slice(0);
    } finally {
      read.destroy();
    }
  }
}
export type GrassPass = Awaited<ReturnType<typeof createGrassPass>>;
export type GrassProbes = GrassPass["probes"];
