// The biome's ground material on the GPU: per pixel, the plot under the
// point (walking the plot split), its rows and painterly value noise, the
// verge along every plot edge, then the forest floor, the road and the water
// bed exactly where the simulation's rules put them. It returns linear albedo
// and roughness; lighting, shadow and FogTerm stay with the world pass.
//
// The plot edges wander (a small warp gives them a hand-cut line); the road
// and water masks are the simulation's own shapes, so a road's 50% blend is
// on the road rule's edge. The forest floor (leaf litter, moss, humus, roots)
// covers the simulation's forest rects and meets the field across a ragged
// verge on each rect's edge (slice 19b): the rect stays the rule, only its
// look is softened. Under the crowns, `groundDapple` lets sun flecks through.
// Detail finer than a pixel fades to its mean, so the patchwork neither
// shimmers nor changes value with zoom.
//
// Rewritten (reuse manifest, technique) from reading ~/dev/game
// battle-renderer/src/shaders/terrainMaterial.ts: the mottle, drift and
// feathered road-edge ideas; not its baked distance texture or rock layers.
import { tgpu, d, std } from "typegpu";
import { MAX_PLOT_DEPTH, NODE_FLOATS, type PlotTree } from "../terrain/plots";
import { RECT_FLOATS, type TerrainSurface } from "../terrain/terrainSurface";
import { SCAR_CHANNELS, type ForestFloor, type ScarMark } from "../terrain/biome";
import type { Rgb } from "../light/sceneLight";
import type { GpuRegistry, GpuSlot } from "./registry";
import { createScarTexture, type GroundMarks } from "./scarTexture";

const TerrainParams = d.struct({
  /** The plot region: minX, minY, maxX, maxY. */
  region: d.vec4f,
  /** Road segments, forest rects, water rects; the plot count (diagnostic). */
  counts: d.vec4u,
  /** Edge warp metres, 1 / warp scale, 1 / fine mottle scale, 1 / broad mottle scale. */
  shape: d.vec4f,
  /** Linear rgb, verge half width. */
  verge: d.vec4f,
  /** Verge feather, road feather, and the pixel footprints (metres) over
   *  which plots give way to the distant colour. */
  feathers: d.vec4f,
  /** Linear rgb, road roughness. */
  road: d.vec4f,
  /** Road mottle, then unused. */
  roadDetail: d.vec4f,
  /** The forest floor's leaf litter, moss and humus (linear rgb). */
  forestLitter: d.vec4f,
  forestMoss: d.vec4f,
  forestHumus: d.vec4f,
  /** 1 / patch scale, mottle, roughness, root contrast. */
  forestDetail: d.vec4f,
  /** Verge width, verge warp, 1 / verge warp scale, 1 / root spacing. */
  forestVerge: d.vec4f,
  /** 1 / fleck size, fleck share, the sun a fleck lets through, unused. */
  forestDapple: d.vec4f,
  waterBed: d.vec4f,
  distant: d.vec4f,
});
/** The scar texture's grid and the biome's scar look (`biome.scars`). */
const ScarParams = d.struct({
  /** 1 / the grid's width and depth in metres, the cell's side, 1 when there
   *  is ground to draw (else 0). */
  grid: d.vec4f,
  /** 255 / each channel's `full`: a texel's channel over its full mark. */
  full: d.vec4f,
  /** Linear rgb and strength per channel. */
  crater: d.vec4f,
  /** The soil thrown onto a crater's rim: linear rgb, strength. */
  ejecta: d.vec4f,
  scorch: d.vec4f,
  tracks: d.vec4f,
  trampled: d.vec4f,
  /** Crater relief metres, rim; 0, 0. */
  relief: d.vec4f,
  /** Grass: thin, tracks' share of it, flatten; 0. */
  grass: d.vec4f,
});
const PlotNode = d.struct({ line: d.vec4f, children: d.vec4i });
const PlotRecord = d.struct({
  /** Linear rgb, roughness. */
  colour: d.vec4f,
  /** Across the rows (unit), row period metres, row contrast. */
  rows: d.vec4f,
  /** Mottle strength, the plot's kind (an index into the biome's plots). */
  detail: d.vec4f,
});
const RoadSegment = d.struct({ ends: d.vec4f, half: d.vec4f });

export const terrainLayout = tgpu.bindGroupLayout({
  params: { uniform: TerrainParams, visibility: ["fragment", "compute"] },
  nodes: {
    storage: (n: number) => d.arrayOf(PlotNode, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  plots: {
    storage: (n: number) => d.arrayOf(PlotRecord, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  roads: {
    storage: (n: number) => d.arrayOf(RoadSegment, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  /** Forest rects, then water rects: minX, minY, maxX, maxY. */
  rects: {
    storage: (n: number) => d.arrayOf(d.vec4f, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  scarParams: { uniform: ScarParams, visibility: ["fragment", "compute"] },
  /** The side's learned ground (`scarTexture.ts`): crater, scorch, tracks,
   *  trampled per cell. */
  scars: { texture: d.texture2d(d.f32), visibility: ["fragment", "compute"] },
  scarSampler: { sampler: "filtering", visibility: ["fragment", "compute"] },
});

/** The patchwork fades to the distant colour over this far inside its region's edge. */
const DISTANT_FADE_M = 600;
/** Plots give way to the distant colour as the smallest plot's side shrinks
 *  from 6 to 2 pixels (as fractions of it, the pixel footprint). */
const PLOT_PIXELS_FADE = [1 / 6, 1 / 2] as const;
const NODE_BYTES = 32;
const PLOT_BYTES = 48;
const ROAD_BYTES = 32;

/** Value noise on a unit lattice, in [0, 1]: an integer hash per lattice
 *  corner (no sin-hash, which loses precision kilometres out), smoothly
 *  interpolated. */
export const valueNoise = tgpu.fn(
  [d.vec2f],
  d.f32,
)(`(p: vec2f) -> f32 {
  let cell = floor(p);
  let f = p - cell;
  let u = f * f * (3.0 - 2.0 * f);
  let ix = bitcast<u32>(i32(cell.x));
  let iy = bitcast<u32>(i32(cell.y));
  var h = array<f32, 4>();
  for (var k = 0u; k < 4u; k++) {
    var s = ((ix + (k & 1u)) * 1597334677u) ^ ((iy + (k >> 1u)) * 3812015801u);
    s = s * 747796405u + 2891336453u;
    s = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    s = (s >> 22u) ^ s;
    h[k] = f32(s) * (1.0 / 4294967295.0);
  }
  return mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
}`);

/** Two octaves of value noise around 0, fading the fine one as it drops below
 *  a pixel (`footprint` is metres per pixel). */
const mottle = tgpu.fn(
  [d.vec2f, d.f32],
  d.f32,
)((xy, footprint) => {
  "use gpu";
  const shape = terrainLayout.$.params.shape;
  const broad = valueNoise(std.mul(xy, shape.w)) - 0.5;
  const fine = valueNoise(std.add(std.mul(xy, shape.z), d.vec2f(37.1, 11.3))) - 0.5;
  const fineShown = 1 - std.smoothstep(0.25, 1, footprint * shape.z);
  return broad * 0.8 + fine * 0.8 * fineShown;
});

/** How far `xy` lies inside rect `r` (negative outside). */
const rectInside = tgpu.fn(
  [d.vec2f, d.vec4f],
  d.f32,
)((xy, r) => {
  "use gpu";
  return std.min(std.min(xy.x - r.x, r.z - xy.x), std.min(xy.y - r.y, r.w - xy.y));
});

/** How far `xy` lies inside the deepest forest rect (negative outside): the
 *  simulation's forest. */
export const groundForest = tgpu.fn(
  [d.vec2f],
  d.f32,
)((xy) => {
  "use gpu";
  let forest = d.f32(-1e9);
  for (let i = d.u32(0); i < terrainLayout.$.params.counts.y; i++) {
    forest = std.max(forest, rectInside(xy, terrainLayout.$.rects[i]));
  }
  return forest;
});

/** How far `xy` lies inside the forest floor's drawn edge, in metres
 *  (negative outside), `forest` metres inside the simulation's forest. The
 *  drawn edge is a verge `verge_m` wide lying mostly outside the rect, its
 *  line wandering and broken into patches, so the wood meets the field
 *  without a ruled edge. The rect stays the rule; this is only its look, and
 *  the grass stops at whichever edge lies farther out. */
export const forestVergeInside = tgpu.fn(
  [d.vec2f, d.f32],
  d.f32,
)((xy, forest) => {
  "use gpu";
  const verge = terrainLayout.$.params.forestVerge;
  const wander = (valueNoise(std.add(std.mul(xy, verge.z), d.vec2f(71.3, 23.9))) - 0.5) * 2;
  const patches = valueNoise(std.add(std.mul(xy, 0.7), d.vec2f(5.1, 91.7))) - 0.5;
  return forest + verge.x * 0.5 + wander * verge.y + patches * verge.x;
});

/** The forest floor's weight at `xy`: 1 inside its drawn edge, 0 outside,
 *  feathered over a pixel at least. */
const forestFloorWeight = tgpu.fn(
  [d.vec2f, d.f32, d.f32],
  d.f32,
)((xy, forest, footprint) => {
  "use gpu";
  const feather = std.max(footprint, terrainLayout.$.params.forestVerge.x * 0.1);
  return std.smoothstep(-feather, feather, forestVergeInside(xy, forest));
});

/** The forest floor's linear albedo at `xy`: leaf litter in patches of moss
 *  and dark humus, crossed by roots, under the ground's own mottle `noise`. */
const forestFloor = tgpu.fn(
  [d.vec2f, d.f32, d.f32],
  d.vec3f,
)((xy, footprint, noise) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const detail = params.forestDetail;
  const at = std.mul(xy, detail.x);
  const moss = std.smoothstep(0.45, 0.7, valueNoise(std.add(at, d.vec2f(13.3, 7.7))));
  const humus = std.smoothstep(
    0.5,
    0.75,
    valueNoise(std.add(std.mul(at, 2.3), d.vec2f(2.9, 41.1))),
  );
  let floor = std.mix(params.forestLitter.xyz, params.forestMoss.xyz, moss);
  floor = std.mix(floor, params.forestHumus.xyz, humus * 0.8);
  // Roots: thin dark lines where a noise field crosses its middle, broken
  // into short runs by a second field, fading to their mean below a few
  // pixels a line.
  const rootsAt = std.mul(xy, params.forestVerge.w);
  const ridge = std.abs(valueNoise(std.add(rootsAt, d.vec2f(29.1, 3.3))) - 0.5);
  const runs = std.smoothstep(
    0.55,
    0.75,
    valueNoise(std.add(std.mul(rootsAt, 1.7), d.vec2f(8.3, 17.9))),
  );
  const shown = 1 - std.smoothstep(0.02, 0.08, footprint * params.forestVerge.w);
  const line = (1 - std.smoothstep(0.015, 0.035, ridge)) * runs;
  const root = line * shown + 0.02 * (1 - shown);
  floor = std.mul(floor, 1 - detail.w * root);
  return std.mul(floor, 1 + detail.y * noise);
});

/** How much sun reaches the ground through the canopy at `xy` (0 where the
 *  canopy's shadow is whole): sun flecks under the forest's crowns. The
 *  shadow map sees each crown as solid; a real crown lets light through its
 *  gaps. Detail finer than a pixel fades to the flecks' mean. */
export const groundDapple = tgpu.fn(
  [d.vec2f, d.f32],
  d.f32,
)((xy, footprint) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const forest = groundForest(xy);
  if (forest < -params.forestVerge.x - params.forestVerge.y) {
    return 0;
  }
  const dapple = params.forestDapple;
  const at = std.mul(xy, dapple.x);
  const n = valueNoise(at) * 0.65 + valueNoise(std.add(std.mul(at, 2.7), d.vec2f(3.7, 8.1))) * 0.35;
  const cut = 1 - dapple.y;
  const fleck = std.smoothstep(cut - 0.06, cut + 0.06, n);
  const shown = 1 - std.smoothstep(0.1, 0.5, footprint * dapple.x);
  const flecks = std.mix(dapple.y, fleck, shown);
  return flecks * dapple.z * forestFloorWeight(xy, forest, footprint);
});

/** Where `xy` sits in the ground's features, in metres:
 *  `(plot, edge, road, forest)`. `plot` is the plot's index (a whole
 *  number); `edge` the distance to its (warped) edge; `road` how far inside
 *  the nearest road's edge (negative outside); `forest` how far inside the
 *  deepest forest rect (negative outside). The ground's colour and the grass
 *  both read it, so grass grows exactly where the ground says what it is. */
export const groundSite = tgpu.fn(
  [d.vec2f],
  d.vec4f,
)((xy) => {
  "use gpu";
  const params = terrainLayout.$.params;
  // The plot under the (warped) point, and the distance to its edge.
  const warp = d.vec2f(
    valueNoise(std.mul(xy, params.shape.y)) - 0.5,
    valueNoise(std.add(std.mul(xy, params.shape.y), d.vec2f(19.7, 5.3))) - 0.5,
  );
  const q = std.add(xy, std.mul(warp, params.shape.x * 2));
  let edge = rectInside(q, params.region);
  let node = d.i32(0);
  let leaf = d.i32(0);
  // Bounded by the depth generation refuses to exceed.
  for (let step = 0; step < MAX_PLOT_DEPTH; step++) {
    const n = terrainLayout.$.nodes[node];
    const s = std.dot(n.line.xy, q) - n.line.z;
    edge = std.min(edge, std.abs(s));
    let child = n.children.y;
    if (s >= 0) {
      child = n.children.x;
    }
    if (child < 0) {
      leaf = -1 - child;
      break;
    }
    node = child;
  }
  // The road: within half width of a segment, the simulation's road rule.
  let road = d.f32(-1e9);
  for (let i = d.u32(0); i < params.counts.x; i++) {
    const seg = terrainLayout.$.roads[i];
    const a = seg.ends.xy;
    const ab = std.sub(seg.ends.zw, a);
    const t = std.clamp(std.dot(std.sub(xy, a), ab) / std.max(std.dot(ab, ab), 1e-6), 0, 1);
    const off = std.length(std.sub(xy, std.add(a, std.mul(ab, t))));
    road = std.max(road, seg.half.x - off);
  }
  return d.vec4f(d.f32(leaf), edge, road, groundForest(xy));
});

/** How far `xy` lies inside the deepest water rect (negative outside). */
export const groundWater = tgpu.fn(
  [d.vec2f],
  d.f32,
)((xy) => {
  "use gpu";
  const params = terrainLayout.$.params;
  let bed = d.f32(-1e9);
  for (let i = d.u32(0); i < params.counts.z; i++) {
    bed = std.max(bed, rectInside(xy, terrainLayout.$.rects[params.counts.y + i]));
  }
  return bed;
});

/** The verge's weight at a site, 1 on it: along every plot edge and beside
 *  the road. It holds its full colour right up to the edge, so neighbouring
 *  plots meet in one colour and a plot boundary never steps from pixel to
 *  pixel; it fades out over a pixel at least. */
export const groundVerge = tgpu.fn(
  [d.vec4f, d.f32],
  d.f32,
)((site, footprint) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const vergeEdge = std.min(site.y, std.max(-site.z, 0));
  const vergeHalf = params.verge.w;
  return (
    1 - std.smoothstep(vergeHalf, vergeHalf + std.max(params.feathers.x, footprint), vergeEdge)
  );
});

/** Linear albedo and roughness of the ground at `xy` with site `site` and
 *  water depth `water`, `footprint` the metres one pixel spans there. */
export const groundColour = tgpu.fn(
  [d.vec2f, d.f32, d.vec4f, d.f32],
  d.vec4f,
)((xy, footprint, site, water) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const aa = footprint * 0.5;
  const plot = terrainLayout.$.plots[d.i32(site.x)];
  const noise = mottle(xy, footprint);
  let albedo = std.mul(plot.colour.xyz, 1 + plot.detail.x * noise);
  let roughness = plot.colour.w;
  // Rows: a cosine across the plot, fading to its mean below two pixels a row.
  const period = plot.rows.z;
  if (period > 0) {
    const phase = std.dot(xy, plot.rows.xy) / period;
    const stripe = 0.5 + 0.5 * std.cos(phase * 6.2831853);
    const shown = 1 - std.smoothstep(0.25, 0.6, footprint / period);
    albedo = std.mul(albedo, 1 - plot.rows.w * (0.5 + (stripe - 0.5) * shown));
  }

  const verge = groundVerge(site, footprint);
  albedo = std.mix(albedo, std.mul(params.verge.xyz, 1 + 0.18 * noise), verge);
  roughness = std.mix(roughness, 0.95, verge);

  // Past the patchwork, and where plots shrink to a few pixels, the distant
  // land: its mean colour.
  const inRegion = rectInside(xy, params.region);
  const distant = std.max(
    1 - std.smoothstep(0, DISTANT_FADE_M, inRegion),
    std.smoothstep(params.feathers.z, params.feathers.w, footprint),
  );
  albedo = std.mix(albedo, params.distant.xyz, distant);

  // The forest floor, over the simulation's forest rects and their verge.
  const forest = forestFloorWeight(xy, site.w, footprint);
  if (forest > 0) {
    albedo = std.mix(albedo, forestFloor(xy, footprint, noise), forest);
    roughness = std.mix(roughness, params.forestDetail.z, forest);
  }

  // The road surface, over all but water.
  const roadFeather = std.max(params.feathers.y, footprint) * 0.5;
  const onRoad = std.smoothstep(-roadFeather, roadFeather, site.z);
  const surface = std.mul(params.road.xyz, 1 + params.roadDetail.x * noise);
  albedo = std.mix(albedo, surface, onRoad);
  roughness = std.mix(roughness, params.road.w, onRoad);

  // The water bed, under the simulation's water rects (water wins over road).
  const bed = std.smoothstep(-aa, aa, water);
  albedo = std.mix(albedo, params.waterBed.xyz, bed);
  return d.vec4f(std.max(albedo, d.vec3f(0)), roughness);
});

/** Linear albedo and roughness of the ground at `world`, with `footprint`
 *  the metres one pixel spans there. */
export const groundSurface = tgpu.fn(
  [d.vec3f, d.f32],
  d.vec4f,
)((world, footprint) => {
  "use gpu";
  const xy = world.xy;
  return groundColour(xy, footprint, groundSite(xy), groundWater(xy));
});

/** The scars at a point: `weights` (crater bowl, soot, tracks, trampled,
 *  each in [0, 1]), `relief` (the surface's slope added by crater and rim,
 *  x and y, then the thrown-soil ring's weight, then the crater's depth over
 *  its full), and `at` (where on the ground the scars were read: `xy`, or
 *  deeper along the view for a bowl, `groundScarsSeen`). */
export const ScarSample = d
  .struct({ weights: d.vec4f, relief: d.vec4f, at: d.vec4f })
  .$name("ScarSample");

/** Crater depth never reads past this many full craters (bytes saturate). */
const CRATER_DEPTH_CAP = 1.6;
/** The rim is read from the crater field this many cells out, diagonally. */
const RIM_REACH_CELLS = 1.5;
/** A full crater's floor is this much of its soil's brightness. */
const CRATER_CAVITY = 0.7;
/** The steepest a scar tilts the shading normal (tan of 40°). */
const MAX_SLOPE = 0.84;
/** The bowl's lip: the depth (in fulls) its edge crosses, give or take
 *  half the wander, by noise over `LIP_WANDER_M`. */
const LIP = [0.3, 0.14] as const;
const LIP_WANDER_M = 0.45;
/** The thrown-soil ring: the cubic crater field's skirt from this depth out
 *  to the lip, its outer edge wandering by the same share. */
const RING = [0.035, 0.03] as const;
/** Ash flecks: noise at this scale (per metre) crossing an edge that falls
 *  from the first value at the scorch's reach to the second at its heart. */
const ASH_SCALE = 4.5;
const ASH_EDGE = [0.97, 0.72] as const;
/** A rut's edge: the eased track weight it crosses, give or take by noise. */
const RUT = [0.35, 0.2] as const;

/** The four channels at `uv`, reconstructed by a cubic B-spline over the
 *  1 m cells (four bilinear taps): a crater's footprint comes back round
 *  and smooth, so its thresholded edges are circles, not diamonds. */
const scarCubic = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(uv: vec2f) -> vec4f {
  let tex = terrainLayout.$.scars;
  let smp = terrainLayout.$.scarSampler;
  let size = vec2f(textureDimensions(tex));
  let p = uv * size - 0.5;
  let i = floor(p);
  let f = p - i;
  let f2 = f * f;
  let f3 = f2 * f;
  let w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
  let w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
  let w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
  let w3 = f3 / 6.0;
  let g0 = w0 + w1;
  let g1 = w2 + w3;
  let h0 = (i - 0.5 + w1 / g0) / size;
  let h1 = (i + 1.5 + w3 / g1) / size;
  let a = textureSampleLevel(tex, smp, vec2f(h0.x, h0.y), 0.0);
  let b = textureSampleLevel(tex, smp, vec2f(h1.x, h0.y), 0.0);
  let c = textureSampleLevel(tex, smp, vec2f(h0.x, h1.y), 0.0);
  let e = textureSampleLevel(tex, smp, vec2f(h1.x, h1.y), 0.0);
  return g0.y * (g0.x * a + g1.x * b) + g1.y * (g0.x * c + g1.x * e);
}`)
  .$uses({ terrainLayout });

/** `x` crossing `edge`, anti-aliased over `width` (a pixel's worth of `x`). */
const crossing = tgpu.fn(
  [d.f32, d.f32, d.f32],
  d.f32,
)(/* wgsl */ `(x: f32, edge: f32, width: f32) -> f32 {
  let w = max(width, 1e-3);
  return smoothstep(edge - w, edge + w, x);
}`);

/** The side's learned scars at `xy` (`footprint` the metres a pixel spans).
 *  The cells are reconstructed by a cubic B-spline and cut by hard,
 *  noise-wandered iso-lines, anti-aliased over one pixel whatever the zoom:
 *  the bowl's lip, the outer edge of its ring of thrown soil, each rut's
 *  edge, and flecks of ash near a burst. Shadows here are soft-edged and even; scars are
 *  hard-edged and ragged. The bowl and rim add a slope. Zero off the grid
 *  and where nothing is marked. */
export const groundScars = tgpu
  .fn(
    [d.vec2f, d.f32],
    ScarSample,
  )(/* wgsl */ `(xy: vec2f, footprint: f32) -> ScarSample {
  var out: ScarSample;
  out.at = vec4f(xy, 0.0, 0.0);
  let P = terrainLayout.$.scarParams;
  if (P.grid.w <= 0.0) { return out; }
  let cell = P.grid.z;
  let uv = xy * P.grid.xy;
  if (any(uv < vec2f(0.0)) || any(uv > vec2f(1.0))) { return out; }
  let texel = P.grid.xy * cell;
  let tex = terrainLayout.$.scars;
  let smp = terrainLayout.$.scarSampler;
  // Cheap reject: nothing marked within reach.
  let r = ${RIM_REACH_CELLS};
  let a = textureSampleLevel(tex, smp, uv + texel * vec2f(-r, -r), 0.0);
  let b = textureSampleLevel(tex, smp, uv + texel * vec2f(r, -r), 0.0);
  let c = textureSampleLevel(tex, smp, uv + texel * vec2f(-r, r), 0.0);
  let e = textureSampleLevel(tex, smp, uv + texel * vec2f(r, r), 0.0);
  let near = max(max(a, b), max(c, e));
  let s = scarCubic(uv);
  if (max(max(s.x, s.y), max(s.z, s.w)) + max(max(near.x, near.y), max(near.z, near.w)) <= 0.0) {
    return out;
  }
  let detail = 1.0 - smoothstep(0.5, 2.0, footprint / cell);
  let k = P.full;
  let depth = min(s.x * k.x, ${CRATER_DEPTH_CAP});
  let wide = min((a.x + b.x + c.x + e.x) * 0.25 * k.x, ${CRATER_DEPTH_CAP});
  // Slopes per metre of the crater field, from taps either side.
  let h = 0.75;
  let xp = textureSampleLevel(tex, smp, uv + texel * vec2f(h, 0.0), 0.0);
  let xm = textureSampleLevel(tex, smp, uv - texel * vec2f(h, 0.0), 0.0);
  let yp = textureSampleLevel(tex, smp, uv + texel * vec2f(0.0, h), 0.0);
  let ym = textureSampleLevel(tex, smp, uv - texel * vec2f(0.0, h), 0.0);
  let bowlSlope = vec2f(xp.x - xm.x, yp.x - ym.x) * k.x / (2.0 * h * cell);
  let wideSlope = vec2f((b.x + e.x) - (a.x + c.x), (c.x + e.x) - (a.x + b.x)) * 0.5 * k.x / (2.0 * r * cell);
  // The height: down into the bowl, up onto the rim just outside it.
  let onRim = select(0.0, 1.0, wide > depth);
  let slope = (P.relief.y * onRim * (wideSlope - bowlSlope) - bowlSlope) * P.relief.x * detail;
  // Iso-lines, each crossing within a pixel.
  let grain = valueNoise(xy * ${1 / LIP_WANDER_M} + vec2f(5.7, 2.1)) - 0.5;
  let pixelDepth = length(bowlSlope) * footprint;
  let bowl = crossing(depth, ${LIP[0]} + ${LIP[1]} * grain, pixelDepth);
  let ring = crossing(depth, ${RING[0]} + ${RING[1]} * grain, pixelDepth) * (1.0 - bowl);
  // Soot: ash in flecks over the scorch, thickest near the burst, never a
  // solid sheet: a flecked dark is one thing no shadow here is.
  let burnt = saturate(s.y * k.y);
  let fleckAt = valueNoise(xy * ${ASH_SCALE} + vec2f(8.1, 3.3)) * 0.75 + valueNoise(xy * ${ASH_SCALE * 3.1}) * 0.25;
  let ash = crossing(fleckAt, mix(${ASH_EDGE[0]}, ${ASH_EDGE[1]}, burnt), footprint * ${ASH_SCALE}) * step(0.45, burnt);
  let soot = ash * (1.0 - bowl) * (1.0 - ring);
  let wear = 1.0 - (1.0 - saturate(s.zw * k.zw)) * (1.0 - saturate(s.zw * k.zw));
  // Ruts have edges: tracks cross their iso-line within a pixel, ragged.
  let trackSlope = vec2f(xp.z - xm.z, yp.z - ym.z) * k.z / (2.0 * h * cell);
  let rut = crossing(wear.x, ${RUT[0]} + ${RUT[1]} * grain, length(trackSlope) * footprint);
  out.weights = vec4f(bowl, soot, rut * wear.x, wear.y);
  out.relief = vec4f(slope, ring, depth);
  return out;
}`)
  .$uses({ terrainLayout, valueNoise, scarCubic, crossing, ScarSample });

/** The scars as seen from `eye` at ground point `world`: a bowl is looked
 *  into, not painted on. The read point steps down the view ray to the
 *  bowl's depth (twice), so at a grazing view the near wall hides the floor
 *  and the far wall faces the eye, as a hole does. Shading only: the ground
 *  and every shadow and fog lookup stay where they are. */
export const groundScarsSeen = tgpu
  .fn(
    [d.vec3f, d.vec3f, d.f32],
    ScarSample,
  )(/* wgsl */ `(world: vec3f, eye: vec3f, footprint: f32) -> ScarSample {
  var scar = groundScars(world.xy, footprint);
  if (scar.relief.w <= 0.0) { return scar; }
  let view = normalize(eye - world);
  let run = -view.xy / max(view.z, 0.25);
  let deep = terrainLayout.$.scarParams.relief.x;
  var at = world.xy;
  for (var i = 0; i < 2; i++) {
    at = world.xy + run * min(scar.relief.w, ${CRATER_DEPTH_CAP}) * deep * 0.5;
    scar = groundScars(at, footprint);
  }
  return scar;
}`)
  .$uses({ terrainLayout, groundScars });

/** `surface` (linear albedo, roughness) under the scars `scar`: trampled
 *  grass pales, tracks churn it to soil, ash flecks it near each burst, a
 *  crater's fresh soil rings it and its floor darkens with depth.
 *  Soot and soil carry a fine grain no shadow has. */
export const scarredSurface = tgpu
  .fn(
    [d.vec4f, ScarSample],
    d.vec4f,
  )(/* wgsl */ `(surface: vec4f, scar: ScarSample) -> vec4f {
  let P = terrainLayout.$.scarParams;
  let w = scar.weights;
  let xy = scar.at.xy;
  var albedo = surface.xyz;
  var roughness = surface.w;
  let grain = valueNoise(xy * 1.7);
  let fleck = valueNoise(xy * 6.3 + vec2f(2.2, 9.1));
  albedo = mix(albedo, P.trampled.xyz * mix(0.9, 1.1, grain), w.w * P.trampled.w);
  albedo = mix(albedo, P.tracks.xyz * mix(0.85, 1.1, grain), w.z * P.tracks.w);
  // Soot is uneven: ash grey and char black in flecks and drifts.
  let ash = mix(0.6, 2.4, fleck * 0.6 + valueNoise(xy * 0.7 + vec2f(4.4, 0.3)) * 0.4);
  albedo = mix(albedo, P.scorch.xyz * ash, w.y * P.scorch.w);
  let ring = scar.relief.z * P.ejecta.w;
  albedo = mix(albedo, P.ejecta.xyz * mix(0.8, 1.15, fleck), ring);
  let bowl = w.x * P.crater.w;
  // The deeper, the darker: the floor sees less sky than the lip.
  let cavity = mix(1.0, ${CRATER_CAVITY}, saturate(scar.relief.w));
  albedo = mix(albedo, P.crater.xyz * mix(0.8, 1.15, fleck) * cavity, bowl);
  // Churned soil and ash are fully rough: a tilted wall never catches sky.
  roughness = mix(roughness, 1.0, max(w.y * P.scorch.w, max(bowl, ring)));
  return vec4f(albedo, roughness);
}`)
  .$uses({ terrainLayout, valueNoise, ScarSample });

/** A terrain normal `n` tilted by the scars' height slope (`relief.xy`). */
export const scarredNormal = tgpu.fn(
  [d.vec3f, ScarSample],
  d.vec3f,
)(/* wgsl */ `(n: vec3f, scar: ScarSample) -> vec3f {
  // At most 40° of tilt: steeper walls turn to the blue sky and read as haze.
  let slope = scar.relief.xy;
  let steep = length(slope);
  let capped = slope * min(1.0, ${MAX_SLOPE} / max(steep, 1e-6));
  return normalize(n - vec3f(capped, 0.0) * n.z);
}`);

const linear = (c: Rgb): [number, number, number] => [c[0] ** 2.2, c[1] ** 2.2, c[2] ** 2.2];

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The terrain's GPU tables, rebuilt whole when the world is set. */
export function createTerrainSource(root: Root, registry: GpuRegistry) {
  const params = registry.own(root.createBuffer(TerrainParams).$usage("uniform"));
  // Each table holds at least one record: a storage binding cannot be empty.
  const nodeBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(PlotNode, Math.max(1, n))).$usage("storage");
  const plotBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(PlotRecord, Math.max(1, n))).$usage("storage");
  const roadBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(RoadSegment, Math.max(1, n))).$usage("storage");
  const rectBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(d.vec4f, Math.max(1, n))).$usage("storage");
  const nodes: GpuSlot<ReturnType<typeof nodeBuffer>> = registry.slot();
  const plots: GpuSlot<ReturnType<typeof plotBuffer>> = registry.slot();
  const roads: GpuSlot<ReturnType<typeof roadBuffer>> = registry.slot();
  const rects: GpuSlot<ReturnType<typeof rectBuffer>> = registry.slot();
  const scarParams = registry.own(root.createBuffer(ScarParams).$usage("uniform"));
  const scarSampler = registry.device.createSampler({
    label: "ground-scars",
    magFilter: "linear",
    minFilter: "linear",
    addressModeU: "clamp-to-edge",
    addressModeV: "clamp-to-edge",
  });
  const scarLook = {
    full: d.vec4f(1, 1, 1, 1),
    crater: d.vec4f(),
    ejecta: d.vec4f(),
    scorch: d.vec4f(),
    tracks: d.vec4f(),
    trampled: d.vec4f(),
    relief: d.vec4f(),
    grass: d.vec4f(),
  };
  const writeScarParams = () => {
    const { cols, rows, cellM } = scars.stats();
    const on = cols > 0 && rows > 0;
    scarParams.write({
      grid: d.vec4f(on ? 1 / (cols * cellM) : 0, on ? 1 / (rows * cellM) : 0, cellM, on ? 1 : 0),
      ...scarLook,
    });
  };
  const groupOf = () =>
    root.createBindGroup(terrainLayout, {
      params,
      nodes: nodes.current!,
      plots: plots.current!,
      roads: roads.current!,
      rects: rects.current!,
      scarParams,
      scars: scars.texture.createView(),
      scarSampler,
    });
  // A new scar texture (a new grid) needs a new group, once the world is set.
  const scars = createScarTexture(registry, () => {
    writeScarParams();
    if (nodes.current) source.group = groupOf();
  });

  const source = {
    group: null as unknown as ReturnType<typeof groupOf>,
    /** Follow the side's learned ground (null: none); true when the scars
     *  changed, so what grows on them must regrow. */
    setGround(ground: GroundMarks | null): boolean {
      return scars.sync(ground);
    },
    scarStats: () => scars.stats(),
    set(surface: TerrainSurface) {
      const { plots: tree, site, biome } = surface;
      nodes.set(nodeBuffer(tree.nodes.length / NODE_FLOATS)).write(packNodes(tree));
      plots.set(plotBuffer(tree.plots.length)).write(packPlots(surface));
      const roadCount = site.roads.length / site.roadStride;
      const roadBytes = new Float32Array(Math.max(1, roadCount) * (ROAD_BYTES / 4));
      for (let r = 0; r < roadCount; r++) {
        roadBytes.set(site.roads.subarray(r * site.roadStride, r * site.roadStride + 5), r * 8);
      }
      roads.set(roadBuffer(roadCount)).write(roadBytes.buffer);
      const forestCount = site.forests.length / RECT_FLOATS;
      const waterCount = site.water.length / RECT_FLOATS;
      const rectBytes = new Float32Array(Math.max(1, forestCount + waterCount) * 4);
      [site.forests, site.water].forEach((list, k) => {
        for (let r = 0; r < list.length / RECT_FLOATS; r++) {
          const [x, y, w, h] = list.subarray(r * RECT_FLOATS, r * RECT_FLOATS + 4);
          rectBytes.set([x, y, x + w, y + h], (k * forestCount + r) * 4);
        }
      });
      rects.set(rectBuffer(forestCount + waterCount)).write(rectBytes.buffer);
      const rules = biome.field_rules;
      const one = (key: string) => linear(biome.palettes[key][0]);
      params.write({
        region: d.vec4f(...tree.region),
        counts: d.vec4u(roadCount, forestCount, waterCount, tree.plots.length),
        shape: d.vec4f(
          rules.edge_warp_m,
          1 / rules.edge_warp_scale_m,
          1 / rules.mottle_scale_m[0],
          1 / rules.mottle_scale_m[1],
        ),
        verge: d.vec4f(...one(biome.verge.palette), biome.verge.width_m / 2),
        feathers: d.vec4f(
          biome.verge.feather_m,
          biome.road.feather_m,
          rules.size_m[0] * PLOT_PIXELS_FADE[0],
          rules.size_m[0] * PLOT_PIXELS_FADE[1],
        ),
        road: d.vec4f(...one(biome.road.palette), biome.road.roughness),
        roadDetail: d.vec4f(biome.road.mottle, 0, 0, 0),
        ...forestParams(biome.forest_floor, biome.palettes[biome.forest_floor.palette]),
        waterBed: d.vec4f(...one("water_bed"), 0),
        distant: d.vec4f(...one("distant"), 0),
      });
      const s = biome.scars;
      const mark = (m: ScarMark) => d.vec4f(...one(m.palette), m.strength);
      scarLook.full = d.vec4f(
        ...(SCAR_CHANNELS.map((c) => 255 / s[c].full) as [number, number, number, number]),
      );
      scarLook.crater = mark(s.crater);
      scarLook.ejecta = d.vec4f(...one(s.crater.ejecta_palette), s.crater.ejecta);
      scarLook.scorch = mark(s.scorch);
      scarLook.tracks = mark(s.tracks);
      scarLook.trampled = mark(s.trampled);
      scarLook.relief = d.vec4f(s.crater.relief_m, s.crater.rim, 0, 0);
      scarLook.grass = d.vec4f(s.grass.thin, s.grass.tracks_thin, s.grass.flatten, 0);
      writeScarParams();
      source.group = groupOf();
    },
  };
  return source;
}
export type TerrainSource = ReturnType<typeof createTerrainSource>;

/** The forest floor's uniform fields: its palette's litter, moss and humus. */
function forestParams(floor: ForestFloor, [litter, moss, humus]: readonly Rgb[]) {
  return {
    forestLitter: d.vec4f(...linear(litter), 0),
    forestMoss: d.vec4f(...linear(moss), 0),
    forestHumus: d.vec4f(...linear(humus), 0),
    forestDetail: d.vec4f(1 / floor.patch_m, floor.mottle, floor.roughness, floor.roots),
    forestVerge: d.vec4f(
      floor.verge_m,
      floor.verge_warp_m,
      1 / floor.verge_warp_scale_m,
      1 / floor.roots_m,
    ),
    forestDapple: d.vec4f(1 / floor.dapple.size_m, floor.dapple.share, floor.dapple.sun, 0),
  };
}

function packNodes(tree: PlotTree): ArrayBuffer {
  const count = tree.nodes.length / NODE_FLOATS;
  const bytes = new ArrayBuffer(Math.max(1, count) * NODE_BYTES);
  const f = new Float32Array(bytes);
  const i = new Int32Array(bytes);
  for (let k = 0; k < count; k++) {
    const o = k * NODE_FLOATS;
    f[k * 8] = tree.nodes[o];
    f[k * 8 + 1] = tree.nodes[o + 1];
    f[k * 8 + 2] = tree.nodes[o + 2];
    i[k * 8 + 4] = tree.nodes[o + 3];
    i[k * 8 + 5] = tree.nodes[o + 4];
  }
  return bytes;
}

function packPlots({ plots: tree, biome }: TerrainSurface): ArrayBuffer {
  const f = new Float32Array(Math.max(1, tree.plots.length) * (PLOT_BYTES / 4));
  tree.plots.forEach((plot, k) => {
    const kind = biome.plots[plot.kind];
    f.set(
      [
        ...linear(plot.colour),
        kind.roughness,
        plot.across[0],
        plot.across[1],
        kind.furrow_m,
        kind.furrow_contrast,
        kind.mottle,
        plot.kind,
      ],
      k * 12,
    );
  });
  return f.buffer;
}
