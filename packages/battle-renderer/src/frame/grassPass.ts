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
// - a drilled crop keeps to its plot's rows, the ones the ground under it is
//   painted with, so a wheat field reads as rows and a meadow never does;
// - a kind of ground grows a mix of grasses: the clump is one of them by its
//   own hash against their shares, a drifting grass gathered into patches.
//   The stand varies across a field in world-anchored patches: taller and
//   lower, thinner, and drier (toward straw, paler and never darker).
//   All of it is a function of the clump's tile, rank and place, so a clump
//   kept as the camera moves stays the grass it was;
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
// Grass casts no sun shadow (cost); it receives the cascades. Every number
// here that sets the look and is not the biome's lives in
// `presentation.grass` in `fixtures/game.json` (documented in
// `crates/presentation`'s `ground::tuning`), spliced into the WGSL as a constant.
import { tgpu, d, std, type TgpuBindGroup, type TgpuRenderPass } from "typegpu";
import { pcgHash } from "../shaders/pcgHash";
import type { Mat4 } from "math";
import { GRASS_HEIGHT_VARIATION } from "@packages/scene-assets/src/grass.ts";
import {
  grass_blade_vertices,
  grass_field_segments,
  grass_strip_indices,
} from "@wasm/presentation/presentation_wasm.js";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { Biome, PresentedGround, TerrainSurface } from "../terrain/terrainSurface";

type GrassRules = Biome["grass"];
import {
  GRASS_GROWTH_ROWS,
  GRASS_MIX_MAX,
  grassShapes,
  type GrassAppearances,
} from "../terrain/grassField";
import game from "@fixtures/game.json";
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
  groundLineBand,
  groundColour,
  groundCell,
  groundPaved,
  groundShore,
  groundShoulder,
  groundShoulderGrass,
  groundSite,
  groundTint,
  groundVerge,
  groundWheeling,
  groundWater,
  valueNoise,
} from "./terrainMaterial";
import { terrainLayout } from "./terrainLayout";
import type { TerrainSource } from "./terrainSource";
import { groundScars, scarredSurface } from "./groundScars";
import { triangleRuleHeight } from "./triangleRule";
import { terrainSample, type TerrainHeights } from "./terrainHeights";
import type { EnvironmentFrame } from "./environmentFrame";
import type { CameraGroup } from "./geometry";
import { FRAME_MSAA, WORLD_OUT, worldTargets } from "./targets";
import type { GpuRegistry, GpuSlot } from "./registry";
import { Clump, GRASS_CLUMP_BYTES, clumpRows, type GrassClumpRow } from "./grassClump";
import { grassVertex } from "./grassVertex";
import { ARGS_WORDS, GrassParams, grassBuildLayout, grassDrawLayout } from "./grassLayouts";
export type { GrassClumpRow } from "./grassClump";
const tuning = game.presentation.grass;

type Root = ReturnType<typeof tgpu.initFromDevice>;

const BUILD_WORKGROUP = 64;
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
  let P = &grassBuildLayout.$.params;
  let nx = (*P).counts.x;
  let ny = (*P).counts.y;
  let fx = clamp(q.x / (*P).ground.x, 0.0, f32(nx - 1u));
  let fy = clamp(q.y / (*P).ground.x, 0.0, f32(ny - 1u));
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
  let P = &grassBuildLayout.$.params;
  let facing = clamp(drop / dist, ${tuning.density.least_facing}, 1.0);
  let footprint = (*P).eye.w * dist;
  return min((*P).density.y, facing / ((*P).density.x * footprint * footprint));
}`)
  .$uses({ grassBuildLayout });

/** Whether `p` lies on a prop's footprint (widened by its margin): asked of
 *  the props listed for `p`'s cell of the table alone. */
const grassUnderProp = tgpu
  .fn(
    [d.vec2f],
    d.bool,
  )(/* wgsl */ `(p: vec2f) -> bool {
  let grid = grassBuildLayout.$.props[0];
  let size = bitcast<vec4u>(grassBuildLayout.$.props[1]);
  let g = (p - grid.xy) / grid.z;
  if (g.x < 0.0 || g.y < 0.0) { return false; }
  let c = vec2u(g);
  if (c.x >= size.x || c.y >= size.y) { return false; }
  let cell = c.y * size.x + c.x;
  let pair = bitcast<vec4u>(grassBuildLayout.$.props[2u + cell / 2u]);
  let run = select(pair.xy, pair.zw, (cell & 1u) == 1u);
  for (var i = 0u; i < run.y; i++) {
    let a = grassBuildLayout.$.props[run.x + 2u * i];
    let b = grassBuildLayout.$.props[run.x + 2u * i + 1u];
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
  let P = &grassBuildLayout.$.params;
  if (wg.x >= (*P).grid.x || wg.y >= (*P).grid.y) { return; }
  let T = (*P).window.z;
  let lo = (*P).window.xy + vec2f(f32(wg.x), f32(wg.y)) * T;
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
  zhi += 1.0 + (*P).window.w;
  let lo2 = lo - vec2f(${tuning.placement.jitter_m} + (*P).clear.y);
  let hi2 = hi + vec2f(${tuning.placement.jitter_m} + (*P).clear.y);
  for (var i = 0u; i < 4u; i++) {
    let pl = (*P).planes[i];
    let pv = vec3f(select(lo2.x, hi2.x, pl.x >= 0.0), select(lo2.y, hi2.y, pl.y >= 0.0), select(zlo, zhi, pl.z >= 0.0));
    if (dot(pl.xyz, pv) + pl.w < 0.0) { return; }
  }
  let eye = (*P).eye.xyz;
  let nearest = clamp(eye, vec3f(lo2, zlo), vec3f(hi2, zhi));
  let dNear = max(distance(eye, nearest), 0.5);
  if ((*P).eye.w * dNear >= (*P).density.w) { return; }
  let n = u32(ceil(T * T * grassDensity(dNear, max(eye.z - zlo, 0.0))));
  // The tile's own offset of the sequence, from its place on the world grid.
  let cell = vec2i(floor(lo / T + 0.5));
  let offset = grassHash(bitcast<u32>(cell.x), bitcast<u32>(cell.y)).xy;
  for (var j = li; j < n; j += ${BUILD_WORKGROUP}u) {
    // The sequence's point, jittered by a fixed amount so a sparse field
    // shows no lattice; both depend on the tile and j alone.
    let h = grassHash(bitcast<u32>(cell.x) ^ (j * 2654435761u), bitcast<u32>(cell.y) + j);
    let jitter = (vec2f(fract(h.z * 7.13), fract(h.x * 5.31)) - 0.5) * ${2 * tuning.placement.jitter_m};
    let fell = lo + T * fract(offset + f32(j + 1u) * vec2f(${R2[0]}, ${R2[1]})) + jitter;
    let fellZ = grassGround(fell);
    let dist = max(distance(eye, vec3f(fell, fellZ)), 0.5);
    let footprint = (*P).eye.w * dist;
    if (footprint >= (*P).density.w) { continue; }
    // Clump j stands once the density passes (j + 1/2) per tile.
    let rank = (f32(j) + 0.5) / (T * T);
    let rho = grassDensity(dist, max(eye.z - fellZ, 0.0));
    if (rank >= rho) { continue; }
    let cell = groundCell(fell, footprint);
    let paved = groundPaved(fell, cell);
    let site = groundSite(fell, cell, paved);
    let water = groundWater(fell, cell);
    // Bare within the margins; thinner and lower for a metre beyond them, so
    // a field meets a road or a wood without a wall of blades. Across a
    // road's shoulder it thins and lowers further as the ground is worn.
    // A wood's edge is its rect or its floor's ragged verge, whichever lies
    // farther out. A tree line is no wood: the verge's grass grows on the
    // band under it, and the field's own right up to that.
    var wood = max(site.w, forestVergeInside(fell, site.w));
    var line = vec2f(0.0, -1e9);
    if (wood > -((*P).clear.x + ${tuning.edge_m})) {
      line = groundLineBand(fell, cell, site.w);
      if (line.x > 0.5) { wood = -1e9; }
    }
    let margin = min(-wood, -water) - (*P).clear.x;
    if (margin < 0.0) { continue; }
    // Bare on the wet bank round water, thickening across the earth behind it.
    let shore = groundShore(fell, footprint, water).z;
    if (shore >= 1.0) { continue; }
    let worn = groundShoulder(fell, footprint, paved);
    // Bare on a road's own surface and at the foot of its shoulder, but for
    // a track's centre strip.
    if (worn.x > ${tuning.stand.bare_wear}) { continue; }
    let edge = smoothstep(0.0, ${tuning.edge_m}, min(margin, abs(site.z))) * (1.0 - worn.x);
    var grows = min(u32(terrainLayout.$.plots[i32(site.x)].detail.y), ${GRASS_GROWTH_ROWS - 2}u);
    if (groundVerge(site, footprint) > 0.5 || line.y > 0.0) { grows = ${GRASS_GROWTH_ROWS - 1}u; }
    let g = (*P).growth[grows];
    let stand = (*P).patches[grows];
    // A drilled crop keeps to its plot's rows, the bright bands between the
    // furrows the ground is painted with: the clump moves across them toward
    // the nearest, but never out of its plot or into a bare margin.
    let plot = terrainLayout.$.plots[i32(site.x)];
    var p = fell;
    if (g.w > 0.0 && plot.rows.z > 0.0) {
      let across = dot(fell, plot.rows.xy) / plot.rows.z;
      let toRow = (floor(across) + 0.5 - across) * plot.rows.z * g.w;
      let room = max(0.0, min(site.y - terrainLayout.$.params.verge.w, margin) - ${tuning.placement.headland_m});
      p = fell + plot.rows.xy * clamp(toRow, -room, room);
    }
    // A wheeling stays bare.
    if (groundWheeling(p, 0.0, i32(site.x)) > 0.5) { continue; }
    let root = vec3f(p, grassGround(p));
    // The side's learned scars: craters, scorch and tracks leave clumps out.
    let scar = groundScars(p, footprint);
    let S = terrainLayout.$.scarParams.grass;
    let bare = max(max(scar.weights.x, scar.weights.y), scar.weights.z * S.y) * S.x;
    // Sparse patches: the stand thins there and nowhere thickens.
    let sparse = 1.0 - smoothstep(${tuning.stand.sparse_cut[0]}, ${tuning.stand.sparse_cut[1]}, valueNoise(p * (*P).patchScales.y + vec2f(41.7, 13.1)));
    let keep = rho * g.x * (1.0 - stand.z * sparse) * mix(${tuning.stand.edge_density}, 1.0, edge) * (1.0 - bare) * (1.0 - shore) * groundShoulderGrass(worn);
    if (rank >= keep) { continue; }
    if (grassUnderProp(p)) { continue; }
    // A clump nearing its rank's threshold is small: it grows as the
    // density passes it, and shrinks away as the fade takes it.
    let grow = 1.0 - smoothstep(${tuning.stand.grow_from} * keep, keep, rank);
    let fade = 1.0 - smoothstep((*P).density.z, (*P).density.w, footprint);
    // Which of the row's grasses this clump is: its hash against their
    // shares, a drifting grass weighted up inside its drifts and down to
    // nothing outside them.
    var shares = array<f32, ${GRASS_MIX_MAX}>();
    var total = 0.0;
    let mixed = u32(g.z);
    for (var i = 0u; i < mixed; i++) {
      let entry = (*P).mixes[grows * ${GRASS_MIX_MAX}u + i];
      var share = entry.y;
      if (entry.z > 0.0) {
        let drift = smoothstep(${tuning.mix.drift_cut[0]}, ${tuning.mix.drift_cut[1]}, valueNoise(p * (*P).patchScales.w + vec2f(f32(i) * 17.3 + 3.1, f32(grows) * 7.9)));
        share *= mix(1.0, ${tuning.mix.drift_gather} * drift, entry.z);
      }
      shares[i] = share;
      total += share;
    }
    var chosen = 0u;
    if (total > 0.0) {
      chosen = mixed - 1u;
      var roll = fract(h.x * 91.37) * total;
      for (var i = 0u; i + 1u < mixed; i++) {
        if (roll < shares[i]) { chosen = i; break; }
        roll -= shares[i];
      }
    }
    let grass = (*P).mixes[grows * ${GRASS_MIX_MAX}u + chosen];
    let kind = u32(grass.x);
    let row = (*P).kinds[kind];
    // Patches a few metres across stand taller or lower, as uneven grass does.
    let tall = valueNoise(p * (*P).patchScales.x);
    let height = row.x * g.y * grow * fade * mix(${GRASS_HEIGHT_VARIATION[0]}, ${GRASS_HEIGHT_VARIATION[1]}, h.x) * mix(stand.x, stand.y, tall) * mix(${tuning.stand.edge_height}, 1.0, edge);
    if (height < ${tuning.stand.least_height_m}) { continue; }
    let centre = root + vec3f(0.0, 0.0, height * 0.5);
    let radius = height * 0.8 + 0.4;
    var shown = true;
    for (var i = 0u; i < 4u; i++) {
      let pl = (*P).planes[i];
      if (dot(pl.xyz, centre) + pl.w < -radius) { shown = false; }
    }
    if (!shown) { continue; }
    var tier = 1u;
    if (height / footprint > (*P).tiers.x * mix(${tuning.tier_jitter[0]}, ${tuning.tier_jitter[1]}, h.y)) { tier = 0u; }
    let width = max(1.0, (*P).tiers.y * footprint / max(row.z, 1e-4));
    // Dry patches, and a grass drier than its neighbours: toward straw at
    // the ground's own luminance, so no patch is darker than its field.
    let parched = smoothstep(${tuning.stand.parched_cut[0]}, ${tuning.stand.parched_cut[1]}, valueNoise(p * (*P).patchScales.z + vec2f(9.3, 77.1)));
    // What sets a clump apart from the ground under it (how dry it is, its
    // grain) goes with the clump as the fade takes it, so the field's far
    // edge meets the painted ground with the ground's own colour.
    let dryness = min(1.0, stand.w * parched + grass.w) * fade;
    // Each clump a little lighter or darker than the next, half on its own
    // and half with the tussock beneath it: the grain a field keeps when
    // it is too far to show blades. Dry grass is paler, never darker.
    let tussock = valueNoise(p * (*P).grain.x + vec2f(63.9, 27.4));
    let value = 1.0 + (*P).shading.z * fade * (fract(h.y * 57.31) + tussock - 1.0);
    let colour = value * (1.0 + (*P).grain.y * dryness) * groundTint(scarredSurface(groundColour(p, footprint, site, paved, water), scar).xyz, dryness);
    // Tracks and trampling lay the clump over (carried by the colour's alpha).
    let flat = max(scar.weights.z, scar.weights.w) * S.z;
    let slot = atomicAdd(&grassBuildLayout.$.args[tier * 5u + 1u], 1u);
    let cap = select((*P).grid.w, (*P).grid.z, tier == 0u);
    if (slot >= cap) { continue; }
    let base = select((*P).grid.z, 0u, tier == 0u);
    grassBuildLayout.$.clumps[base + slot] = GrassClump(
      root, height, pack4x8unorm(vec4f(sqrt(max(colour, vec3f(0.0))), 1.0 - flat)), kind,
      sin(h.z * 6.2831853), cos(h.z * 6.2831853), width);
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
    groundPaved,
    groundSite,
    groundWater,
    groundShore,
    groundShoulder,
    groundShoulderGrass,
    groundVerge,
    groundWheeling,
    groundColour,
    groundTint,
    forestVergeInside,
    groundLineBand,
    groundScars,
    scarredSurface,
    valueNoise,
    Clump,
  });

/** Clamp each tier's count to its capacity, keeping the raw count. */
const finishFn = tgpu
  .computeFn({ workgroupSize: [1] })(/* wgsl */ `{
  let P = &grassBuildLayout.$.params;
  for (var t = 0u; t < 2u; t++) {
    let raw = atomicLoad(&grassBuildLayout.$.args[t * 5u + 1u]);
    atomicStore(&grassBuildLayout.$.args[10u + t], raw);
    atomicStore(&grassBuildLayout.$.args[t * 5u + 1u], min(raw, select((*P).grid.w, (*P).grid.z, t == 0u)));
  }
}`)
  .$uses({ grassBuildLayout });

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

export async function createGrassPass(
  root: Root,
  registry: GpuRegistry,
  environment: EnvironmentFrame,
  terrain: TerrainSource,
  terrainHeights: TerrainHeights,
  densityMultiplier = 1,
) {
  const device = registry.device;
  const fragment = tgpu.fragmentFn({
    in: {
      clip: d.builtin.position,
      world: d.vec3f,
      root: d.vec3f,
      normal: d.vec3f,
      albedo: d.vec3f,
      ground: d.f32,
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
    // A blade's shading normal leans to the ground's, so the field lights
    // like the ground it grows on and never glitters; far clumps more so.
    const shading = std.normalize(std.mix(n, up, v.ground));
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
      tuning.blade.roughness,
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

  const drawPipeline = root
    .createRenderPipeline({
      vertex: grassVertex,
      fragment,
      targets: worldTargets(),
      primitive: { topology: "triangle-list", cullMode: "none" },
      depthStencil: battleWorldDepth("read-write"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("grassDraw");
  const build = root.createComputePipeline({ compute: buildFn }).$name("grassBuild");
  const finish = root.createComputePipeline({ compute: finishFn }).$name("grassFinish");
  await Promise.all([drawPipeline.initAsync(), build.initAsync(), finish.initAsync()]);

  const { STORAGE, COPY_DST, COPY_SRC, INDIRECT, INDEX, MAP_READ } = GPUBufferUsage;
  const params = registry.own(root.createBuffer(GrassParams).$usage("uniform"));
  /** The field's tiers, near then far, by their blades' segments. */
  const tierSegments = [...grass_field_segments()];
  const tierUniforms = tierSegments.map(() =>
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
  let ground: PresentedGround | null = null;
  let appearances: GrassAppearances | null = null;
  let rules: GrassRules | null = null;
  /** The lab's rules in place of the biome's (`probes.retune`). */
  let retuned: GrassRules | null = null;
  let kindNames: string[] = [];
  let capacity: [number, number] = [0, 0];
  let indexCounts: [number, number] = [0, 0];
  let ready = false;
  let tiles = 0;
  let buildGroup: ReturnType<typeof makeBuildGroup> | null = null;
  let drawGroups: ReturnType<typeof makeDrawGroup>[] = [];
  /** This frame's window (`Presentation.grass_window`): its corner, tiles across
   *  and up, reach, and far corner. */
  let window: Float64Array = new Float64Array(7);
  /** `GrassParams` as the ground laid it (`Presentation.grass_params`), its
   *  per-frame fields (the planes, the eye, the window's corner and tile, its
   *  tiles) written over each regrow. */
  let frameParams: Uint32Array | null = null;

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
    if (!surface || !grid || !appearances || !ground) return;
    rules = retuned ?? surface.biome.grass;
    ground.retune_grass(retuned && JSON.stringify(retuned));
    const names = JSON.parse(ground.grass_kinds()) as string[];
    if (!names.every((name) => appearances!.has(name))) return;
    const packed = grassShapes(names, appearances);
    const shapes = packed.shapes();
    const maxBlades = packed.max_blades();
    let laid: Uint32Array;
    try {
      laid = ground.grass_params(packed, densityMultiplier);
    } finally {
      packed.free();
    }
    kindNames = names;
    const props = ground.grass_props();
    slots.props.set(storage("grass-props", props.byteLength));
    device.queue.writeBuffer(slots.props.current!, 0, props);
    capacity = [rules.capacity[0], rules.capacity[1]];
    slots.clumps.set(
      storage("grass-clumps", (capacity[0] + capacity[1]) * GRASS_CLUMP_BYTES, COPY_SRC),
    );
    slots.shapes.set(storage("grass-shapes", shapes.byteLength));
    device.queue.writeBuffer(slots.shapes.current!, 0, shapes);
    tierSegments.forEach((segments, t) => {
      const indices = grass_strip_indices(maxBlades, segments);
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
      tierUniforms[t].write(
        d.vec4u(t, t === 0 ? 0 : capacity[0], grass_blade_vertices(segments), 0),
      );
    });
    frameParams = laid;
    buildGroup = makeBuildGroup();
    drawGroups = tierSegments.map((_, t) => makeDrawGroup(t));
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
    /** The terrain the grass grows on, the grass kinds and the ground the
     *  scene's presentation resolved for it; null grows none. */
    setWorld(
      terrainSurface: TerrainSurface | null,
      kinds: GrassAppearances | null,
      presented: PresentedGround | null,
    ) {
      [surface, appearances, ground] = [terrainSurface, kinds, presented];
      rebuild();
    },
    /** Fit this frame's window and write its parameters. */
    prepare(viewProj: Mat4) {
      drawn = false;
      regrow = false;
      if (!ready || suppressed || !frameParams || !surface || !ground) return;
      window = ground.grass_window();
      tiles = window.length ? window[2] * window[3] : 0;
      if (!tiles) return;
      drawn = true;
      // The reach follows the pixel span alone.
      let same = grownFor[16] === Math.fround(window[4]);
      for (let i = 0; i < 16 && same; i++) same = grownFor[i] === Math.fround(viewProj[i]);
      if (same) return;
      grownFor.set(viewProj);
      grownFor[16] = window[4];
      regrow = true;
      ground.grass_frame_params(frameParams, window);
      params.write(frameParams.buffer as ArrayBuffer);
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
      return regrow ? [window[0], window[1], window[5], window[6]] : undefined;
    },
    /** Grow this frame's clumps (compute, before the colour pass). */
    encodeBuild(encoder: GPUCommandEncoder) {
      if (!regrow || !buildGroup) return;
      build
        .with(buildGroup)
        .with(terrain.group)
        .with(encoder)
        .dispatchWorkgroups(window[2], window[3]);
      finish.with(buildGroup).with(encoder).dispatchWorkgroups(1);
    },
    /** Draw both tiers into the world pass. */
    draw(pass: TgpuRenderPass, cameraGroup: CameraGroup, fogGround: TgpuBindGroup) {
      if (!drawn) return;
      tierSegments.forEach((_, t) => {
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
        clumpBytes: ready ? (capacity[0] + capacity[1]) * GRASS_CLUMP_BYTES : 0,
        kinds: kindNames,
      };
    },
    /** Lab probes (debug readbacks, never in a frame). */
    probes: {
      /** Draw no grass while on (a paired cost measure). */
      suppress(on: boolean) {
        suppressed = on;
      },
      /** Grow by `rules` in place of the biome's (null returns to them): one
       *  page compares fields that differ in a single number. */
      retune(next: GrassRules | null) {
        retuned = next;
        rebuild();
      },
      /** The rules the field grows by now. */
      rules: () => rules,
      async counts(): Promise<GrassCounts> {
        if (!drawn) return { near: 0, far: 0, nearFound: 0, farFound: 0 };
        const words = new Uint32Array(await readback(args, ARGS_WORDS * 4));
        return { near: words[1], far: words[6], nearFound: words[10], farFound: words[11] };
      },
      /** The clumps of the frame submitted last before the call, both tiers.
       *  Both copies are queued in the calling task, so a call in the task
       *  that drew a frame reads that frame's clumps. */
      async clumps(): Promise<GrassClumpRow[]> {
        if (!drawn || !slots.clumps.current) return [];
        const [near] = capacity;
        const names = kindNames;
        const [countBytes, bytes] = await Promise.all([
          readback(args, ARGS_WORDS * 4),
          readback(slots.clumps.current, (capacity[0] + capacity[1]) * GRASS_CLUMP_BYTES),
        ]);
        const counts = new Uint32Array(countBytes);
        const tiers = [
          [0, 0, counts[1]],
          [1, near, counts[6]],
        ] as const;
        return clumpRows(bytes, tiers, names);
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
