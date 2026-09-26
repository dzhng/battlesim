// The biome's ground material on the GPU: per pixel, the plot under the
// point (walking the plot split), its rows and painterly value noise, the
// verge along every plot edge, then the forest floor, the road and the water
// bed exactly where the simulation's rules put them. It returns linear albedo
// and roughness; lighting, shadow and FogTerm stay with the world pass.
//
// Only the plot edges wander (a small warp gives them a hand-cut line); the
// road, forest and water masks are the simulation's own shapes, so a road's
// 50% blend is on the road rule's edge. Detail finer than a pixel fades to
// its mean, so the patchwork neither shimmers nor changes value with zoom.
//
// Rewritten (reuse manifest, technique) from reading ~/dev/game
// battle-renderer/src/shaders/terrainMaterial.ts: the mottle, drift and
// feathered road-edge ideas; not its baked distance texture or rock layers.
import { tgpu, d, std } from "typegpu";
import { MAX_PLOT_DEPTH, NODE_FLOATS, type PlotTree } from "../terrain/plots";
import { RECT_FLOATS, type TerrainSurface } from "../terrain/terrainSurface";
import type { Rgb } from "../light/sceneLight";
import type { GpuRegistry, GpuSlot } from "./registry";

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
  forestFloor: d.vec4f,
  waterBed: d.vec4f,
  distant: d.vec4f,
});
const PlotNode = d.struct({ line: d.vec4f, children: d.vec4i });
const PlotRecord = d.struct({
  /** Linear rgb, roughness. */
  colour: d.vec4f,
  /** Across the rows (unit), row period metres, row contrast. */
  rows: d.vec4f,
  /** Mottle strength. */
  detail: d.vec4f,
});
const RoadSegment = d.struct({ ends: d.vec4f, half: d.vec4f });

export const terrainLayout = tgpu.bindGroupLayout({
  params: { uniform: TerrainParams, visibility: ["fragment"] },
  nodes: {
    storage: (n: number) => d.arrayOf(PlotNode, n),
    access: "readonly",
    visibility: ["fragment"],
  },
  plots: {
    storage: (n: number) => d.arrayOf(PlotRecord, n),
    access: "readonly",
    visibility: ["fragment"],
  },
  roads: {
    storage: (n: number) => d.arrayOf(RoadSegment, n),
    access: "readonly",
    visibility: ["fragment"],
  },
  /** Forest rects, then water rects: minX, minY, maxX, maxY. */
  rects: {
    storage: (n: number) => d.arrayOf(d.vec4f, n),
    access: "readonly",
    visibility: ["fragment"],
  },
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
const valueNoise = tgpu.fn(
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

/** Linear albedo and roughness of the ground at `world`, with `footprint`
 *  the metres one pixel spans there. */
export const groundSurface = tgpu.fn(
  [d.vec3f, d.f32],
  d.vec4f,
)((world, footprint) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const xy = world.xy;
  const aa = footprint * 0.5;

  // The plot under the (warped) point, and the distance to its edge.
  const warp = d.vec2f(
    valueNoise(std.mul(xy, params.shape.y)) - 0.5,
    valueNoise(std.add(std.mul(xy, params.shape.y), d.vec2f(19.7, 5.3))) - 0.5,
  );
  const q = std.add(xy, std.mul(warp, params.shape.x * 2));
  const region = params.region;
  const inRegion = rectInside(q, region);
  let edge = inRegion;
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
  const plot = terrainLayout.$.plots[leaf];
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
  // The verge along every plot edge and beside the road. It holds its full
  // colour right up to the edge, so neighbouring plots meet in one colour and
  // a plot boundary never steps from pixel to pixel; it fades out over a
  // pixel at least.
  const vergeEdge = std.min(edge, std.max(-road, 0));
  const vergeHalf = params.verge.w;
  const verge =
    1 - std.smoothstep(vergeHalf, vergeHalf + std.max(params.feathers.x, footprint), vergeEdge);
  albedo = std.mix(albedo, std.mul(params.verge.xyz, 1 + 0.18 * noise), verge);
  roughness = std.mix(roughness, 0.95, verge);

  // Past the patchwork, and where plots shrink to a few pixels, the distant
  // land: its mean colour.
  const distant = std.max(
    1 - std.smoothstep(0, DISTANT_FADE_M, inRegion),
    std.smoothstep(params.feathers.z, params.feathers.w, footprint),
  );
  albedo = std.mix(albedo, params.distant.xyz, distant);

  // The forest floor, inside the simulation's forest rects.
  let forest = d.f32(0);
  for (let i = d.u32(0); i < params.counts.y; i++) {
    const inside = rectInside(xy, terrainLayout.$.rects[i]);
    forest = std.max(forest, std.smoothstep(-aa, aa, inside));
  }
  albedo = std.mix(albedo, std.mul(params.forestFloor.xyz, 1 + 0.3 * noise), forest);
  roughness = std.mix(roughness, 0.97, forest);

  // The road surface, over all but water.
  const roadFeather = std.max(params.feathers.y, footprint) * 0.5;
  const onRoad = std.smoothstep(-roadFeather, roadFeather, road);
  const surface = std.mul(params.road.xyz, 1 + params.roadDetail.x * noise);
  albedo = std.mix(albedo, surface, onRoad);
  roughness = std.mix(roughness, params.road.w, onRoad);

  // The water bed, under the simulation's water rects (water wins over road).
  let bed = d.f32(0);
  for (let i = d.u32(0); i < params.counts.z; i++) {
    const inside = rectInside(xy, terrainLayout.$.rects[params.counts.y + i]);
    bed = std.max(bed, std.smoothstep(-aa, aa, inside));
  }
  albedo = std.mix(albedo, params.waterBed.xyz, bed);
  return d.vec4f(std.max(albedo, d.vec3f(0)), roughness);
});

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
  const groupOf = () =>
    root.createBindGroup(terrainLayout, {
      params,
      nodes: nodes.current!,
      plots: plots.current!,
      roads: roads.current!,
      rects: rects.current!,
    });

  const source = {
    group: null as unknown as ReturnType<typeof groupOf>,
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
        forestFloor: d.vec4f(...one("forest_floor"), 0),
        waterBed: d.vec4f(...one("water_bed"), 0),
        distant: d.vec4f(...one("distant"), 0),
      });
      source.group = groupOf();
    },
  };
  return source;
}
export type TerrainSource = ReturnType<typeof createTerrainSource>;

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
      ],
      k * 12,
    );
  });
  return f.buffer;
}
