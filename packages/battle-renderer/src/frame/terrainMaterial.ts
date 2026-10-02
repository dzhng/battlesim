// The biome's ground material on the GPU: per pixel, the plot under the
// point (walking the plot split), its rows and painterly value noise, the
// verge along every plot edge, then the forest floor, the road and the water
// bed exactly where the simulation's rules put them. It returns linear albedo
// and roughness; lighting, shadow and FogTerm stay with the world pass.
//
// The plot edges wander (a small warp gives them a hand-cut line); the road
// and water masks are the simulation's own shapes, so a road's 50% blend is
// on the road rule's edge. The forest floor (leaf litter, moss, humus, roots)
// covers the simulation's forest shapes and meets the field across a ragged
// verge on each shape's edge: the shape stays the rule, only its
// look is softened. Under the crowns, `groundDapple` lets sun flecks through.
// Detail finer than a pixel fades to its mean, so the patchwork neither
// shimmers nor changes value with zoom.
//
// Rewritten from reading ~/dev/game
// battle-renderer/src/shaders/terrainMaterial.ts: the mottle, drift and
// feathered road-edge ideas; not its baked distance texture or rock layers.
import { tgpu, d, std } from "typegpu";
import { pcgHash } from "../shaders/pcgHash";
import { MAX_PLOT_DEPTH, NODE_FLOATS, type PlotTree } from "../terrain/plots";
import type { TerrainSite, TerrainSurface } from "../terrain/terrainSurface";
import {
  buildSurfaceField,
  SURFACE_CELL_WORDS,
  SURFACE_FLOATS,
  SURFACE_KIND_SHIFT,
  SURFACE_LEVEL_WORDS,
  SURFACE_NEW_SHAPE,
  SURFACE_RECORD_MASK,
  SURFACE_RECT,
  SURFACE_STROKE,
  SURFACE_TRIANGLE,
  type SurfaceReach,
} from "../terrain/surfaceField";
import { CUT_A, CUT_B } from "../terrain/strokes";
import { SURFACE_AREA_KINDS } from "../terrain/surfaces";
import {
  CLASS_EDGE,
  CLASS_FOREST_SHIFT,
  CLASS_HASH_MASK,
  CLASS_KIND_MAX,
  CLASS_KIND_SHIFT,
  CLASS_STEPS_PER_M,
} from "../terrain/groundClasses";
import { GRASS_EDGE_M } from "../terrain/grassField";
import { longestBank } from "../terrain/rivers";
import {
  SCAR_CHANNELS,
  type Biome,
  type ForestFloor,
  type Road,
  type ScarMark,
} from "../terrain/biome";
import { pick } from "@packages/renderer-core/src/kindTable";
import type { Rgb } from "../light/sceneLight";
import type { GpuRegistry, GpuSlot } from "./registry";
import { groundFilterReady, scarFilterQuanta, scarFilterPosition } from "./scarFilter";
import {
  createScarTexture,
  SCAR_UNIFORM,
  SCAR_DENSE,
  SCAR_KEY_MASK,
  SCAR_REGION_CELLS,
  type ScarRegion,
  type GroundMarks,
} from "./scarTexture";

/** The paved kinds a map can hold: a row of the look table each. */
const ROAD_KINDS = SURFACE_AREA_KINDS.length;
/** How one paved kind is drawn (`biome.roads`). */
const RoadLook = d.struct({
  /** The surface: linear rgb, roughness. */
  core: d.vec4f,
  /** What its patches wear toward (linear rgb, at the surface's luminance),
   *  and how far a patch goes to it. */
  worn: d.vec4f,
  /** Edge feather metres, 1 / patch size, grain strength, 1 / grain size. */
  shape: d.vec4f,
  /** How far a later kind's road is carried onto this one where they join, metres; then
   *  unused. */
  join: d.vec4f,
});

const TerrainParams = d.struct({
  /** The plot region: minX, minY, maxX, maxY. */
  region: d.vec4f,
  /** The surface field's grid (`terrain/surfaceField.ts`): its low corner,
   *  1 / the finest cell's side, the widest pixel the finest level serves. */
  field: d.vec4f,
  /** The finest level's cells across and up, and the levels. */
  fieldGrid: d.vec4u,
  /** Edge warp metres, 1 / warp scale, 1 / fine mottle scale, 1 / broad mottle scale. */
  shape: d.vec4f,
  /** Linear rgb, verge half width. */
  verge: d.vec4f,
  /** Verge feather, unused, and the pixel footprints (metres) over which
   *  plots give way to the distant colour. */
  feathers: d.vec4f,
  /** Each paved kind's look, by its tag (`SURFACE_AREA_KINDS`). */
  roads: d.arrayOf(RoadLook, ROAD_KINDS),
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
  /** The water bed's colour (linear rgb), and how far past a bank's top a
   *  ground triangle can carry its tilt, in metres: a triangle's diagonal. */
  waterBed: d.vec4f,
  /** The water surface's colour (linear rgb) over deep water, and its opacity there. */
  water: d.vec4f,
  /** The banks' wet soil (linear rgb) and the shore's width in metres. */
  shore: d.vec4f,
  distant: d.vec4f,
  /** 1 draws the ground's classes (`groundClasses`) in place of its lit
   *  colour: the "ground-classes" frame view. Then unused. */
  view: d.vec4u,
});
/** The scar texture's grid and the biome's scar look (`biome.scars`). */
const ScarParams = d.struct({
  /** 1 / the grid's width and depth in metres, the cell's side, 1 when there
   *  is ground to draw (else 0). */
  grid: d.vec4f,
  /** Grid axes, sparse directory mask and reserved word. */
  pages: d.vec4u,
  /** Half-open world region for a bounded scar draw; zero span disables clipping. */
  clip: d.vec4f,
  /** Cache bounds in tile coordinates; misses inside use the exact common word. */
  cacheBounds: d.vec4u,
  defaultWord: d.vec4u,
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
const SurfaceRecord = d.struct({ ends: d.vec4f, detail: d.vec4f });

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
  /** The surface field's records: every paved, forest and water primitive. */
  surfaces: {
    storage: (n: number) => d.arrayOf(SurfaceRecord, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  /** The surface field's index: per level and cell, which records to read. */
  surfaceIndex: {
    storage: (n: number) => d.arrayOf(d.u32, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  scarParams: { uniform: ScarParams, visibility: ["fragment", "compute"] },
  /** The side's learned ground (`scarTexture.ts`): crater, scorch, tracks,
   *  trampled per cell. */
  scarPages: { texture: d.texture2d(d.u32), visibility: ["fragment", "compute"] },
  scars: { texture: d.texture2dArray(d.f32), visibility: ["fragment", "compute"] },
});

/** The patchwork fades to the distant colour over this far inside its region's edge. */
const DISTANT_FADE_M = 600;
/** Plots give way to the distant colour as the smallest plot's side shrinks
 *  from 6 to 2 pixels (as fractions of it, the pixel footprint). */
const PLOT_PIXELS_FADE = [1 / 6, 1 / 2] as const;
/** The mottle's weights (times each plot kind's `mottle`): the dry strips
 *  shift hue only; the fine noise is mostly brightness, a little hue. */
const MOTTLE_STRIP_HUE = 0.6;
const MOTTLE_FINE_VALUE = 0.45;
const MOTTLE_FINE_HUE = 0.4;
/** The strips are this many times longer along a plot's rows than across
 *  them, and cover the share of the plot where their noise passes this. */
const MOTTLE_STREAK_ASPECT = 16;
const MOTTLE_STRIP_CUT = 0.58;
/** A strip's edge is never sharper than this many metres, nor than a pixel;
 *  its slope is read over this step of the noise's lattice. */
const MOTTLE_STRIP_EDGE_M = 0.3;
const MOTTLE_SLOPE_STEP = 0.05;
/** How a hue shift scales linear rgb before its luminance is restored: toward
 *  ochre, as drier grass or crop. Never toward blue, as a shadow under the
 *  sky is. */
const MOTTLE_DRY = d.vec3f(0.7, 0, -0.9);
const NODE_BYTES = 32;
const PLOT_BYTES = 48;

/** Value noise on a unit lattice, in [0, 1]: an integer hash per lattice
 *  corner (no sin-hash, which loses precision kilometres out), smoothly
 *  interpolated. */
export const valueNoise = tgpu
  .fn(
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
    let s = pcgHash(((ix + (k & 1u)) * 1597334677u) ^ ((iy + (k >> 1u)) * 3812015801u));
    h[k] = f32(s) * (1.0 / 4294967295.0);
  }
  return mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
}`)
  .$uses({ pcgHash });

/** The painterly variation inside a plot, `(value, dry)`. `dry` (0 or more)
 *  is the hue: strips laid along the plot's rows (`across` is the unit
 *  vector across them), drawn afresh in every plot (`plot` its index), with
 *  firm edges, where the crop is drier; `groundTint` turns it into ochre at
 *  unchanged luminance. `value` (around 0) is the brightness: fine value
 *  noise only, fading as it drops below a pixel (`footprint` is metres per
 *  pixel). Nothing broad is darker or cooler than its plot: a large soft
 *  darker patch reads as a cloud's shadow with nothing to cast it. */
const mottle = tgpu.fn(
  [d.vec2f, d.f32, d.vec2f, d.f32],
  d.vec2f,
)((xy, footprint, across, plot) => {
  "use gpu";
  const shape = terrainLayout.$.params.shape;
  const along = d.vec2f(-across.y, across.x);
  const lane = d.vec2f(
    std.dot(xy, along) * shape.w,
    std.dot(xy, across) * shape.w * MOTTLE_STREAK_ASPECT,
  );
  const seed = d.vec2f(std.fract(plot * 0.6180339) * 997, std.fract(plot * 0.7548776) * 991);
  const at = std.add(lane, seed);
  const streak = valueNoise(at);
  // The strip's edge is where the noise crosses the cut. Its distance in
  // metres (the noise over its slope, by finite differences) gives an edge
  // a pixel wide however slowly the noise crosses, so no strip fades softly
  // out as a shadow's penumbra does.
  const slope = d.vec2f(
    ((valueNoise(std.add(at, d.vec2f(MOTTLE_SLOPE_STEP, 0))) - streak) / MOTTLE_SLOPE_STEP) *
      shape.w,
    ((valueNoise(std.add(at, d.vec2f(0, MOTTLE_SLOPE_STEP))) - streak) / MOTTLE_SLOPE_STEP) *
      shape.w *
      MOTTLE_STREAK_ASPECT,
  );
  const inside = (streak - MOTTLE_STRIP_CUT) / std.max(std.length(slope), 1e-4);
  const edge = std.max(footprint, MOTTLE_STRIP_EDGE_M) * 0.5;
  const strip = std.smoothstep(-edge, edge, inside);
  const fine = valueNoise(std.add(std.mul(xy, shape.z), d.vec2f(37.1, 11.3))) - 0.5;
  const fineShown = 1 - std.smoothstep(0.25, 1, footprint * shape.z);
  return d.vec2f(
    fine * MOTTLE_FINE_VALUE * fineShown,
    strip * MOTTLE_STRIP_HUE + std.max(fine, 0) * MOTTLE_FINE_HUE * fineShown,
  );
});

/** `albedo` shifted toward ochre by `dry` (0 or more) at its own Rec. 709
 *  luminance. */
const groundTint = tgpu.fn(
  [d.vec3f, d.f32],
  d.vec3f,
)((albedo, dry) => {
  "use gpu";
  const luma = d.vec3f(0.2126, 0.7152, 0.0722);
  const tinted = std.mul(albedo, std.add(d.vec3f(1), std.mul(MOTTLE_DRY, dry)));
  return std.mul(tinted, std.dot(albedo, luma) / std.max(std.dot(tinted, luma), 1e-5));
});

/** How far `xy` lies inside rect `r` (negative outside). */
const rectInside = tgpu.fn(
  [d.vec2f, d.vec4f],
  d.f32,
)((xy, r) => {
  "use gpu";
  return std.min(std.min(xy.x - r.x, r.z - xy.x), std.min(xy.y - r.y, r.w - xy.y));
});

/** Distance to a closed segment, for polygon edges and forest strokes. Road
 * strokes keep their own inline form: changing its float order would move
 * the village's road edges by an ULP. */
const polygonEdgeDistance = tgpu.fn(
  [d.vec2f, d.vec2f, d.vec2f],
  d.f32,
)(/* wgsl */ `(p:vec2f,a:vec2f,b:vec2f)->f32 {
 let ab=b-a;let len2=dot(ab,ab);var t=0.0;if(len2>0.0){t=clamp(dot(p-a,ab)/len2,0.0,1.0);}return length(p-(a+ab*t));
}`);

/** A stroke stretch's `inside` (its half width less the distance to it)
 *  once its cut ends are taken: past an end the stretch is cut square at, the
 *  distance to that flat end; short of it, no deeper than the end is near.
 *  `detail` is the stretch's `(half width, tag, cuts)`; `terrain/strokes.ts`
 *  `strokeInside` is this on the CPU. An uncut stretch keeps `inside` as its
 *  caller computed it. */
const strokeCutInside = tgpu.fn(
  [d.vec2f, d.vec4f, d.vec4f, d.f32],
  d.f32,
)(/* wgsl */ `(xy:vec2f,ends:vec4f,detail:vec4f,inside:f32)->f32 {
 let cuts=u32(detail.z);
 if(cuts==0u){return inside;}
 let a=ends.xy;let ab=ends.zw-a;let len=length(ab);
 let along=dot(xy-a,ab)/len;
 let past=max(select(-1e9,-along,(cuts&${CUT_A}u)!=0u),select(-1e9,along-len,(cuts&${CUT_B}u)!=0u));
 if(past<=0.0){return min(inside,-past);}
 let rel=xy-a;
 let aside=max(abs(rel.x*ab.y-rel.y*ab.x)/len-detail.x,0.0);
 return -length(vec2f(aside,past));
}`);

const polygonTriangleInside = tgpu.fn(
  [d.vec2f, d.vec2f, d.vec2f, d.vec2f],
  d.bool,
)(/* wgsl */ `(xy:vec2f,a:vec2f,b:vec2f,c:vec2f)->bool {
 let ab=b-a;let bc=c-b;let ca=a-c;
 let area=dot(vec2f(-ab.y,ab.x),c-a);
 let s0=dot(vec2f(-ab.y,ab.x),xy-a);let s1=dot(vec2f(-bc.y,bc.x),xy-b);let s2=dot(vec2f(-ca.y,ca.x),xy-c);
 return area!=0.0&&((min(s0,min(s1,s2))>=0.0)||(max(s0,max(s1,s2))<=0.0));
}`);

/** The surface field's cell under `xy` for a pixel `footprint` metres wide:
 *  where its paved, forest and water lists start in `surfaceIndex`, and where
 *  they end. A wider pixel reads its distances farther out (its feathers are
 *  a pixel wide), so it takes a level whose cells list farther; past the last
 *  level it reads that one. `terrain/surfaceField.ts` `surfaceCell` is this
 *  on the CPU. */
export const groundCell = tgpu
  .fn(
    [d.vec2f, d.f32],
    d.vec4u,
  )(/* wgsl */ `(xy:vec2f,footprint:f32)->vec4u {
 let P=terrainLayout.$.params;
 var level=0u;var widest=P.field.w;
 while(footprint>widest&&level+1u<P.fieldGrid.z){widest*=2.0;level++;}
 let shift=terrainLayout.$.surfaceIndex[level*${SURFACE_LEVEL_WORDS}u+1u];
 let finest=clamp(vec2i(floor((xy-P.field.xy)*P.field.z)),vec2i(0),vec2i(P.fieldGrid.xy)-1);
 let cell=vec2u(finest)>>vec2u(shift);
 let cols=((P.fieldGrid.x-1u)>>shift)+1u;
 let at=terrainLayout.$.surfaceIndex[level*${SURFACE_LEVEL_WORDS}u]+(cell.y*cols+cell.x)*${SURFACE_CELL_WORDS}u;
 return vec4u(terrainLayout.$.surfaceIndex[at],terrainLayout.$.surfaceIndex[at+1u],terrainLayout.$.surfaceIndex[at+2u],terrainLayout.$.surfaceIndex[at+3u]);
}`)
  .$uses({ terrainLayout });

/** How far `xy` lies inside the deepest forest shape of `cell` (negative
 *  outside). Rectangles keep their original arithmetic; a polygon's triangles
 *  are only membership, its exposed ring the distance, never a diagonal. */
export const groundForest = tgpu
  .fn(
    [d.vec2f, d.vec4u],
    d.f32,
  )(/* wgsl */ `(xy:vec2f,cell:vec4u)->f32 {
 var forest=-1e9;var distance=-1e9;var nearest=1e9;var inside=false;
 for(var i=cell.y;i<cell.z;i++){
  let entry=terrainLayout.$.surfaceIndex[i];
  let record=terrainLayout.$.surfaces[entry&${SURFACE_RECORD_MASK}u];
  let kind=entry>>${SURFACE_KIND_SHIFT}u;
  if((entry&${SURFACE_NEW_SHAPE}u)!=0u){
   forest=max(forest,max(distance,select(-nearest,nearest,inside)));
   distance=-1e9;nearest=1e9;inside=false;
  }
  if(kind==${SURFACE_RECT}u){forest=max(forest,rectInside(xy,record.ends));}
  else if(kind==${SURFACE_STROKE}u){distance=max(distance,strokeCutInside(xy,record.ends,record.detail,record.detail.x-polygonEdgeDistance(xy,record.ends.xy,record.ends.zw)));}
  else if(kind==${SURFACE_TRIANGLE}u){inside=inside||polygonTriangleInside(xy,record.ends.xy,record.ends.zw,record.detail.xy);}
  else{nearest=min(nearest,polygonEdgeDistance(xy,record.ends.xy,record.ends.zw));}
 }
 return max(forest,max(distance,select(-nearest,nearest,inside)));
}`)
  .$uses({
    terrainLayout,
    rectInside,
    polygonEdgeDistance,
    polygonTriangleInside,
    strokeCutInside,
  });

/** How far `xy` lies inside the forest floor's drawn edge, in metres
 *  (negative outside), `forest` metres inside the simulation's forest. The
 *  drawn edge is a verge `verge_m` wide lying mostly outside the shape, its
 *  line wandering and broken into patches, so the wood meets the field
 *  without a ruled edge. The native shape stays the rule; this is only its look, and
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

/** The floor's edge is never sharper than this share of its verge. */
const FOREST_FEATHER = 0.1;

/** The forest floor's weight at `xy`: 1 inside its drawn edge, 0 outside,
 *  feathered over a pixel at least. */
const forestFloorWeight = tgpu.fn(
  [d.vec2f, d.f32, d.f32],
  d.f32,
)((xy, forest, footprint) => {
  "use gpu";
  const feather = std.max(footprint, terrainLayout.$.params.forestVerge.x * FOREST_FEATHER);
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
 *  gaps. Detail finer than a pixel fades to the flecks' mean. `cell` is the
 *  point's `groundCell`. */
export const groundDapple = tgpu.fn(
  [d.vec2f, d.f32, d.vec4u],
  d.f32,
)((xy, footprint, cell) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const forest = groundForest(xy, cell);
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

/** How far `xy` lies inside the paving of `cell`, kind by kind (by its tag;
 *  negative outside, far outside a kind the cell lists nothing of). A stroke
 *  counts for its own kind. The polygons are one union: membership comes from
 *  native triangles and only the exposed union boundary contributes
 *  feathering, so their distance goes to one kind, the one the point stands
 *  on (where kinds overlap, the earlier), or outside them all the nearest
 *  edge's. Stroke math retains its original order. */
export const groundPaved = tgpu
  .fn(
    [d.vec2f, d.vec4u],
    d.vec4f,
  )(/* wgsl */ `(xy:vec2f,cell:vec4u)->vec4f {
 var paved=vec4f(-1e9);var member=${ROAD_KINDS}u;var nearest=1e9;var edge=0u;
 for(var i=cell.x;i<cell.y;i++){
  let entry=terrainLayout.$.surfaceIndex[i];
  let seg=terrainLayout.$.surfaces[entry&${SURFACE_RECORD_MASK}u];
  let kind=entry>>${SURFACE_KIND_SHIFT}u;
  if(kind==${SURFACE_STROKE}u){
   let a=seg.ends.xy;let ab=seg.ends.zw-a;
   let t=clamp(dot(xy-a,ab)/max(dot(ab,ab),1e-6),0.0,1.0);
   let off=length(xy-(a+ab*t));let own=u32(seg.detail.y);
   paved[own]=max(paved[own],strokeCutInside(xy,seg.ends,seg.detail,seg.detail.x-off));
  }
  else if(kind==${SURFACE_TRIANGLE}u){
   if(polygonTriangleInside(xy,seg.ends.xy,seg.ends.zw,seg.detail.xy)){member=min(member,u32(seg.detail.z));}
  }
  else{
   let off=polygonEdgeDistance(xy,seg.ends.xy,seg.ends.zw);
   if(off<nearest){nearest=off;edge=u32(seg.detail.x);}
  }
 }
 if(member<${ROAD_KINDS}u){paved[member]=max(paved[member],nearest);}
 else if(nearest<1e9){paved[edge]=max(paved[edge],-nearest);}
 return paved;
}`)
  .$uses({ terrainLayout, polygonEdgeDistance, polygonTriangleInside, strokeCutInside });

/** Where `xy` sits in the ground's features, in metres:
 *  `(plot, edge, road, forest)`. `plot` is the plot's index (a whole
 *  number); `edge` the distance to its (warped) edge; `road` how far inside
 *  the nearest paved shape's edge (negative outside); `forest` how far inside the
 *  deepest forest shape (negative outside). The ground's colour and the grass
 *  both read it, so grass grows exactly where the ground says what it is.
 *  `cell` is the point's `groundCell` and `paved` its `groundPaved`: `road`
 *  and `forest` are exact as far as a pixel that wide reads them
 *  (`groundReach`), and keep their side beyond. */
export const groundSite = tgpu.fn(
  [d.vec2f, d.vec4u, d.vec4f],
  d.vec4f,
)((xy, cell, paved) => {
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
  const road = std.max(std.max(paved.x, paved.y), std.max(paved.z, paved.w));
  return d.vec4f(d.f32(leaf), edge, road, groundForest(xy, cell));
});

/** How far `xy` lies inside the water's edge (negative outside): the deepest
 *  any river stretch of `cell`, its `groundCell`, puts it. A stretch's half
 *  width runs linearly between its ends; the point reads it at the closest
 *  point of the centreline (`terrain/rivers.ts` `stretchInside`, and the
 *  simulation's `contract::river::section`). */
export const groundWater = tgpu
  .fn(
    [d.vec2f, d.vec4u],
    d.f32,
  )(/* wgsl */ `(xy:vec2f,cell:vec4u)->f32 {
 var bed=-1e9;
 for(var i=cell.z;i<cell.w;i++){
  let stretch=terrainLayout.$.surfaces[terrainLayout.$.surfaceIndex[i]&${SURFACE_RECORD_MASK}u];
  let a=stretch.ends.xy;let ab=stretch.ends.zw-a;let len2=dot(ab,ab);
  var t=0.0;if(len2>0.0){t=clamp(dot(xy-a,ab)/len2,0.0,1.0);}
  let half=stretch.detail.x+(stretch.detail.y-stretch.detail.x)*t;
  bed=max(bed,half-length(xy-(a+ab*t)));
 }
 return bed;
}`)
  .$uses({ terrainLayout });

/** The bank's shading ends over at least this far either side of its top, and
 *  gives way to the bed's this far inside the waterline, so neither is a
 *  ruled crease. */
const BANK_EASE_M = 0.75;
/** Stretches share the shading by how far inside each puts the point: one
 *  that puts it this many metres farther out counts 1/e as much. The nearest
 *  stretch alone would turn the bank's face at a stroke along the line where
 *  two stretches are as near, on the inside of every bend. */
const BANK_BLEND_M = 0.7;
/** A stretch's weight stops changing this far inside or outside its water's
 *  edge: beyond it `exp` leaves what a 32-bit float holds, on a river wider
 *  than 120 m or a bank cut longer than 70 m. */
const BANK_BLEND_REACH_M = 50;

/** How the ground at `xy` was cut for a river, as shading reads it: the
 *  bank's slope there (`xy`: rise per metre east and north), and how much the
 *  cut decides the shading normal (`z`: 1 on the bed, on the bank and as far
 *  past its top as a triangle that straddles the top reaches; 0 beyond).
 *  Only the bank slopes: the bed is lit flat, since through the water a bed
 *  lit as the V it is reads as one bright half and one dark, and past the
 *  bank's top the land is lit flat too.
 *  The grid's triangles cannot draw a bank a few metres wide, and lit by
 *  their own normals they read as steps along the river; lit by this, the
 *  bank is the round band the distance says. Shading only: the ground's
 *  height stays the simulation's, and the water's edge stays the nearest
 *  stretch's. `cell` is the point's `groundCell`, `footprint` the metres a
 *  pixel spans. */
export const groundBank = tgpu
  .fn(
    [d.vec2f, d.vec4u, d.f32],
    d.vec3f,
  )(/* wgsl */ `(xy:vec2f,cell:vec4u,footprint:f32)->vec3f {
 var sum=vec4f(0.0);
 for(var i=cell.z;i<cell.w;i++){
  let at=terrainLayout.$.surfaceIndex[i]&${SURFACE_RECORD_MASK}u;
  let stretch=terrainLayout.$.surfaces[at];
  let a=stretch.ends.xy;let ab=stretch.ends.zw-a;let len2=dot(ab,ab);
  var t=0.0;if(len2>0.0){t=clamp(dot(xy-a,ab)/len2,0.0,1.0);}
  let away=xy-(a+ab*t);let off=length(away);
  let half=stretch.detail.x+(stretch.detail.y-stretch.detail.x)*t;
  let inside=half-off;
  let grade=stretch.detail.z+(stretch.detail.w-stretch.detail.z)*t;
  let heights=terrainLayout.$.surfaces[at+1u].ends.xy;
  let run=(heights.x+(heights.y-heights.x)*t)/max(grade,1e-4);
  let ease=max(footprint,${BANK_EASE_M});
  let sloped=(1.0-smoothstep(run-ease,run+ease,-inside))*(1.0-smoothstep(0.0,ease,inside));
  let top=run+terrainLayout.$.params.waterBed.w;
  let weight=exp(clamp(inside,-${BANK_BLEND_REACH_M},${BANK_BLEND_REACH_M})/${BANK_BLEND_M});
  sum+=vec4f(away/max(off,1e-4)*grade*sloped,1.0-smoothstep(top,top+2.0*ease,-inside),1.0)*weight;
 }
 if(sum.w<=0.0){return vec3f(0.0);}
 return sum.xyz/sum.w;
}`)
  .$uses({ terrainLayout });

/** The wet bank is whole out to this share of the biome's shore width, and
 *  fades from there to the width's end. */
const SHORE_WHOLE = 0.55;

/** How wet and bare a bank is `water` metres inside the water's edge (its
 *  `groundWater`: negative on the bank): 1 at the edge, fading out by the
 *  biome's shore width; 0 farther out. The band follows the water's edge and
 *  nothing else, so it is as round as the river. The terrain paints it wet
 *  soil, and grass leaves it bare. */
export const groundShore = tgpu.fn(
  [d.f32],
  d.f32,
)((water) => {
  "use gpu";
  const reach = terrainLayout.$.params.shore.w;
  return 1 - std.smoothstep(reach * SHORE_WHOLE, reach, -water);
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

/** Noise in [0, 1] with no lattice to see: three octaves of value noise, each
 *  turned off the grid, read at a point a broader octave has pushed about.
 *  Cut at a level, a plain octave shows its cells as straight-edged blotches. */
const wanderNoise = tgpu
  .fn(
    [d.vec2f],
    d.f32,
  )(/* wgsl */ `(at:vec2f)->f32 {
 let push=vec2f(valueNoise(at*0.61+vec2f(11.3,47.1)),valueNoise(at*0.61+vec2f(83.9,5.7)))-0.5;
 let p=at+push*1.4;
 let a=valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y));
 let b=valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*2.03+vec2f(19.1,7.7));
 let c=valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*4.01+vec2f(3.3,41.9));
 return a*0.5+b*0.3+c*0.2;
}`)
  .$uses({ valueNoise });

/** An octave of the grain shows while its cells are wider than a pixel or
 *  two, as cells over the pixel. */
const GRAIN_FADE = [0.6, 1.2] as const;
/** The grain's stones: specks lighter and darker than the surface, where the
 *  finest octave passes these levels either way. */
const GRAIN_STONES = [0.7, 0.76] as const;

/** A road surface's grain around 0, at `p` in grain sizes with a pixel
 *  `pixel` of them wide: lumps at three scales and, finest, stones as crisp
 *  specks. Each scale fades out on its own as it nears a pixel, so the
 *  surface coarsens with distance and never crawls or steps. */
const roadGrain = tgpu
  .fn(
    [d.vec2f, d.f32],
    d.f32,
  )(/* wgsl */ `(p:vec2f,pixel:f32)->f32 {
 let shown=vec4f(1.0)-smoothstep(vec4f(${GRAIN_FADE[0]}),vec4f(${GRAIN_FADE[1]}),pixel*vec4f(1.0,2.03,4.01,8.3));
 var grain=0.0;
 if(shown.x>0.0){grain+=(valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y))-0.5)*0.25*shown.x;}
 if(shown.y>0.0){grain+=(valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*2.03+vec2f(19.1,7.7))-0.5)*0.35*shown.y;}
 if(shown.z>0.0){grain+=(valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*4.01+vec2f(3.3,41.9))-0.5)*0.4*shown.z;}
 if(shown.w>0.0){
  let stone=valueNoise(p*8.3+vec2f(71.9,13.7));
  grain+=(smoothstep(${GRAIN_STONES[0]},${GRAIN_STONES[1]},stone)-smoothstep(${GRAIN_STONES[0]},${GRAIN_STONES[1]},1.0-stone))*0.5*shown.w;
 }
 return grain;
}`)
  .$uses({ valueNoise });

/** A patch covers the share of a surface where its noise passes this. */
const PATCH_CUT = [0.52, 0.66] as const;

/** The roads at `xy` over the ground `under` (linear albedo, roughness):
 *  each paved kind as its own surface (`biome.roads`), feathered across its
 *  edge over a pixel at least, the later kinds under the earlier, so a track
 *  ends at the edge of the road it joins; there the track's earth is carried
 *  a way onto the road, thinning out. A surface is its colour, in patches a
 *  second hue at the same brightness, under its grain. `paved` is the
 *  point's `groundPaved`. */
const groundRoads = tgpu
  .fn(
    [d.vec2f, d.f32, d.vec4f, d.vec4f],
    d.vec4f,
  )(/* wgsl */ `(xy:vec2f,footprint:f32,paved:vec4f,under:vec4f)->vec4f {
 var surface=under;
 // How much of the ground here a later kind's road already covers.
 var laid=0.0;
 for(var k=${ROAD_KINDS}u;k>0u;k--){
  let look=terrainLayout.$.params.roads[k-1u];
  var feather=max(look.shape.x,footprint)*0.5;
  var inside=paved[k-1u];
  if(laid>0.0){
   // The blend starts at this road's edge and runs inward.
   feather=max(feather,look.join.x*0.5*laid);
   inside-=feather;
  }
  if(inside<=-feather){continue;}
  let hue=smoothstep(${PATCH_CUT[0]},${PATCH_CUT[1]},wanderNoise(xy*look.shape.y+vec2f(61.7,17.3)))*look.worn.w;
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  let core=mix(look.core.xyz,look.worn.xyz,hue)*(1.0+look.shape.z*grain);
  let on=smoothstep(-feather,feather,inside);
  surface=mix(surface,vec4f(core,look.core.w),on);
  laid=max(laid,on);
 }
 return surface;
}`)
  .$uses({ terrainLayout, wanderNoise, roadGrain });

/** Linear albedo and roughness of the ground at `xy` with site `site`,
 *  paving `paved` (`groundPaved`) and water depth `water`, `footprint` the
 *  metres one pixel spans there. */
export const groundColour = tgpu.fn(
  [d.vec2f, d.f32, d.vec4f, d.vec4f, d.f32],
  d.vec4f,
)((xy, footprint, site, paved, water) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const aa = footprint * 0.5;
  const plot = terrainLayout.$.plots[d.i32(site.x)];
  const variation = mottle(xy, footprint, plot.rows.xy, site.x);
  const noise = variation.x;
  let albedo = groundTint(
    std.mul(plot.colour.xyz, 1 + plot.detail.x * noise),
    plot.detail.x * variation.y,
  );
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

  // The forest floor, over the simulation's forest shapes and their verge.
  const forest = forestFloorWeight(xy, site.w, footprint);
  if (forest > 0) {
    albedo = std.mix(albedo, forestFloor(xy, footprint, noise), forest);
    roughness = std.mix(roughness, params.forestDetail.z, forest);
  }

  // The banks' wet soil, round the water.
  const wet = groundShore(water);
  albedo = std.mix(albedo, std.mul(params.shore.xyz, 1 + 0.15 * noise), wet);
  roughness = std.mix(roughness, 0.6, wet);

  // The roads, over all but water.
  const paving = groundRoads(xy, footprint, paved, d.vec4f(albedo, roughness));

  // The water bed, under the simulation's rivers (water wins over road).
  const bed = std.smoothstep(-aa, aa, water);
  albedo = std.mix(paving.xyz, params.waterBed.xyz, bed);
  return d.vec4f(std.max(albedo, d.vec3f(0)), paving.w);
});

/** Whether the frame draws the ground's classes in place of its lit colour. */
export const groundClassView = tgpu.fn(
  [],
  d.bool,
)(() => {
  "use gpu";
  return terrainLayout.$.params.view.x === 1;
});

/** A distance byte of the class view, over 255: `inside` metres inside an
 *  edge. Never 0, which is the view's "not ground". */
const classDistance = tgpu.fn(
  [d.f32],
  d.f32,
)((inside) => {
  "use gpu";
  return std.clamp(std.round(CLASS_EDGE - inside * CLASS_STEPS_PER_M), 1, 255) / 255;
});

/** What the ground at `xy` is, as the class view's three bytes over 255
 *  (`terrain/groundClasses.ts` decodes them), from the same site, water and
 *  footprint `groundColour` paints by: the distance outside the paving and
 *  outside the water, then whether the forest's floor is drawn there (inside
 *  the simulation's shape, or on the verge round it), the plot's kind and two
 *  hashed bits of its index. */
export const groundClasses = tgpu.fn(
  [d.vec2f, d.f32, d.vec4f, d.f32],
  d.vec3f,
)((xy, footprint, site, water) => {
  "use gpu";
  const plot = terrainLayout.$.plots[d.i32(site.x)];
  let forest = d.u32(0);
  if (site.w >= 0) {
    forest = d.u32(2);
  } else if (forestFloorWeight(xy, site.w, footprint) > 0.5) {
    forest = d.u32(1);
  }
  const kind = std.min(d.u32(plot.detail.y), d.u32(CLASS_KIND_MAX));
  const hash = pcgHash(d.u32(site.x)) & CLASS_HASH_MASK;
  const packed = (forest << CLASS_FOREST_SHIFT) | (kind << CLASS_KIND_SHIFT) | hash;
  return d.vec3f(classDistance(site.z), classDistance(water), d.f32(packed) / 255);
});

// The water surface's look (presentation, not rules): opaque over deep water,
// the bed showing through for the first metres inside the edge; smooth enough
// that sky and sun reflect in it, broken by two octaves of ripples. The
// surface ends on the simulation's water edge, feathered over a pixel: the
// ground meets the surface there, so nothing hides it and nothing steps.
const WATER_OPACITY = 0.78;
const WATER_SHORE_OPACITY = 0.35;
const WATER_SHORE_IN_M = 3;
export const WATER_ROUGHNESS = 0.14;
/** How much of its light water keeps in a shadow: the murk in it is lit by the
 *  sun, so a bridge or a tree shades it though the sky it reflects does not. */
export const WATER_SHADOW = 0.55;
const RIPPLE_LONG_M = 3.2;
const RIPPLE_SHORT_M = 0.9;
const RIPPLE_SLOPE = 0.09;
const RIPPLE_STEP_M = 0.25;

/** Ripple height at `xy`: two octaves of value noise, in about [0, 1.5]. */
const rippleHeight = tgpu.fn(
  [d.vec2f],
  d.f32,
)((xy) => {
  "use gpu";
  return (
    valueNoise(std.mul(xy, 1 / RIPPLE_LONG_M)) +
    0.5 * valueNoise(std.add(std.mul(xy, 1 / RIPPLE_SHORT_M), d.vec2f(17.3, 5.1)))
  );
});

/** The water surface at `xy`: linear colour and opacity (clear at the shore,
 *  murky out in the channel, `groundWater` giving how far in it lies), none
 *  outside the water's edge. `footprint` is the metres one pixel spans. */
export const waterSurface = tgpu.fn(
  [d.vec2f, d.f32],
  d.vec4f,
)((xy, footprint) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const water = groundWater(xy, groundCell(xy, footprint));
  const deep = std.smoothstep(0, WATER_SHORE_IN_M, water);
  const colour = std.mix(std.mul(params.waterBed.xyz, 0.55), params.water.xyz, deep);
  const edge = std.smoothstep(-footprint * 0.5, footprint * 0.5, water);
  return d.vec4f(colour, std.mix(WATER_SHORE_OPACITY, params.water.w, deep) * edge);
});

/** The water's normal at `xy`: small ripples, fading out where a pixel spans
 *  more than a ripple (`footprint` metres), so far water lies flat and calm. */
export const waterNormal = tgpu.fn(
  [d.vec2f, d.f32],
  d.vec3f,
)((xy, footprint) => {
  "use gpu";
  const ex = d.vec2f(RIPPLE_STEP_M, 0);
  const ey = d.vec2f(0, RIPPLE_STEP_M);
  const dx = rippleHeight(std.add(xy, ex)) - rippleHeight(std.sub(xy, ex));
  const dy = rippleHeight(std.add(xy, ey)) - rippleHeight(std.sub(xy, ey));
  const k = (RIPPLE_SLOPE / (2 * RIPPLE_STEP_M)) * (1 - std.smoothstep(0.15, 1.2, footprint));
  return std.normalize(d.vec3f(-dx * k, -dy * k, 1));
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

/** Directory entries are exact uniform words or resident nonuniform pages. */
const scarEntry = tgpu
  .fn(
    [d.vec2u],
    d.vec2u,
  )(/* wgsl */ `(tile:vec2u)->vec2u {
 let P=terrainLayout.$.scarParams;
 if(any(tile<P.cacheBounds.xy)||any(tile>P.cacheBounds.zw)){return vec2u(0u);}
 let tilesX=(P.pages.x+15u)/16u;let key=tile.y*tilesX+tile.x;
 var at=((key*0x9e3779b1u)^(key>>16u))&P.pages.z;
 loop {let dim=textureDimensions(terrainLayout.$.scarPages);let e=textureLoad(terrainLayout.$.scarPages,vec2i(i32(at%dim.x),i32(at/dim.x)),0).xy;
  if(e.x==0u){return vec2u(${SCAR_UNIFORM}u,P.defaultWord.x);}
  if((e.x&${SCAR_KEY_MASK}u)==key+1u){return e;}at=(at+1u)&P.pages.z;}
}`)
  .$uses({ terrainLayout });
const scarPoolWord = tgpu
  .fn(
    [d.u32],
    d.vec4f,
  )(/* wgsl */ `(offset:u32)->vec4f {
 let width=terrainLayout.$.scarParams.pages.w;
 return textureLoad(terrainLayout.$.scars,vec2i(i32(offset%width),i32(offset/width)),0,0);
}`)
  .$uses({ terrainLayout });
const scarNormalized = tgpu
  .fn(
    [d.u32],
    d.f32,
  )(/* wgsl */ `(q:u32)->f32 {
 let bytes=vec4u(round(scarPoolWord(q)*255.0));return bitcast<f32>(bytes.x|(bytes.y<<8u)|(bytes.z<<16u)|(bytes.w<<24u));
}`)
  .$uses({ scarPoolWord });
const scarWord = tgpu
  .fn(
    [d.u32],
    d.vec4f,
  )(/* wgsl */ `(word:u32)->vec4f {
 return vec4f(scarNormalized((word&255u)*16u),scarNormalized(((word>>8u)&255u)*16u),scarNormalized(((word>>16u)&255u)*16u),scarNormalized((word>>24u)*16u));
}`)
  .$uses({ scarNormalized });
const scarCell = tgpu
  .fn(
    [d.vec2i],
    d.vec4f,
  )(/* wgsl */ `(cell:vec2i)->vec4f {
 let P=terrainLayout.$.scarParams;let c=vec2u(clamp(cell,vec2i(0),vec2i(P.pages.xy)-1));let e=scarEntry(c/16u);
 if(e.x==0u){return vec4f(0.0);}if((e.x&${SCAR_UNIFORM}u)!=0u){return scarWord(e.y);}
 let local=(c.y%16u)*16u+c.x%16u;
 if((e.x&${SCAR_DENSE}u)!=0u){return scarPoolWord(e.y+local);}
 let offset=e.y>>8u;var lo=0u;var hi=e.y&255u;
 while(lo<hi){let mid=(lo+hi)/2u;let bytes=vec4u(round(scarPoolWord(offset+mid*2u)*255.0));let end=bytes.x+bytes.y*256u;if(end<=local){lo=mid+1u;}else{hi=mid;}}
 return scarPoolWord(offset+lo*2u+1u);
}`)
  .$uses({ terrainLayout, scarEntry, scarWord, scarPoolWord });
/** The original four-neighbor linear rule across every representation join. */
export const sampleGroundLinear = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(uv:vec2f)->vec4f {
 let P=terrainLayout.$.scarParams;let size=vec2f(P.pages.xy);let p=scarFilterPosition(uv,size);
 let cell=vec2i(floor(p));let f=fract(p);let e=scarEntry(vec2u(cell)/16u);
 if((e.x==0u||(e.x&${SCAR_UNIFORM}u)!=0u)&&all(vec2u(cell)%16u<vec2u(15u))){return scarWord(e.y);}
 let a=vec4u(round(scarCell(cell)*255.0));let b=vec4u(round(scarCell(cell+vec2i(1,0))*255.0));let c=vec4u(round(scarCell(cell+vec2i(0,1))*255.0));let other=vec4u(round(scarCell(cell+vec2i(1,1))*255.0));
 let q=scarFilterQuanta(a,b,c,other,f);
 return vec4f(scarNormalized(q.x),scarNormalized(q.y),scarNormalized(q.z),scarNormalized(q.w));
}`)
  .$uses({
    terrainLayout,
    scarEntry,
    scarWord,
    scarCell,
    scarNormalized,
    scarFilterQuanta,
    scarFilterPosition,
  });

/** The four channels at `uv`, reconstructed by a cubic B-spline over the
 *  1 m cells (four bilinear taps): a crater's footprint comes back round
 *  and smooth, so its thresholded edges are circles, not diamonds. */
export const sampleGroundCubic = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(uv: vec2f) -> vec4f {
  let size = vec2f(terrainLayout.$.scarParams.pages.xy);
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
  let a = sampleGroundLinear(vec2f(h0.x, h0.y));
  let b = sampleGroundLinear(vec2f(h1.x, h0.y));
  let c = sampleGroundLinear(vec2f(h0.x, h1.y));
  let e = sampleGroundLinear(vec2f(h1.x, h1.y));
  // Keep the original hardware cubic outer contraction explicit after inlining.
  return fma(vec4f(g0.y), g0.x * a + g1.x * b, g1.y * (g0.x * c + g1.x * e));
}`)
  .$uses({ terrainLayout, sampleGroundLinear });

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
  // Cheap reject: nothing marked within reach.
  let r = ${RIM_REACH_CELLS};
  let a = sampleGroundLinear(uv + texel * vec2f(-r, -r));
  let b = sampleGroundLinear(uv + texel * vec2f(r, -r));
  let c = sampleGroundLinear(uv + texel * vec2f(-r, r));
  let e = sampleGroundLinear(uv + texel * vec2f(r, r));
  let near = max(max(a, b), max(c, e));
  let s = sampleGroundCubic(uv);
  if (max(max(s.x, s.y), max(s.z, s.w)) + max(max(near.x, near.y), max(near.z, near.w)) <= 0.0) {
    return out;
  }
  let detail = 1.0 - smoothstep(0.5, 2.0, footprint / cell);
  let k = P.full;
  let depth = min(s.x * k.x, ${CRATER_DEPTH_CAP});
  let wide = min((a.x + b.x + c.x + e.x) * 0.25 * k.x, ${CRATER_DEPTH_CAP});
  // Slopes per metre of the crater field, from taps either side.
  let h = 0.75;
  let xp = sampleGroundLinear(uv + texel * vec2f(h, 0.0));
  let xm = sampleGroundLinear(uv - texel * vec2f(h, 0.0));
  let yp = sampleGroundLinear(uv + texel * vec2f(0.0, h));
  let ym = sampleGroundLinear(uv - texel * vec2f(0.0, h));
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
  .$uses({
    terrainLayout,
    valueNoise,
    sampleGroundLinear,
    sampleGroundCubic,
    crossing,
    ScarSample,
  });

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

/** Every terrain fragment belongs to exactly one cache region. */
export const scarRegionContains = tgpu.fn(
  [d.vec2f],
  d.bool,
)((world) => {
  "use gpu";
  const r = terrainLayout.$.scarParams.clip;
  return r.z <= r.x || (world.x >= r.x && world.y >= r.y && world.x < r.z && world.y < r.w);
});

const EMPTY_SCAR_REGIONS: readonly ScarRegion[] = [];

/** The terrain's GPU tables, rebuilt whole when the world is set. */
export function createTerrainSource(root: Root, registry: GpuRegistry) {
  const params = registry.own(root.createBuffer(TerrainParams).$usage("uniform"));
  // Each table holds at least one record: a storage binding cannot be empty.
  const nodeBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(PlotNode, Math.max(1, n))).$usage("storage");
  const plotBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(PlotRecord, Math.max(1, n))).$usage("storage");
  const surfaceBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(SurfaceRecord, Math.max(1, n))).$usage("storage");
  const indexBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(d.u32, Math.max(1, n))).$usage("storage");
  const nodes: GpuSlot<ReturnType<typeof nodeBuffer>> = registry.slot();
  const plots: GpuSlot<ReturnType<typeof plotBuffer>> = registry.slot();
  const surfaces: GpuSlot<ReturnType<typeof surfaceBuffer>> = registry.slot();
  const surfaceIndex: GpuSlot<ReturnType<typeof indexBuffer>> = registry.slot();
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
  const noClip: ScarRegion = [0, 0, 0, 0];
  let clip: ScarRegion = noClip;
  let regions: ScarRegion[] = [];
  let regionGrid = "";
  const writeScarParams = () => {
    const stats = scars.stats();
    const { cols, rows, cellM, poolWidth, directoryEntries } = stats;
    const on = cols > 0 && rows > 0;
    scarParams.write({
      grid: d.vec4f(on ? 1 / (cols * cellM) : 0, on ? 1 / (rows * cellM) : 0, cellM, on ? 1 : 0),
      pages: d.vec4u(cols, rows, directoryEntries - 1, poolWidth),
      clip: d.vec4f(...clip),
      cacheBounds: d.vec4u(...stats.cacheBounds),
      defaultWord: d.vec4u(stats.defaultWord, 0, 0, 0),
      ...scarLook,
    });
  };
  const groupOf = () =>
    root.createBindGroup(terrainLayout, {
      params,
      nodes: nodes.current!,
      plots: plots.current!,
      surfaces: surfaces.current!,
      surfaceIndex: surfaceIndex.current!,
      scarParams,
      scars: scars.texture.createView({ dimension: "2d-array" }),
      scarPages: scars.directory.createView(),
    });
  const scars = createScarTexture(registry);
  /** The surface's numbers as `set` last packed them, and the frame's view. */
  let look: Omit<d.Infer<typeof TerrainParams>, "view"> | null = null;
  let classView = false;
  const writeParams = () => {
    if (look) params.write({ ...look, view: d.vec4u(classView ? 1 : 0, 0, 0, 0) });
  };

  const source = {
    /** Draw the ground's classes in place of its lit colour, or not. */
    setClassView(on: boolean) {
      if (on === classView) return;
      classView = on;
      writeParams();
    },
    ready: () => groundFilterReady(root, registry, scarSampler),
    group: null as unknown as ReturnType<typeof groupOf>,
    /** Follow the side's learned ground (null: none); true when the scars
     *  changed, so what grows on them must regrow. */
    setGround(ground: GroundMarks | null): boolean {
      const changed = scars.setGround(ground);
      const key = ground ? `${ground.cols}/${ground.rows}/${ground.cellM}` : "";
      if (key !== regionGrid) {
        regionGrid = key;
        regions = [];
        if (ground) {
          const span = SCAR_REGION_CELLS * ground.cellM;
          for (let y = 0; y < ground.rows * ground.cellM; y += span)
            for (let x = 0; x < ground.cols * ground.cellM; x += span)
              regions.push([
                x,
                y,
                Math.min(x + span, ground.cols * ground.cellM),
                Math.min(y + span, ground.rows * ground.cellM),
              ]);
        }
      }
      return changed;
    },
    scarStats: () => scars.stats(),
    /** Exact filter stencil plus the bounded crater view-ray displacement. */
    prepareScars(region?: ScarRegion, clipped = false) {
      clip = clipped && region ? region : noClip;
      const cellM = scars.stats().cellM;
      const priorDirectory = scars.directory;
      scars.prepare(region, 2 + (2 * CRATER_DEPTH_CAP * scarLook.relief.x) / cellM);
      writeScarParams();
      if (scars.directory !== priorDirectory) source.group = groupOf();
    },
    scarRegions(): readonly ScarRegion[] {
      return scars.hasMarks() && !scars.globalEligible() ? regions : EMPTY_SCAR_REGIONS;
    },
    set(surface: TerrainSurface) {
      const { plots: tree, site, biome } = surface;
      nodes.set(nodeBuffer(tree.nodes.length / NODE_FLOATS)).write(packNodes(tree));
      plots.set(plotBuffer(tree.plots.length)).write(packPlots(surface));
      const field = buildSurfaceField(site, terrainReach(surface));
      surfaces
        .set(surfaceBuffer(field.records.length / SURFACE_FLOATS))
        .write(field.records.buffer as ArrayBuffer);
      surfaceIndex.set(indexBuffer(field.index.length)).write(field.index.buffer as ArrayBuffer);
      const rules = biome.field_rules;
      const one = (key: string) => linear(biome.palettes[key][0]);
      look = {
        region: d.vec4f(...tree.region),
        field: d.vec4f(field.origin[0], field.origin[1], 1 / field.cellM, field.footprintM),
        fieldGrid: d.vec4u(field.cols, field.rows, field.levels, 0),
        shape: d.vec4f(
          rules.edge_warp_m,
          1 / rules.edge_warp_scale_m,
          1 / rules.mottle_scale_m[0],
          1 / rules.mottle_scale_m[1],
        ),
        verge: d.vec4f(...one(biome.verge.palette), biome.verge.width_m / 2),
        feathers: d.vec4f(
          biome.verge.feather_m,
          0,
          rules.size_m[0] * PLOT_PIXELS_FADE[0],
          rules.size_m[0] * PLOT_PIXELS_FADE[1],
        ),
        roads: roadLooks(biome),
        ...forestParams(biome.forest_floor, biome.palettes[biome.forest_floor.palette]),
        waterBed: d.vec4f(...one("water_bed"), triangleReach(site)),
        water: d.vec4f(...one("water"), WATER_OPACITY),
        shore: d.vec4f(...linear(biome.palettes[biome.shore.palette][0]), biome.shore.width_m),
        distant: d.vec4f(...one("distant"), 0),
      };
      writeParams();
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

/** How far past a bank's top a ground triangle that straddles it carries its
 *  tilt: a triangle's diagonal. */
function triangleReach(site: TerrainSite): number {
  return site.gridM * Math.SQRT2;
}

/** How far each ground rule of `surface` is read at a pixel `footprint`
 *  metres wide: what its surface field is built with. The material and any
 *  check of its lookups both build from this. */
export function terrainReach({ site, biome }: TerrainSurface) {
  const bankM = longestBank(site.rivers) + triangleReach(site);
  return (footprint: number) => groundReach(biome, footprint, bankM);
}

/** How far from its edge each ground rule is read by a pixel `footprint`
 *  metres wide, in metres: the surface field lists what lies within it, and a
 *  distance past it only keeps its side. Each term is one reader:
 *
 *  - paved: the verge beside a road (`groundVerge`: its half width, then a
 *    feather a pixel wide at least); the road's own edge (`groundColour`: half
 *    a feather either side); the grass thinning past its road margin;
 *  - forest: the floor's ragged verge (`forestVergeInside` moves the edge out
 *    by up to the verge and its warp, `forestFloorWeight` feathers it by a
 *    pixel at least), which `groundDapple` and the grass read too, the grass
 *    thinning past its margin;
 *  - water: the wet bank (`groundShore`), the bed's and the water surface's
 *    pixel-wide edge, the surface's shore band (`waterSurface`), the grass's
 *    margin, and the bank's shading (`groundBank`) out to `bankM` from the
 *    water: the ground's longest bank and a triangle past its top. */
export function groundReach(biome: Biome, footprint: number, bankM = 0): SurfaceReach {
  const floor = biome.forest_floor;
  const grass = biome.grass.clear_m;
  return {
    paved: Math.max(
      biome.verge.width_m / 2 + Math.max(biome.verge.feather_m, footprint),
      Math.max(...Object.values(biome.roads).map((road) => road.feather_m), footprint) / 2,
      grass.road + GRASS_EDGE_M,
    ),
    forest:
      floor.verge_m +
      floor.verge_warp_m +
      Math.max(footprint, floor.verge_m * FOREST_FEATHER, grass.area + GRASS_EDGE_M),
    water: Math.max(
      biome.shore.width_m,
      footprint / 2,
      WATER_SHORE_IN_M,
      bankM + 2 * Math.max(footprint, BANK_EASE_M),
      grass.area + GRASS_EDGE_M,
    ),
  };
}

const LUMA = [0.2126, 0.7152, 0.0722] as const;
const luminance = (c: readonly number[]) => c[0] * LUMA[0] + c[1] * LUMA[1] + c[2] * LUMA[2];

/** One paved kind's row of the look table. The patches' colour is scaled to
 *  the surface's own luminance: a patch is a change of hue alone. */
function roadLook(road: Road, palettes: Biome["palettes"]) {
  const [core, worn] = palettes[road.palette].map(linear);
  const level = luminance(core) / luminance(worn);
  return {
    core: d.vec4f(...core, road.roughness),
    worn: d.vec4f(worn[0] * level, worn[1] * level, worn[2] * level, road.mottle),
    shape: d.vec4f(road.feather_m, 1 / road.patch_m, road.grain, 1 / road.grain_m),
    join: d.vec4f(road.join_m, 0, 0, 0),
  };
}

/** The look table: each paved kind's own row of `biome.roads`, or its
 *  default, in the order of the kinds' tags. */
export function roadLooks(biome: Biome) {
  return SURFACE_AREA_KINDS.map((kind) => roadLook(pick(biome.roads, kind), biome.palettes));
}

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
