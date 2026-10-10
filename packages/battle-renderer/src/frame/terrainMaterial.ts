// The biome's ground material on the GPU. `groundColour` paints a point in
// layers, each a term of its own below, the lowest first:
// - the plot under the point (walking the plot split), with its own texture
//   (rows, grain, wheelings: `fieldTexture`; dry patches: `mottle`), and the
//   verge along every plot edge;
// - the forest floor (`forestFloor`: leaf litter, moss, humus, roots) over
//   the simulation's forest shapes;
// - the shore's bands along the water (`groundShore`);
// - the roads (`groundRoads`), each kind its own surface (`biome.roads`);
// - the water bed.
// It returns linear albedo and roughness; lighting, shadow and FogTerm stay
// with the world pass, which also reads the terms that only shade: a bank's
// slope (`groundBank`), a road's ruts and curb (`groundRoadRelief`), the sun
// flecks under the crowns (`groundDapple`), the water's surface
// (`waterSurface`) and the side's learned scars (`groundScars`).
//
// Every edge is the simulation's own shape, and only its look is softened.
// A road's 50% blend is on the road rule's edge; the ground beside it is
// worn across a shoulder the grass thins over, and a town street has a paved
// walk there instead, a curb along its edge and painted lines. The forest
// floor meets the field across a ragged verge on each shape's edge. The plot
// edges wander (a small warp gives them a hand-cut line). Detail finer than
// a pixel fades to its mean, so the patchwork neither shimmers nor changes
// value with zoom.
//
// Every number here that sets the look and is not the biome's lives in
// `terrain/terrainTuning.ts`, spliced into the WGSL as a constant.
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
  type SurfaceField,
  type SurfaceReach,
} from "../terrain/surfaceField";
import { CUT_A, CUT_B } from "../terrain/strokes";
import { isRoad, SURFACE_AREA_KINDS, type SurfaceAreaKind } from "../terrain/surfaces";
import {
  CLASS_EDGE,
  CLASS_FOREST_SHIFT,
  CLASS_HASH_MASK,
  CLASS_KIND_MAX,
  CLASS_KIND_SHIFT,
  CLASS_STEPS_PER_M,
} from "../terrain/groundClasses";
import { GRASS_EDGE_M } from "../terrain/grassField";
import { terrainTuning as tuning } from "../terrain/terrainTuning";
import { longestBank } from "../terrain/rivers";
import {
  linearRgb,
  roadRow,
  SCAR_CHANNELS,
  type Biome,
  type ForestFloor,
  type Road,
  type ScarMark,
} from "../terrain/biome";
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
  /** How far this kind's road is carried onto a road drawn over it, metres;
   *  then unused; its walk's width in metres (0 for none) and the row that
   *  draws the walk. */
  join: d.vec4f,
  /** The worn ground beside it: linear rgb, and its width in metres. */
  shoulder: d.vec4f,
  /** The shoulder's outer edge: the share of its width it wanders in by,
   *  1 / the wander's size; then the share of the field's grass that still
   *  grows where the ground is most worn, and how far the ground goes to the
   *  shoulder's colour there. */
  edge: d.vec4f,
  /** Its ruts: two distances from a stroke's centreline (0 for none), a
   *  rut's width, the steepest slope of a rut's side (rise over run). */
  ruts: d.vec4f,
  /** How far a rut darkens the surface at its middle; a centre strip's half
   *  width, and the widest stroke (as a half width) that has one; then 1
   *  where the kind is a carriageway (paving is not). */
  track: d.vec4f,
  /** Its walk's slabs: 1 / a slab's length along the road (0 for none), how
   *  far a joint darkens the surface; then its curb's face: its width, and
   *  its slope (rise over run). */
  slabs: d.vec4f,
  /** Its curb's stones: linear rgb, and their width in metres (0 for none). */
  curb: d.vec4f,
  /** Its painted lines: the paint (linear rgb) and how much of the surface
   *  it hides where it is whole. */
  paint: d.vec4f,
  /** The centre line's half width in metres (0 for no markings), a dash's
   *  length, a dash and its gap together, and the share of its cover the
   *  paint loses where it is worn. */
  marks: d.vec4f,
  /** A crossing's bars: a bar's width, its length along the road, how far
   *  from the crossing road's edge it starts and how far it keeps from its
   *  own road's edge. */
  crossing: d.vec4f,
  /** Its kerbstones: 1 / a stone's length along the road, and how far the
   *  joint between two darkens them; then its slabs where it is laid as an
   *  area: 1 / a slab's side (0 for none), how far a joint darkens it. */
  stones: d.vec4f,
});

export const TerrainParams = d.struct({
  /** The plot region: minX, minY, maxX, maxY. */
  region: d.vec4f,
  /** The surface field's grid (`terrain/surfaceField.ts`): its low corner,
   *  1 / the finest cell's side, the widest pixel the finest level serves. */
  field: d.vec4f,
  /** The finest level's cells across and up, and the levels. */
  fieldGrid: d.vec4u,
  /** Edge warp metres, 1 / warp scale, 1 / the mottle's scale, then unused. */
  shape: d.vec4f,
  /** Linear rgb, verge half width. */
  verge: d.vec4f,
  /** Verge feather, the pixel footprints (metres) over which plots give way
   *  to the distant colour, then unused. */
  feathers: d.vec4f,
  /** Each paved kind's look, by its tag (`SURFACE_AREA_KINDS`). */
  roads: d.arrayOf(RoadLook, ROAD_KINDS),
  /** The rows' tags in the order they are painted, the lowest layer first. */
  roadOrder: d.vec4u,
  /** The forest floor's leaf litter, moss and humus (linear rgb); with the
   *  moss and the humus, how far each takes the litter over. */
  forestLitter: d.vec4f,
  forestMoss: d.vec4f,
  forestHumus: d.vec4f,
  /** 1 / patch scale, mottle, roughness, root contrast. */
  forestDetail: d.vec4f,
  /** Verge width, verge warp, 1 / verge warp scale, 1 / root spacing. */
  forestVerge: d.vec4f,
  /** 1 / fleck size, fleck share, the sun a fleck lets through, unused. */
  forestDapple: d.vec4f,
  /** A tree line's band: the length it narrows to a point over before each
   *  end of its strip, how far its edge wanders, then unused. */
  forestLine: d.vec4f,
  /** The water bed's colour (linear rgb), and how far past a bank's top a
   *  ground triangle can carry its tilt, in metres: a triangle's diagonal. */
  waterBed: d.vec4f,
  /** The water surface's colour (linear rgb) out in the channel, and its opacity there. */
  water: d.vec4f,
  /** The surface's colour (linear rgb) at its edge, and its opacity there. */
  waterEdge: d.vec4f,
  /** 1 / the shallows' width in metres, the streaks' light, then unused. */
  waterLook: d.vec4f,
  /** The wet bank's silt (linear rgb) and its width in metres. */
  shore: d.vec4f,
  /** The bare earth behind it (linear rgb) and how far from the water it ends. */
  shoreEarth: d.vec4f,
  /** How far in the earth's line wanders (a share of its reach), 1 / the
   *  wander's scale, the bands' lift over the ground's luminance, and the
   *  share of a bank's slope its shading shows. */
  shoreEdge: d.vec4f,
  distant: d.vec4f,
  /** 1 draws the ground's classes (`groundClasses`) in place of its lit
   *  colour: the "ground-classes" frame view. Then 1 draws the roads plain
   *  (`setRoadWear`). Then unused. */
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
export const PlotNode = d.struct({ line: d.vec4f, children: d.vec4i });
export const PlotRecord = d.struct({
  /** Linear rgb, roughness. */
  colour: d.vec4f,
  /** Across the rows (unit), row period metres, row contrast. */
  rows: d.vec4f,
  /** Dry patches' strength, the plot's kind (an index into the biome's
   *  plots), how far the rows break along their length. */
  detail: d.vec4f,
  /** 1 / the grain's size, its contrast, 1 / its stretch along the rows. */
  grain: d.vec4f,
  /** 1 / a dry patch's length along the rows and 1 / its width across them;
   *  then unused. */
  dry: d.vec4f,
  /** Wheelings: one pair every this many rows, half a wheeling's width in
   *  metres, how far it darkens. */
  tram: d.vec4f,
});
export const SurfaceRecord = d.struct({ ends: d.vec4f, detail: d.vec4f });

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

/** Rec. 709's weights: a linear colour's luminance is its dot with them. */
const LUMA_WEIGHTS = [0.2126, 0.7152, 0.0722] as const;
const LUMA = d.vec3f(...LUMA_WEIGHTS);
const MOTTLE_DRY = d.vec3f(...tuning.mottle.dry);
/** A number as WGSL float text: a whole number keeps its `.0`. */
const wgslFloat = (n: number) => (Number.isInteger(n) ? n.toFixed(1) : `${n}`);
const NODE_BYTES = 32;
const PLOT_BYTES = 96;

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

/** The painterly variation of the ground, `(value, dry)`. `dry` (0 or more)
 *  is the hue: patches drawn afresh in every plot (`plot` its index), `reach`
 *  their inverse length along the plot's rows and width across them (`across`
 *  is the unit vector across the rows), with firm edges, where the crop is
 *  drier; `groundTint` turns it into ochre at unchanged luminance. `value`
 *  (around 0) is the brightness: fine value noise only, fading as it drops
 *  below a pixel (`footprint` is metres per pixel). Nothing broad is darker
 *  or cooler than its plot: a large soft darker patch reads as a cloud's
 *  shadow with nothing to cast it. */
const mottle = tgpu.fn(
  [d.vec2f, d.f32, d.vec2f, d.f32, d.vec2f],
  d.vec2f,
)((xy, footprint, across, plot, reach) => {
  "use gpu";
  const shape = terrainLayout.$.params.shape;
  const along = d.vec2f(-across.y, across.x);
  const lane = d.vec2f(std.dot(xy, along) * reach.x, std.dot(xy, across) * reach.y);
  const seed = d.vec2f(std.fract(plot * 0.6180339) * 997, std.fract(plot * 0.7548776) * 991);
  const at = std.add(lane, seed);
  const streak = valueNoise(at);
  // The patch's edge is where the noise crosses the cut. Its distance in
  // metres (the noise over its slope, by finite differences) gives an edge
  // a pixel wide however slowly the noise crosses, so no patch fades softly
  // out as a shadow's penumbra does.
  const slope = d.vec2f(
    ((valueNoise(std.add(at, d.vec2f(tuning.mottle.slopeStep, 0))) - streak) /
      tuning.mottle.slopeStep) *
      reach.x,
    ((valueNoise(std.add(at, d.vec2f(0, tuning.mottle.slopeStep))) - streak) /
      tuning.mottle.slopeStep) *
      reach.y,
  );
  const inside = (streak - tuning.mottle.patchCut) / std.max(std.length(slope), 1e-4);
  const edge = std.max(footprint, tuning.mottle.patchEdgeM) * 0.5;
  const parched = std.smoothstep(-edge, edge, inside);
  const fine = valueNoise(std.add(std.mul(xy, shape.z), d.vec2f(37.1, 11.3))) - 0.5;
  const fineShown =
    1 - std.smoothstep(tuning.mottle.fineFade[0], tuning.mottle.fineFade[1], footprint * shape.z);
  return d.vec2f(
    fine * tuning.mottle.fineValue * fineShown,
    parched * tuning.mottle.patchHue + std.max(fine, 0) * tuning.mottle.fineHue * fineShown,
  );
});

/** How much of a pixel `footprint` metres wide at `xy` a wheeling of plot
 *  `plot` covers: the tracks a tractor leaves through a drilled crop, two
 *  furrows bare in every `tram_rows`. The grass leaves them bare too. */
export const groundWheeling = tgpu
  .fn(
    [d.vec2f, d.f32, d.i32],
    d.f32,
  )(/* wgsl */ `(xy: vec2f, footprint: f32, plot: i32) -> f32 {
  let rows = terrainLayout.$.plots[plot].rows;
  let tram = terrainLayout.$.plots[plot].tram;
  if (tram.x <= 0.0 || rows.z <= 0.0) { return 0.0; }
  let phase = dot(xy, rows.xy) / rows.z;
  let furrow = round(phase);
  let nth = furrow - tram.x * floor(furrow / tram.x);
  if (nth != 0.0 && nth != ${wgslFloat(tuning.wheeling.gaugeRows)}) { return 0.0; }
  let off = abs(phase - furrow) * rows.z;
  let pixel = max(footprint, ${tuning.wheeling.leastPixelM});
  return clamp((tram.y - off) / pixel + 0.5, 0.0, 1.0) * min(1.0, 2.0 * tram.y / pixel);
}`)
  .$uses({ terrainLayout });

/** What a plot's own ground does to its colour at `xy`, as a factor on its
 *  albedo: its rows (dark furrows between beds, broken along each row's
 *  length), its wheelings, and its grain (clods, tussocks, stubble: four
 *  octaves of noise in the plot's own frame, stretched along the rows).
 *
 *  Rows and grain leave the plot's mean where its palette put it: the rows
 *  darken by half their contrast on average, as they always did, and the
 *  grain is as much lighter as darker. Every term is finer than a few
 *  metres and fades to its mean as it drops below a pixel (`footprint` is
 *  metres per pixel), so a field neither shimmers nor changes value with
 *  zoom, and nothing broad is darker than its plot. */
const fieldTexture = tgpu
  .fn(
    [d.vec2f, d.f32, d.i32],
    d.f32,
  )(/* wgsl */ `(xy: vec2f, footprint: f32, plot: i32) -> f32 {
  let rows = terrainLayout.$.plots[plot].rows;
  let grain = terrainLayout.$.plots[plot].grain;
  let breaks = terrainLayout.$.plots[plot].detail.z;
  let u = dot(xy, vec2f(-rows.y, rows.x));
  let v = dot(xy, rows.xy);
  var value = 1.0;
  if (rows.z > 0.0) {
    let phase = v / rows.z;
    let shown = 1.0 - smoothstep(${tuning.rows.fade[0]}, ${tuning.rows.fade[1]}, footprint / rows.z);
    let stripe = 0.5 + 0.5 * cos(phase * 6.2831853);
    var furrow = stripe * stripe * stripe;
    let along = grain.x * grain.z / ${wgslFloat(tuning.rows.breakGrains)};
    let seen = shown * (1.0 - smoothstep(${tuning.grain.fade[0]}, ${tuning.grain.fade[1]}, footprint * along));
    if (breaks > 0.0 && seen > 0.0) {
      // Each furrow has its own noise along its length; two meet on the bed
      // between them, where neither darkens anything, so nothing steps.
      let gap = valueNoise(vec2f(u * along, floor(phase + 0.5) * 7.31 + 0.5));
      furrow *= 1.0 + breaks * (gap * 2.0 - 1.0) * seen;
    }
    let bare = terrainLayout.$.plots[plot].tram.z * groundWheeling(xy, footprint, plot);
    value = (1.0 - rows.w * (0.5 + (furrow - ${tuning.rows.mean}) * shown)) * (1.0 - bare);
  }
  if (grain.y > 0.0) {
    var size = grain.x / ${tuning.grain.octave * tuning.grain.octave};
    var sum = 0.0;
    var push = vec2f(0.0);
    var weights = array<f32, 4>(${tuning.grain.weights.join(", ")});
    var turns = array<vec2f, 4>(${tuning.grain.turns.map((t) => `vec2f(${Math.cos(t)}, ${Math.sin(t)})`).join(", ")});
    for (var i = 0u; i < 4u; i++) {
      let shown = 1.0 - smoothstep(${tuning.grain.fade[0]}, ${tuning.grain.fade[1]}, footprint * size);
      if (shown > 0.0) {
        let turn = turns[i];
        let p = vec2f(u * grain.z, v) * size;
        let seed = vec2f(f32(i) * 17.31 + 3.7, f32(plot % 64) * 1.37);
        let n = valueNoise(vec2f(dot(p, turn), dot(p, vec2f(-turn.y, turn.x))) + seed + push);
        sum += weights[i] * shown * clamp((n - 0.5) * ${wgslFloat(2 * tuning.grain.crisp)}, -1.0, 1.0);
        push = vec2f(cos(n * 6.2831853), sin(n * 6.2831853)) * ${tuning.grain.warp};
      }
      size *= ${tuning.grain.octave};
    }
    value *= 1.0 + grain.y * sum;
  }
  return value;
}`)
  .$uses({ terrainLayout, valueNoise, groundWheeling });

/** `albedo` shifted toward ochre by `dry` (0 or more) at its own Rec. 709
 *  luminance. */
export const groundTint = tgpu.fn(
  [d.vec3f, d.f32],
  d.vec3f,
)((albedo, dry) => {
  "use gpu";
  const tinted = std.mul(albedo, std.add(d.vec3f(1), std.mul(MOTTLE_DRY, dry)));
  return std.mul(tinted, std.dot(albedo, LUMA) / std.max(std.dot(tinted, LUMA), 1e-5));
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
 * the street test map's road edges by an ULP. */
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
 *  without a ruled edge. The native shape stays the rule; this is only its
 *  look, and the grass stops at whichever edge lies farther out. */
export const forestVergeInside = tgpu.fn(
  [d.vec2f, d.f32],
  d.f32,
)((xy, forest) => {
  "use gpu";
  const verge = terrainLayout.$.params.forestVerge;
  const wander = (valueNoise(std.add(std.mul(xy, verge.z), d.vec2f(71.3, 23.9))) - 0.5) * 2;
  const patches =
    valueNoise(std.add(std.mul(xy, tuning.forestVerge.patchScale), d.vec2f(5.1, 91.7))) - 0.5;
  return forest + verge.x * tuning.forestVerge.inset + wander * verge.y + patches * verge.x;
});

/** The forest floor's weight at `xy`: 1 inside its drawn edge, 0 outside,
 *  feathered over a pixel at least. */
const forestFloorWeight = tgpu.fn(
  [d.vec2f, d.f32, d.f32],
  d.f32,
)((xy, forest, footprint) => {
  "use gpu";
  const feather = std.max(
    footprint,
    terrainLayout.$.params.forestVerge.x * tuning.forestVerge.feather,
  );
  return std.smoothstep(-feather, feather, forestVergeInside(xy, forest));
});

/** The tree line (a strip of forest) at `xy`, in metres: `x`, how far inside
 *  the deepest strip of `cell` the point lies, by `groundForest`'s own
 *  arithmetic; `y`, how far inside the band drawn under it, which is the
 *  strip itself but for the last `tree_line.taper_m` before an end it is cut
 *  square at, where its half width falls evenly to nothing. Both far outside
 *  where the cell lists no strip. */
const groundTreeLine = tgpu
  .fn(
    [d.vec2f, d.vec4u],
    d.vec2f,
  )(/* wgsl */ `(xy:vec2f,cell:vec4u)->vec2f {
 var inside=-1e9;var band=-1e9;
 let taper=terrainLayout.$.params.forestLine.x;
 for(var i=cell.y;i<cell.z;i++){
  let entry=terrainLayout.$.surfaceIndex[i];
  if((entry>>${SURFACE_KIND_SHIFT}u)!=${SURFACE_STROKE}u){continue;}
  let record=terrainLayout.$.surfaces[entry&${SURFACE_RECORD_MASK}u];
  let here=strokeCutInside(xy,record.ends,record.detail,record.detail.x-polygonEdgeDistance(xy,record.ends.xy,record.ends.zw));
  inside=max(inside,here);
  let cuts=u32(record.detail.z);
  var narrow=here;
  if(cuts!=0u){
   let a=record.ends.xy;let ab=record.ends.zw-a;let len=length(ab);let rel=xy-a;
   let along=dot(rel,ab)/len;
   let end=min(select(1e9,along,(cuts&${CUT_A}u)!=0u),select(1e9,len-along,(cuts&${CUT_B}u)!=0u));
   narrow=min(here,record.detail.x*min(1.0,end/taper)-abs(rel.x*ab.y-rel.y*ab.x)/len);
  }
  band=max(band,narrow);
 }
 return vec2f(inside,band);
}`)
  .$uses({ terrainLayout, polygonEdgeDistance, strokeCutInside });

/** Whether the forest at `xy` is a tree line, and its band: `x` is 1 where
 *  the deepest forest shape there (`forest` metres inside it) is a strip;
 *  `y` how far inside the band's drawn edge the point then lies, in metres,
 *  the edge wandering by `tree_line.warp_m`. A strip has no floor of its own:
 *  its band is the plots' verge, under the verge's grass, so the ground and
 *  the grass both read this. `cell` is the point's `groundCell`. */
export const groundLineBand = tgpu.fn(
  [d.vec2f, d.vec4u, d.f32],
  d.vec2f,
)((xy, cell, forest) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const line = groundTreeLine(xy, cell);
  if (line.x < forest) {
    return d.vec2f(0, -1e9);
  }
  const at = std.add(std.mul(xy, params.forestVerge.z), d.vec2f(71.3, 23.9));
  return d.vec2f(1, line.y + (valueNoise(at) - 0.5) * 2 * params.forestLine.y);
});

/** What a forest draws on the ground at `xy`, `forest` metres inside the
 *  deepest shape: `x`, the weight of a wood's floor; `y`, of a tree line's
 *  band. One of them is 0. */
const groundFloor = tgpu.fn(
  [d.vec2f, d.f32, d.f32],
  d.vec2f,
)((xy, footprint, forest) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const wood = forestFloorWeight(xy, forest, footprint);
  // Only near a forest can the shape be a strip.
  if (wood > 0 || forest > -params.forestVerge.x - params.forestVerge.y) {
    const band = groundLineBand(xy, groundCell(xy, footprint), forest);
    if (band.x > 0.5) {
      const feather = std.max(footprint, params.forestVerge.x * tuning.forestVerge.feather);
      return d.vec2f(0, std.smoothstep(-feather, feather, band.y));
    }
  }
  return d.vec2f(wood, 0);
});

/** The forest floor's linear albedo at `xy`: leaf litter drifting into moss
 *  and darker humus, crossed by roots, under the ground's own mottle `noise`.
 *  Each drift is two octaves of noise on lattices turned against each other
 *  and against the map's axes, eased over a wide band: one lattice cut at a
 *  threshold drew square patches in rows, which read as noise through the
 *  crowns. */
const forestFloor = tgpu.fn(
  [d.vec2f, d.f32, d.f32],
  d.vec3f,
)((xy, footprint, noise) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const detail = params.forestDetail;
  const at = std.mul(xy, detail.x);
  const turned = d.vec2f(at.x * 0.8 - at.y * 0.6, at.x * 0.6 + at.y * 0.8);
  const across = d.vec2f(at.x * 0.28 + at.y * 0.96, at.y * 0.28 - at.x * 0.96);
  const moss = std.smoothstep(
    tuning.forestFloor.moss.cut[0],
    tuning.forestFloor.moss.cut[1],
    valueNoise(std.add(turned, d.vec2f(13.3, 7.7))) * tuning.forestFloor.moss.weights[0] +
      valueNoise(std.add(std.mul(across, tuning.forestFloor.moss.fineScale), d.vec2f(4.1, 9.2))) *
        tuning.forestFloor.moss.weights[1],
  );
  const humus = std.smoothstep(
    tuning.forestFloor.humus.cut[0],
    tuning.forestFloor.humus.cut[1],
    valueNoise(std.add(std.mul(across, tuning.forestFloor.humus.scales[0]), d.vec2f(2.9, 41.1))) *
      tuning.forestFloor.humus.weights[0] +
      valueNoise(std.add(std.mul(turned, tuning.forestFloor.humus.scales[1]), d.vec2f(17.3, 5.9))) *
        tuning.forestFloor.humus.weights[1],
  );
  let floor = std.mix(params.forestLitter.xyz, params.forestMoss.xyz, moss * params.forestMoss.w);
  floor = std.mix(floor, params.forestHumus.xyz, humus * params.forestHumus.w);
  // Roots: thin dark lines where a noise field crosses its middle, broken
  // into short runs by a second field, fading to their mean below a few
  // pixels a line.
  const rootsAt = std.mul(xy, params.forestVerge.w);
  const ridge = std.abs(valueNoise(std.add(rootsAt, d.vec2f(29.1, 3.3))) - 0.5);
  const runs = std.smoothstep(
    tuning.forestFloor.roots.runs[0],
    tuning.forestFloor.roots.runs[1],
    valueNoise(std.add(std.mul(rootsAt, tuning.forestFloor.roots.runScale), d.vec2f(8.3, 17.9))),
  );
  const shown =
    1 -
    std.smoothstep(
      tuning.forestFloor.roots.fade[0],
      tuning.forestFloor.roots.fade[1],
      footprint * params.forestVerge.w,
    );
  const line =
    (1 -
      std.smoothstep(tuning.forestFloor.roots.line[0], tuning.forestFloor.roots.line[1], ridge)) *
    runs;
  const root = line * shown + tuning.forestFloor.roots.mean * (1 - shown);
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
  const n =
    valueNoise(at) * tuning.dapple.weights[0] +
    valueNoise(std.add(std.mul(at, tuning.dapple.octave), d.vec2f(3.7, 8.1))) *
      tuning.dapple.weights[1];
  const cut = 1 - dapple.y;
  const fleck = std.smoothstep(cut - tuning.dapple.edge, cut + tuning.dapple.edge, n);
  const shown =
    1 - std.smoothstep(tuning.dapple.fade[0], tuning.dapple.fade[1], footprint * dapple.x);
  const flecks = std.mix(dapple.y, fleck, shown);
  return flecks * dapple.z * forestFloorWeight(xy, forest, footprint);
});

/** Where a point sits in the paving (`groundPaved`). */
export const GroundPaved = d
  .struct({
    /** How far inside each row's paving it lies, by the tag of the kind
     *  whose row draws it: negative outside, far outside a row the cell
     *  lists nothing of. A stroke or an area is drawn by its own kind's row,
     *  and a walk (a look, not the simulation's paving) by its road's
     *  `walk.kind` row. */
    drawn: d.vec4f,
    /** The stroke it lies deepest in, as a lane to drive along: the unit
     *  vector away from the stroke's centreline, the distance from it, and
     *  the stroke's half width. */
    lane: d.vec4f,
    /** That stroke's unit direction there, how far along the stroke the
     *  point is (metres from its first point), and the stroke's kind (its
     *  tag; -1 where the cell lists no stroke). */
    run: d.vec4f,
    /** How far inside the simulation's paving it lies, of any kind: the
     *  rule, which a walk is no part of. */
    rule: d.f32,
    /** The cell's paved list (where it starts and ends in `surfaceIndex`),
     *  for what reads the strokes round the point again (`groundMarks`). */
    list: d.vec2u,
    /** The slab bearing of the area the point stands in
     *  (`SURFACE_AREA_BEARING`); negative outside every area. */
    bearing: d.f32,
  })
  .$name("GroundPaved");

/** Where `xy` sits in the paving of `cell`: how far inside each row's (a
 *  stroke counts for its own kind's), and the stroke it lies deepest in. The
 *  polygons are one union: membership comes from native triangles and only
 *  the exposed union boundary contributes feathering, so their distance goes
 *  to one row, that of the kind the point stands on (where kinds overlap,
 *  the earlier), or outside them all the nearest edge's. A stroke whose kind
 *  has a walk is drawn that much wider by the walk's row. A stretch's
 *  record holds how far along its stroke it starts (`SURFACE_STROKE_ALONG`,
 *  its `detail.w`). Stroke math retains its original order. */
export const groundPaved = tgpu
  .fn(
    [d.vec2f, d.vec4u],
    GroundPaved,
  )(/* wgsl */ `(xy:vec2f,cell:vec4u)->GroundPaved {
 var paved=vec4f(-1e9);var member=${ROAD_KINDS}u;var nearest=1e9;var edge=0u;
 var lane=vec4f(0.0);var run=vec4f(0.0,0.0,0.0,-1.0);var deepest=-1e9;var bearing=-1.0;
 for(var i=cell.x;i<cell.y;i++){
  let entry=terrainLayout.$.surfaceIndex[i];
  let seg=terrainLayout.$.surfaces[entry&${SURFACE_RECORD_MASK}u];
  let kind=entry>>${SURFACE_KIND_SHIFT}u;
  if(kind==${SURFACE_STROKE}u){
   let a=seg.ends.xy;let ab=seg.ends.zw-a;
   let t=clamp(dot(xy-a,ab)/max(dot(ab,ab),1e-6),0.0,1.0);
   let away=xy-(a+ab*t);let off=length(away);let own=u32(seg.detail.y);
   let inside=strokeCutInside(xy,seg.ends,seg.detail,seg.detail.x-off);
   paved[own]=max(paved[own],inside);
   let walk=terrainLayout.$.params.roads[own].join.zw;
   if(walk.x>0.0){paved[u32(walk.y)]=max(paved[u32(walk.y)],inside+walk.x);}
   if(inside>deepest){
    deepest=inside;lane=vec4f(away/max(off,1e-5),off,seg.detail.x);
    let len=max(length(ab),1e-6);run=vec4f(ab/len,seg.detail.w+t*len,f32(own));
   }
  }
  else if(kind==${SURFACE_TRIANGLE}u){
   if(polygonTriangleInside(xy,seg.ends.xy,seg.ends.zw,seg.detail.xy)&&u32(seg.detail.z)<=member){
    member=u32(seg.detail.z);bearing=seg.detail.w;
   }
  }
  else{
   let off=polygonEdgeDistance(xy,seg.ends.xy,seg.ends.zw);
   if(off<nearest){nearest=off;edge=u32(seg.detail.x);}
  }
 }
 var rule=deepest;
 if(member<${ROAD_KINDS}u){
  paved[member]=max(paved[member],nearest);rule=max(rule,nearest);
 }
 else if(nearest<1e9){
  paved[edge]=max(paved[edge],-nearest);rule=max(rule,-nearest);
 }
 return GroundPaved(paved,lane,run,rule,cell.xy,bearing);
}`)
  .$uses({
    terrainLayout,
    polygonEdgeDistance,
    polygonTriangleInside,
    strokeCutInside,
    GroundPaved,
  });

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
  [d.vec2f, d.vec4u, GroundPaved],
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
  return d.vec4f(d.f32(leaf), edge, paved.rule, groundForest(xy, cell));
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

/** How the ground at `xy` was cut for a river, as shading reads it: the
 *  bank's slope there (`xy`: rise per metre east and north), and how much the
 *  cut decides the shading normal (`z`: 1 on the bed, on the bank and as far
 *  past its top as a triangle that straddles the top reaches; 0 beyond).
 *  Only the bank slopes: the bed is lit flat, since through the water a bed
 *  lit as the V it is reads as one bright half and one dark, and past the
 *  bank's top the land is lit flat too.
 *  The grid's triangles cannot draw a bank a few metres wide, and lit by
 *  their own normals they read as steps along the river; lit by this, the
 *  bank is the round band the distance says. It shows the biome's share of
 *  the bank's true slope (`shore.relief`), never past `shading.maxSlope`. Shading
 *  only: the ground's height stays the simulation's, and the water's edge
 *  stays the nearest stretch's. `cell` is the point's `groundCell`,
 *  `footprint` the metres a pixel spans. */
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
  let ease=max(footprint,${tuning.bank.easeM});
  let sloped=(1.0-smoothstep(run-ease,run+ease,-inside))*(1.0-smoothstep(0.0,ease,inside));
  let top=run+terrainLayout.$.params.waterBed.w;
  let weight=exp(clamp(inside,-${tuning.bank.blendReachM},${tuning.bank.blendReachM})/${tuning.bank.blendM});
  let shown=min(grade*terrainLayout.$.params.shoreEdge.w,${tuning.shading.maxSlope});
  sum+=vec4f(away/max(off,1e-4)*shown*sloped,1.0-smoothstep(top,top+2.0*ease,-inside),1.0)*weight;
 }
 if(sum.w<=0.0){return vec3f(0.0);}
 return sum.xyz/sum.w;
}`)
  .$uses({ terrainLayout });

/** `xy` on a lattice turned `turn` radians from the map's axes. */
const turned = tgpu.fn(
  [d.vec2f, d.f32],
  d.vec2f,
)((xy, turn) => {
  "use gpu";
  const c = std.cos(turn);
  const s = std.sin(turn);
  return d.vec2f(xy.x * c + xy.y * s, xy.y * c - xy.x * s);
});

/** The shore's bands `water` metres inside the water's edge (`groundWater`:
 *  negative on the bank), as `(wet, earth, bare)`: how much the wet bank's
 *  silt and the bare earth's colour cover the ground, and how bare of grass
 *  it is. All are 1 at the water. The wet bank follows the water's edge and
 *  nothing else, so it is as round as the river; the earth's outer line
 *  wanders in toward the water, never out past the biome's reach, so the
 *  bank meets the field raggedly and the distance is read no farther. Grass
 *  thickens all the way across the earth, so tufts stand on it short of its
 *  line. `footprint` is the metres a pixel spans: a wander finer than it
 *  fades to its mean. */
export const groundShore = tgpu.fn(
  [d.vec2f, d.f32, d.f32],
  d.vec3f,
)((xy, footprint, water) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const out = -water;
  const wet = params.shore.w;
  const reach = params.shoreEarth.w;
  if (out >= reach) {
    return d.vec3f(0);
  }
  const broad = valueNoise(std.mul(turned(xy, tuning.shore.turns[0]), params.shoreEdge.y));
  const scale = params.shoreEdge.y * tuning.shore.fine;
  const fine = std.mix(
    valueNoise(std.add(std.mul(turned(xy, tuning.shore.turns[1]), scale), d.vec2f(41.7, 13.1))),
    0.5,
    std.smoothstep(tuning.shore.fineFade[0], tuning.shore.fineFade[1], footprint * scale),
  );
  const line = reach * (1 - params.shoreEdge.x * std.mix(broad, fine, tuning.shore.fineShare));
  return d.vec3f(
    1 - std.smoothstep(wet * tuning.shore.wetWhole, wet, out),
    1 - std.smoothstep(std.mix(wet, line, tuning.shore.earthWhole), line, out),
    1 - std.smoothstep(wet, line, out),
  );
});

/** `colour`, scaled up to luminance `least` where it is darker. */
const atLeast = tgpu.fn(
  [d.vec3f, d.f32],
  d.vec3f,
)((colour, least) => {
  "use gpu";
  const luminance = std.dot(colour, LUMA);
  return std.mul(colour, std.max(1, least / std.max(luminance, 1e-5)));
});

/** The verge's weight at a site, 1 on it: along every plot edge. It holds
 *  its full colour right up to the edge, so neighbouring plots meet in one
 *  colour and a plot boundary never steps from pixel to pixel; it fades out
 *  over a pixel at least. Beside a road the ground is its shoulder's
 *  (`groundShoulder`), not this. */
export const groundVerge = tgpu.fn(
  [d.vec4f, d.f32],
  d.f32,
)((site, footprint) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const vergeHalf = params.verge.w;
  return 1 - std.smoothstep(vergeHalf, vergeHalf + std.max(params.feathers.x, footprint), site.y);
});

/** Noise in [0, 1] with no lattice to see: three octaves of value noise, each
 *  turned off the grid, read at a point a broader octave has pushed about.
 *  Cut at a level, a plain octave shows its cells as straight-edged blotches. */
const wanderNoise = tgpu
  .fn(
    [d.vec2f],
    d.f32,
  )(/* wgsl */ `(at:vec2f)->f32 {
 let push=vec2f(valueNoise(at*${tuning.wander.pushScale}+vec2f(11.3,47.1)),valueNoise(at*${tuning.wander.pushScale}+vec2f(83.9,5.7)))-0.5;
 let p=at+push*${tuning.wander.push};
 let a=valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y));
 let b=valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*${tuning.wander.octaves[0]}+vec2f(19.1,7.7));
 let c=valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*${tuning.wander.octaves[1]}+vec2f(3.3,41.9));
 return a*${tuning.wander.weights[0]}+b*${tuning.wander.weights[1]}+c*${tuning.wander.weights[2]};
}`)
  .$uses({ valueNoise });

/** A road surface's grain around 0, at `p` in grain sizes with a pixel
 *  `pixel` of them wide: lumps at three scales and, finest, stones as crisp
 *  specks. Each scale fades out on its own as it nears a pixel, so the
 *  surface coarsens with distance and never crawls or steps. */
const roadGrain = tgpu
  .fn(
    [d.vec2f, d.f32],
    d.f32,
  )(/* wgsl */ `(p:vec2f,pixel:f32)->f32 {
 let shown=vec4f(1.0)-smoothstep(vec4f(${tuning.roadGrain.fade[0]}),vec4f(${tuning.roadGrain.fade[1]}),pixel*vec4f(1.0,${tuning.roadGrain.octaves.join(",")}));
 var grain=0.0;
 if(shown.x>0.0){grain+=(valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y))-0.5)*${tuning.roadGrain.weights[0]}*shown.x;}
 if(shown.y>0.0){grain+=(valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*${tuning.roadGrain.octaves[0]}+vec2f(19.1,7.7))-0.5)*${tuning.roadGrain.weights[1]}*shown.y;}
 if(shown.z>0.0){grain+=(valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*${tuning.roadGrain.octaves[1]}+vec2f(3.3,41.9))-0.5)*${tuning.roadGrain.weights[2]}*shown.z;}
 if(shown.w>0.0){
  let stone=valueNoise(p*${tuning.roadGrain.octaves[2]}+vec2f(71.9,13.7));
  grain+=(smoothstep(${tuning.roadGrain.stones[0]},${tuning.roadGrain.stones[1]},stone)-smoothstep(${tuning.roadGrain.stones[0]},${tuning.roadGrain.stones[1]},1.0-stone))*${tuning.roadGrain.weights[3]}*shown.w;
 }
 return grain;
}`)
  .$uses({ valueNoise });

/** The kind (its tag) of the stroke whose lanes run under a point: the
 *  stroke it lies deepest in, where that stroke's own surface is the one on
 *  top. -1 off the strokes, and where a higher layer lies over this one. */
const roadLane = tgpu
  .fn(
    [GroundPaved],
    d.i32,
  )(/* wgsl */ `(paved:GroundPaved)->i32 {
 if(paved.run.w<0.0){return -1;}
 let own=u32(paved.run.w);
 if(paved.drawn[own]<=0.0){return -1;}
 for(var n=${ROAD_KINDS}u;n>0u;n--){
  let k=terrainLayout.$.params.roadOrder[n-1u];
  if(k==own){break;}
  if(paved.drawn[k]>0.0){return -1;}
 }
 return i32(own);
}`)
  .$uses({ terrainLayout, GroundPaved });

/** How much of a grass centre strip lies at `xy`, 0 to 1: along the middle
 *  of a stroke whose kind has one and which is narrow enough for it, its
 *  edge wandering, fading out below a pixel or two. */
const groundStrip = tgpu
  .fn(
    [d.vec2f, d.f32, GroundPaved],
    d.f32,
  )(/* wgsl */ `(xy:vec2f,footprint:f32,paved:GroundPaved)->f32 {
 let kind=roadLane(paved);
 if(kind<0||terrainLayout.$.params.view.y==1u){return 0.0;}
 let look=terrainLayout.$.params.roads[kind];
 let half=look.track.y;
 if(half<=0.0||paved.lane.w>look.track.z){return 0.0;}
 let reach=half*(1.0+(wanderNoise(xy*${1 / tuning.strip.wanderM}+vec2f(41.3,9.7))-0.5)*${tuning.strip.wander});
 let soft=max(footprint,${tuning.strip.edgeM});
 let shown=1.0-smoothstep(${tuning.strip.pixels[0]},${tuning.strip.pixels[1]},footprint/(2.0*half));
 return (1.0-smoothstep(reach-soft,reach+soft,paved.lane.z))*shown;
}`)
  .$uses({ terrainLayout, wanderNoise, roadLane, GroundPaved });

/** A rut's cross-section is `(1 - u * u)^2` over its width (u from -1 to 1):
 *  this is its mean, and the steepest its side gets per unit of u. */
const RUT_MEAN = 8 / 15;
const RUT_STEEPEST = 1.5396;

/** The ruts at `xy`: `(slope east, slope north, shade)`. The slope is the
 *  rut's side as shading reads it (rise per metre; the ground itself is
 *  never moved); `shade` scales the surface's albedo by `1 + shade`, darker
 *  in a rut and lighter between by the ruts' share of the road, so the
 *  road's mean never changes. Ruts run at their kind's distances either side
 *  of a stroke's centreline, where the stroke is wide enough to hold them. */
const groundRuts = tgpu
  .fn(
    [d.vec2f, d.f32, GroundPaved],
    d.vec3f,
  )(/* wgsl */ `(xy:vec2f,footprint:f32,paved:GroundPaved)->vec3f {
 let kind=roadLane(paved);
 if(kind<0||terrainLayout.$.params.view.y==1u){return vec3f(0.0);}
 let look=terrainLayout.$.params.roads[kind];
 let width=look.ruts.z;
 if(width<=0.0){return vec3f(0.0);}
 let shown=1.0-smoothstep(${tuning.ruts.fade[0]},${tuning.ruts.fade[1]},footprint/width);
 if(shown<=0.0){return vec3f(0.0);}
 let half=paved.lane.w;
 var depth=0.0;var rise=0.0;var ruts=0.0;
 for(var i=0;i<2;i++){
  let at=look.ruts[i];
  if(at<=0.0||at+width>half){continue;}
  ruts+=2.0;
  let u=(paved.lane.z-at)/(width*0.5);
  if(abs(u)<1.0){let b=1.0-u*u;depth=b*b;rise=4.0*u*b;}
 }
 if(ruts==0.0){return vec3f(0.0);}
 let faint=mix(${tuning.ruts.faint},1.0,smoothstep(${tuning.ruts.breakCut[0]},${tuning.ruts.breakCut[1]},wanderNoise(xy*${1 / tuning.ruts.breakM}+vec2f(13.9,57.3))))*shown;
 let mean=ruts*width*${RUT_MEAN}/(2.0*half);
 return vec3f(paved.lane.xy*(look.ruts.w*rise*${1 / RUT_STEEPEST}*faint),look.track.x*(mean-depth)*faint);
}`)
  .$uses({ terrainLayout, wanderNoise, roadLane, GroundPaved });

/** How worn the ground is by the roads at `xy`, and by which kind of road:
 *  `(wear, kind's tag)`. On a road's own surface the wear is 1, less on a
 *  track's grass centre strip (`groundStrip`); beside a road it is 1 at the
 *  edge and 0 past its shoulder. A shoulder is its kind's `width_m` at the
 *  widest; its outer edge wanders inward from there by noise fixed to the
 *  ground (never outward, so nothing is read past the width), and the wear
 *  fades to that edge, broken toward it by tufts of the field. The ground's
 *  colour and the grass both read it: grass thins exactly where the ground
 *  shows worn. `paved` is the point's `groundPaved`. */
export const groundShoulder = tgpu
  .fn(
    [d.vec2f, d.f32, GroundPaved],
    d.vec2f,
  )(/* wgsl */ `(xy:vec2f,footprint:f32,paved:GroundPaved)->vec2f {
 var worn=vec2f(0.0);
 let plain=terrainLayout.$.params.view.y==1u;
 for(var k=0u;k<${ROAD_KINDS}u;k++){
  let look=terrainLayout.$.params.roads[k];
  let width=look.shoulder.w;
  let out=-paved.drawn[k];
  if(out<=0.0){worn=vec2f(1.0,f32(k));continue;}
  if(plain||out>=width){continue;}
  let wander=saturate((wanderNoise(xy*look.edge.y+vec2f(27.3,88.1))-0.5)*${tuning.shoulder.wander}+0.5);
  let reach=width*(1.0-look.edge.x*wander);
  let across=saturate(out/reach);
  let tufts=1.0-across*(1.0-smoothstep(${tuning.shoulder.tufts[0]},${tuning.shoulder.tufts[1]},wanderNoise(xy*${1 / tuning.shoulder.tuftM}+vec2f(5.9,63.1))));
  let wear=(1.0-smoothstep(${tuning.shoulder.whole},1.0,out/reach))*tufts*(1.0-smoothstep(${tuning.shoulder.pixels[0]},${tuning.shoulder.pixels[1]},footprint/width));
  if(wear>worn.x){worn=vec2f(wear,f32(k));}
 }
 let strip=groundStrip(xy,footprint,paved);
 if(strip>0.0){worn=vec2f(1.0-strip*${tuning.strip.grass},f32(roadLane(paved)));}
 return worn;
}`)
  .$uses({ terrainLayout, wanderNoise, groundStrip, roadLane, GroundPaved });

/** The share of a field's grass that grows on ground worn `worn`
 *  (`groundShoulder`): all of it off the shoulder, its kind's `grass` share
 *  where the wear is whole. (Where the wear is 1 the ground is a road's own
 *  surface, or the foot of its shoulder: the grass leaves that bare.) */
export const groundShoulderGrass = tgpu
  .fn(
    [d.vec2f],
    d.f32,
  )(/* wgsl */ `(worn:vec2f)->f32 {
 return mix(1.0,terrainLayout.$.params.roads[u32(worn.y)].edge.z,worn.x);
}`)
  .$uses({ terrainLayout });

/** How much of a joint `width` wide lies `along` metres along a line, 0 to
 *  1, where one crosses every `1 / perMetre` metres: drawn sharp, and gone
 *  once it is under a pixel or so (`slabs.jointPixels`). */
const strokeJoint = tgpu.fn(
  [d.f32, d.f32, d.f32, d.f32],
  d.f32,
)(/* wgsl */ `(along:f32,perMetre:f32,footprint:f32,width:f32)->f32 {
 let shown=1.0-smoothstep(${tuning.slabs.jointPixels[0]},${tuning.slabs.jointPixels[1]},footprint/width);
 if(perMetre<=0.0||shown<=0.0){return 0.0;}
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(width*0.5,width*0.5+footprint,to))*shown;
}`);

/** As `strokeJoint`, for an area's slabs, which are read from much farther
 *  off: a joint narrower than a pixel is spread over the pixel at its own
 *  share of it, so it thins to the surface's colour with distance instead
 *  of breaking into dashes. */
const areaJoint = tgpu.fn(
  [d.f32, d.f32, d.f32, d.f32],
  d.f32,
)(/* wgsl */ `(along:f32,perMetre:f32,footprint:f32,width:f32)->f32 {
 let drawn=max(width,footprint);
 let to=abs(fract(along*perMetre+0.5)-0.5)/perMetre;
 return (1.0-smoothstep(drawn*0.5-footprint*0.5,drawn*0.5+footprint*0.5,to))*(width/drawn);
}`);

/** How far the joint between two of a walk's slabs darkens a point: the
 *  slabs are laid along the stroke the walk runs beside, a joint across the
 *  walk every slab's length. */
const walkJoint = tgpu
  .fn(
    [d.f32, GroundPaved],
    d.f32,
  )(/* wgsl */ `(footprint:f32,paved:GroundPaved)->f32 {
 if(paved.run.w<0.0||terrainLayout.$.params.view.y==1u){return 0.0;}
 let road=terrainLayout.$.params.roads[u32(paved.run.w)];
 let beyond=paved.lane.z-paved.lane.w;
 if(beyond<=0.0||beyond>=road.join.z){return 0.0;}
 return strokeJoint(paved.run.z,road.slabs.x,footprint,${tuning.slabs.jointM})*road.slabs.y;
}`)
  .$uses({ terrainLayout, strokeJoint, GroundPaved });

/** The curb at a point: `(slope east, slope north, stones)`. Along the edge
 *  of a stroke whose kind has one, the kerbstones lie just outside the edge
 *  (`stones`: how much of the ground is theirs) and the step up to them just
 *  inside it, as the slope shading reads there (rise per metre; the ground
 *  itself is never moved). There is none where another road covers the
 *  edge: across a street's mouth, or where two streets meet. */
const groundCurb = tgpu
  .fn(
    [d.f32, GroundPaved],
    d.vec3f,
  )(/* wgsl */ `(footprint:f32,paved:GroundPaved)->vec3f {
 if(paved.run.w<0.0||terrainLayout.$.params.view.y==1u){return vec3f(0.0);}
 let own=u32(paved.run.w);
 let look=terrainLayout.$.params.roads[own];
 let width=look.curb.w;
 if(width<=0.0){return vec3f(0.0);}
 let inside=paved.drawn[own];
 if(inside<-2.0*width||inside>2.0*look.slabs.z){return vec3f(0.0);}
 let shown=1.0-smoothstep(${tuning.curb.pixels[0]},${tuning.curb.pixels[1]},footprint/width);
 if(shown<=0.0){return vec3f(0.0);}
 let soft=max(footprint,${tuning.curb.edgeM})*0.5;
 // Another carriageway's surface over the edge leaves no curb there.
 var open=1.0;
 for(var k=0u;k<${ROAD_KINDS}u;k++){
  if(k!=own&&terrainLayout.$.params.roads[k].track.w>0.0){open*=1.0-smoothstep(-soft,soft,paved.drawn[k]);}
 }
 let edge=smoothstep(-soft,soft,inside);
 let stones=smoothstep(-width-soft,-width+soft,inside)*(1.0-edge);
 let face=edge*(1.0-smoothstep(look.slabs.z-soft,look.slabs.z+soft,inside));
 return vec3f(paved.lane.xy*(look.slabs.w*face),stones)*(open*shown);
}`)
  .$uses({ terrainLayout, GroundPaved });

/** The slope a road's relief gives the shading normal at `xy` (rise per
 *  metre east and north): its ruts' sides and its curb's face. */
export const groundRoadRelief = tgpu.fn(
  [d.vec2f, d.f32, GroundPaved],
  d.vec2f,
)((xy, footprint, paved) => {
  "use gpu";
  return std.add(groundRuts(xy, footprint, paved).xy, groundCurb(footprint, paved).xy);
});

/** How much paint lies at `xy`, 0 to 1: the lines of the stroke whose lane
 *  the point is in, where that stroke's kind has markings. A dashed line
 *  runs down the stroke's middle, laid out by the distance along it; a dash
 *  that would reach a road that crosses, or its crossing, is left out whole
 *  (it is judged at the end of it nearer that road, as far as the road runs
 *  straight). Where a road does cross (a carriageway's stroke that runs on
 *  across this road's whole width, not one that ends in it), a crossing's
 *  bars lie on this road before it: along the road, side by side across it,
 *  clear of its edges. Every line is on its own road's surface, inside its
 *  edge: none reaches a walk. */
const groundMarks = tgpu
  .fn(
    [d.vec2f, d.f32, GroundPaved],
    d.f32,
  )(/* wgsl */ `(xy:vec2f,footprint:f32,paved:GroundPaved)->f32 {
 let kind=roadLane(paved);
 if(kind<0||terrainLayout.$.params.view.y==1u){return 0.0;}
 let look=terrainLayout.$.params.roads[kind];
 let line=look.marks.x;
 if(line<=0.0){return 0.0;}
 let bars=look.crossing;
 let shown=vec2f(1.0)-smoothstep(vec2f(${tuning.marks.pixels[0]}),vec2f(${tuning.marks.pixels[1]}),footprint/vec2f(2.0*line,bars.x));
 if(shown.x<=0.0&&shown.y<=0.0){return 0.0;}
 let far=bars.z+bars.y;
 // How far along this stroke the point is from its dash's middle.
 let dash=(fract(paved.run.z/look.marks.z)-0.5)*look.marks.z;
 // How far inside the nearest road that crosses this one the point's dash
 // reaches (near), and the point itself lies where that road runs on
 // across this road's width (cross).
 var near=-1e9;var cross=-1e9;
 for(var i=paved.list.x;i<paved.list.y;i++){
  let entry=terrainLayout.$.surfaceIndex[i];
  if((entry>>${SURFACE_KIND_SHIFT}u)!=${SURFACE_STROKE}u){continue;}
  let seg=terrainLayout.$.surfaces[entry&${SURFACE_RECORD_MASK}u];
  if(terrainLayout.$.params.roads[u32(seg.detail.y)].track.w<=0.0){continue;}
  let a=seg.ends.xy;let ab=seg.ends.zw-a;let len=max(length(ab),1e-6);
  if(abs(dot(ab,paved.run.xy))>${tuning.marks.crossingCos}*len){continue;}
  let free=dot(xy-a,ab)/len;
  let away=xy-(a+ab*(clamp(free,0.0,len)/len));let off=length(away);
  let inside=strokeCutInside(xy,seg.ends,seg.detail,seg.detail.x-off);
  // Toward the crossing road its inside grows by this much a metre along
  // this stroke.
  let nearer=dot(paved.run.xy,away)/max(off,1e-5);
  near=max(near,inside+nearer*dash+abs(nearer)*look.marks.y*0.5);
  let cuts=u32(seg.detail.z);
  let room=2.0*paved.lane.w;
  // Beside it: not past an end its stroke is cut at, nor within this
  // road's width of one (round a bend its stretches meet end to end), and
  // beside the stretch's own length: past its end the distance is to a
  // point, and a crossing laid by it would be a fan of bars round the end
  // of a street that only joins this road.
  let ends=((cuts&${CUT_A}u)!=0u&&free<room)||((cuts&${CUT_B}u)!=0u&&len-free<room);
  if(!ends&&free>=0.0&&free<=len){cross=max(cross,inside);}
 }
 let soft=max(footprint,${tuning.marks.edgeM})*0.5;
 let centre=(1.0-smoothstep(line-soft,line+soft,paved.lane.z))
  *(1.0-smoothstep(look.marks.y*0.5-soft,look.marks.y*0.5+soft,abs(dash)))
  *(1.0-step(-far-${tuning.marks.clearM},near));
 let zone=smoothstep(-far-soft,-far+soft,cross)*(1.0-smoothstep(-bars.z-soft,-bars.z+soft,cross));
 let across=abs(fract(paved.lane.z/(2.0*bars.x))-0.5)*2.0*bars.x;
 let bar=1.0-smoothstep(bars.x*0.5-soft,bars.x*0.5+soft,across);
 let kept=1.0-smoothstep(paved.lane.w-bars.w-soft,paved.lane.w-bars.w+soft,paved.lane.z);
 return max(centre*shown.x,zone*bar*kept*shown.y);
}`)
  .$uses({ terrainLayout, strokeCutInside, roadLane, GroundPaved });

/** The roads at `xy` over the ground `under` (linear albedo, roughness).
 *  First the worn shoulder beside them (`groundShoulder`), in its kind's
 *  colour: lifted to the luminance of the ground it lies on where that is the
 *  brighter, so worn ground differs from the field by hue and is never a
 *  darker band along the road. Then each row's paving as its own surface
 *  (`biome.roads`), feathered across its edge over a pixel at least, the
 *  lower layers under the higher, so a track ends at the edge of the road it
 *  joins, and a street's walk lies under every road; there the track's earth
 *  is carried a way onto the road (its own `join_m`), thinning out. A surface
 *  is its colour, in patches a second hue at the same brightness, under its
 *  grain; along a stroke's lanes it is shaded by its ruts (`groundRuts`), and
 *  a narrow track's middle goes to the verge's grass (`groundStrip`); a walk
 *  is crossed by its slabs' joints (`walkJoint`), an area by its own slabs'
 *  joints, square to its outline's dominant bearing (`areaBearings`). Last a
 *  street's painted lines (`groundMarks`), worn in patches, and its
 *  kerbstones along its edge (`groundCurb`). `paved` is the point's `groundPaved`. */
const groundRoads = tgpu
  .fn(
    [d.vec2f, d.f32, GroundPaved, d.vec4f],
    d.vec4f,
  )(/* wgsl */ `(xy:vec2f,footprint:f32,paved:GroundPaved,under:vec4f)->vec4f {
 var surface=under;
 let plain=terrainLayout.$.params.view.y==1u;
 let worn=groundShoulder(xy,footprint,paved);
 if(worn.x>0.0){
  let look=terrainLayout.$.params.roads[u32(worn.y)];
  let luma=vec3f(0.2126,0.7152,0.0722);
  let lift=max(1.0,dot(under.xyz,luma)/max(dot(look.shoulder.xyz,luma),1e-5));
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w)*${tuning.shoulder.grain};
  surface=mix(under,vec4f(look.shoulder.xyz*lift*(1.0+look.shape.z*grain),look.core.w),worn.x*look.edge.w);
 }
 let lane=roadLane(paved);
 // How far the roads already laid here are carried onto the next: each
 // one's own carry, by how much of the ground it covers.
 var carried=0.0;
 for(var n=0u;n<${ROAD_KINDS}u;n++){
  let k=terrainLayout.$.params.roadOrder[n];
  let look=terrainLayout.$.params.roads[k];
  var feather=max(look.shape.x,footprint)*0.5;
  var inside=paved.drawn[k];
  if(carried>0.0){
   // The blend starts at this road's edge and runs inward, from a line
   // that wanders as far either way: gravel spilt over asphalt in drifts,
   // never a ruled end across a road.
   feather=max(feather,carried*0.5);
   inside-=feather;
   if(!plain){inside+=(wanderNoise(xy*${1 / tuning.surface.joinDriftM}+vec2f(7.9,52.3))-0.5)*2.0*carried;}
  }
  if(inside<=-feather){continue;}
  var core=look.core.xyz;
  if(!plain){
   let hue=smoothstep(${tuning.surface.patchCut[0]},${tuning.surface.patchCut[1]},wanderNoise(xy*look.shape.y+vec2f(61.7,17.3)))*look.worn.w;
   let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
   core=mix(core,look.worn.xyz,hue)*(1.0+look.shape.z*grain);
   if(i32(k)==lane){
    core*=1.0+groundRuts(xy,footprint,paved).z;
    let strip=groundStrip(xy,footprint,paved)*${tuning.strip.cover};
    core=mix(core,terrainLayout.$.params.verge.xyz*(1.0+look.shape.z*grain*${tuning.shoulder.grain}),strip);
   }
   else if(lane<0){
    // Beside a street, its walk and the walk's slabs; past the walk, an
    // area's own slabs, on its own grid (SURFACE_AREA_BEARING), with a
    // deeper joint along the walk's outer edge (the carriageway's, where it
    // has none), so the walk or the road reads as its own strip.
    var out=1e9;
    if(paved.run.w>=0.0){
     let street=terrainLayout.$.params.roads[u32(paved.run.w)];
     out=paved.lane.z-paved.lane.w-street.join.z;
     if(out<0.0&&u32(street.join.w)==k){core*=1.0-walkJoint(footprint,paved);}
    }
    if(out>=0.0&&look.stones.z>0.0&&paved.bearing>=0.0&&!plain){
     // A plain border a slab wide (at most slabs.areaBorderM) runs along the
     // edge line before the grid starts: where the grid is not square to
     // the street, it meets the border, not the kerb.
     let border=1.0-smoothstep(0.0,footprint,out-min(1.0/look.stones.z,${tuning.slabs.areaBorderM}));
     let w=${tuning.slabs.areaJointM};
     let along=vec2f(cos(paved.bearing),sin(paved.bearing));
     // The grid fades out before a slab is a few pixels across, so far
     // paving reads as its colour rather than a shimmering mesh.
     let near=(1.0-smoothstep(${tuning.slabs.areaSlabPixels[0]},${tuning.slabs.areaSlabPixels[1]},footprint*look.stones.z))*(1.0-border);
     let joint=near*max(areaJoint(dot(xy,along),look.stones.z,footprint,w),areaJoint(dot(xy,vec2f(-along.y,along.x)),look.stones.z,footprint,w));
     let drawn=max(w,footprint);
     let edge=(1.0-smoothstep(drawn-footprint*0.5,drawn+footprint*0.5,out))*(w/drawn);
     core*=1.0-max(joint,edge*${tuning.slabs.areaEdgeDepth})*look.stones.w;
    }
   }
  }
  let on=smoothstep(-feather,feather,inside);
  surface=mix(surface,vec4f(core,look.core.w),on);
  // Only a carriageway is carried onto the road it joins.
  if(look.track.w>0.0){carried=max(carried,look.join.x*on);}
 }
 let paint=groundMarks(xy,footprint,paved);
 if(paint>0.0){
  let look=terrainLayout.$.params.roads[u32(paved.run.w)];
  let worn=1.0-look.marks.w*smoothstep(${tuning.marks.wearCut[0]},${tuning.marks.wearCut[1]},wanderNoise(xy*${1 / tuning.marks.wearM}+vec2f(33.7,71.1)));
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  surface=mix(surface,vec4f(look.paint.xyz*(1.0+look.shape.z*grain),look.core.w),paint*look.paint.w*worn);
 }
 let stones=groundCurb(footprint,paved).z;
 if(stones>0.0){
  let look=terrainLayout.$.params.roads[u32(paved.run.w)];
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  let joint=strokeJoint(paved.run.z,look.stones.x,footprint,${tuning.slabs.jointM})*look.stones.y;
  surface=mix(surface,vec4f(look.curb.xyz*(1.0+look.shape.z*grain)*(1.0-joint),look.core.w),stones);
 }
 return surface;
}`)
  .$uses({
    terrainLayout,
    wanderNoise,
    roadGrain,
    groundShoulder,
    groundRuts,
    groundStrip,
    groundCurb,
    groundMarks,
    walkJoint,
    strokeJoint,
    areaJoint,
    roadLane,
    GroundPaved,
  });

/** Linear albedo and roughness of the ground at `xy` with site `site`,
 *  paving `paved` (`groundPaved`) and water depth `water`, `footprint` the
 *  metres one pixel spans there. */
export const groundColour = tgpu.fn(
  [d.vec2f, d.f32, d.vec4f, GroundPaved, d.f32],
  d.vec4f,
)((xy, footprint, site, paved, water) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const aa = footprint * 0.5;
  const plot = terrainLayout.$.plots[d.i32(site.x)];
  const variation = mottle(xy, footprint, plot.rows.xy, site.x, plot.dry.xy);
  const noise = variation.x;
  let albedo = groundTint(
    std.mul(plot.colour.xyz, fieldTexture(xy, footprint, d.i32(site.x))),
    plot.detail.x * variation.y,
  );
  let roughness = plot.colour.w;

  const verge = groundVerge(site, footprint);
  const vergeColour = std.mul(params.verge.xyz, 1 + tuning.verge.mottle * noise);
  albedo = std.mix(albedo, vergeColour, verge);
  roughness = std.mix(roughness, tuning.verge.roughness, verge);

  // Past the patchwork, and where plots shrink to a few pixels, the distant
  // land: its mean colour.
  const inRegion = rectInside(xy, params.region);
  const distant = std.max(
    1 - std.smoothstep(0, tuning.distant.fadeM, inRegion),
    std.smoothstep(params.feathers.y, params.feathers.z, footprint),
  );
  albedo = std.mix(albedo, params.distant.xyz, distant);

  // The forest floor, over the simulation's forest shapes and their verge;
  // under a tree line, a band of the plots' verge instead.
  const floor = groundFloor(xy, footprint, site.w);
  if (floor.x > 0) {
    albedo = std.mix(albedo, forestFloor(xy, footprint, noise), floor.x);
    roughness = std.mix(roughness, params.forestDetail.z, floor.x);
  }
  if (floor.y > 0) {
    albedo = std.mix(albedo, vergeColour, floor.y);
    roughness = std.mix(roughness, tuning.verge.roughness, floor.y);
  }

  // The shore: bare earth along the water, wet silt at its edge. Neither is
  // darker than the field it lies on (a dark shore reads as a shadow on it):
  // they differ from it in hue. The field is the plot's own colour, its rows
  // at their mean (a furrow's stripe is not the bank's), eased to the
  // verge's toward the plot's edge.
  const shore = groundShore(xy, footprint, water);
  if (shore.y > 0) {
    const field = std.mix(
      std.dot(params.verge.xyz, LUMA),
      std.dot(plot.colour.xyz, LUMA) * (1 - 0.5 * plot.rows.w),
      std.smoothstep(0, tuning.shore.fieldEaseM, site.y),
    );
    const least = field * params.shoreEdge.z;
    const earth = std.mul(params.shoreEarth.xyz, 1 + tuning.shore.earthMottle * noise);
    albedo = std.mix(albedo, atLeast(earth, least), shore.y);
    roughness = std.mix(roughness, tuning.shore.earthRoughness, shore.y);
    albedo = std.mix(albedo, atLeast(params.shore.xyz, least), shore.x);
    roughness = std.mix(roughness, tuning.shore.wetRoughness, shore.x);
  }

  // The roads, over all but water.
  const paving = groundRoads(xy, footprint, paved, d.vec4f(albedo, roughness));

  // The water bed, under the simulation's rivers (water wins over road).
  const bed = std.smoothstep(-aa, std.max(aa, tuning.shore.bedEaseM), water);
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
  } else {
    const floor = groundFloor(xy, footprint, site.w);
    if (std.max(floor.x, floor.y) > 0.5) {
      forest = d.u32(1);
    }
  }
  const kind = std.min(d.u32(plot.detail.y), d.u32(CLASS_KIND_MAX));
  const hash = pcgHash(d.u32(site.x)) & CLASS_HASH_MASK;
  const packed = (forest << CLASS_FOREST_SHIFT) | (kind << CLASS_KIND_SHIFT) | hash;
  return d.vec3f(classDistance(site.z), classDistance(water), d.f32(packed) / 255);
});

// The water surface's look (presentation, not rules; the biome's `water`
// row): clear at its edge, where the bed shows through, taking on the
// channel's colour and opacity over the shallows; smooth enough that sky and
// sun reflect in it, broken by two octaves of ripples. The surface ends on the
// simulation's water edge, feathered over a pixel: the ground meets the
// surface there, so nothing hides it and nothing steps. Its numbers are
// `terrainTuning.water`, which the world pass reads too.

/** Ripple height at `xy`: two octaves of value noise, in about [0, 1.5]. */
const rippleHeight = tgpu.fn(
  [d.vec2f],
  d.f32,
)((xy) => {
  "use gpu";
  return (
    valueNoise(std.mul(xy, 1 / tuning.water.ripple.longM)) +
    tuning.water.ripple.shortWeight *
      valueNoise(std.add(std.mul(xy, 1 / tuning.water.ripple.shortM), d.vec2f(17.3, 5.1)))
  );
});

/** The water surface at `xy`: linear colour and opacity (clear at the shore,
 *  murky out in the channel, `groundWater` giving how far in it lies), none
 *  outside the water's edge. Streaks of light lie on it in lanes along the
 *  stream: a reflection shows the ripples only where the sun or the sky's
 *  bright edge happens to lie behind the water, and without the streaks a
 *  reach seen from above is a flat band. A pixel wider than a ripple takes
 *  their mean, so the water keeps its value as the camera pulls out.
 *  `footprint` is the metres one pixel spans. */
export const waterSurface = tgpu.fn(
  [d.vec2f, d.f32],
  d.vec4f,
)((xy, footprint) => {
  "use gpu";
  const params = terrainLayout.$.params;
  const water = groundWater(xy, groundCell(xy, footprint));
  const deep = 1 - std.exp(-std.max(water, 0) * params.waterLook.x);
  const shown =
    1 - std.smoothstep(tuning.water.ripple.fadeM[0], tuning.water.ripple.fadeM[1], footprint);
  const drift = tuning.water.lanes.drift * valueNoise(std.mul(xy, 1 / tuning.water.lanes.driftM));
  const lane = valueNoise(d.vec2f(water / tuning.water.lanes.m + drift, 3.7));
  const streak = valueNoise(
    std.add(std.mul(xy, 1 / tuning.water.lanes.streakM), d.vec2f(63.1, 29.3)),
  );
  const lit = std.smoothstep(
    tuning.water.lanes.lit[0],
    tuning.water.lanes.lit[1],
    tuning.water.lanes.weights[0] * lane + tuning.water.lanes.weights[1] * streak,
  );
  const colour = std.mul(
    std.mix(params.waterEdge.xyz, params.water.xyz, deep),
    1 + params.waterLook.y * std.mix(tuning.water.lanes.litMean, lit, shown),
  );
  const edge = std.smoothstep(-footprint * 0.5, footprint * 0.5, water);
  return d.vec4f(colour, std.mix(params.waterEdge.w, params.water.w, deep) * edge);
});

/** The water's normal at `xy`: small ripples, fading out where a pixel spans
 *  more than a ripple (`footprint` metres), so far water lies flat and calm. */
export const waterNormal = tgpu.fn(
  [d.vec2f, d.f32],
  d.vec3f,
)((xy, footprint) => {
  "use gpu";
  const ex = d.vec2f(tuning.water.ripple.stepM, 0);
  const ey = d.vec2f(0, tuning.water.ripple.stepM);
  const dx = rippleHeight(std.add(xy, ex)) - rippleHeight(std.sub(xy, ex));
  const dy = rippleHeight(std.add(xy, ey)) - rippleHeight(std.sub(xy, ey));
  const k =
    (tuning.water.ripple.slope / (2 * tuning.water.ripple.stepM)) *
    (1 - std.smoothstep(tuning.water.ripple.fadeM[0], tuning.water.ripple.fadeM[1], footprint));
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
 *  edge, and flecks of ash near a burst. Shadows here are soft-edged and
 *  even; scars are hard-edged and ragged. The bowl and rim add a slope. Zero
 *  off the grid and where nothing is marked. */
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
  let r = ${tuning.scars.rimReachCells};
  let a = sampleGroundLinear(uv + texel * vec2f(-r, -r));
  let b = sampleGroundLinear(uv + texel * vec2f(r, -r));
  let c = sampleGroundLinear(uv + texel * vec2f(-r, r));
  let e = sampleGroundLinear(uv + texel * vec2f(r, r));
  let near = max(max(a, b), max(c, e));
  let s = sampleGroundCubic(uv);
  if (max(max(s.x, s.y), max(s.z, s.w)) + max(max(near.x, near.y), max(near.z, near.w)) <= 0.0) {
    return out;
  }
  let detail = 1.0 - smoothstep(${tuning.scars.detailFade[0]}, ${wgslFloat(tuning.scars.detailFade[1])}, footprint / cell);
  let k = P.full;
  let depth = min(s.x * k.x, ${tuning.scars.depthCap});
  let wide = min((a.x + b.x + c.x + e.x) * 0.25 * k.x, ${tuning.scars.depthCap});
  // Slopes per metre of the crater field, from taps either side.
  let h = ${tuning.scars.slopeTapCells};
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
  let grain = valueNoise(xy * ${1 / tuning.scars.lipWanderM} + vec2f(5.7, 2.1)) - 0.5;
  let pixelDepth = length(bowlSlope) * footprint;
  let bowl = crossing(depth, ${tuning.scars.lip[0]} + ${tuning.scars.lip[1]} * grain, pixelDepth);
  let skirt = crossing(depth, ${tuning.scars.ring[0]} + ${tuning.scars.ring[1]} * grain, pixelDepth) * (1.0 - bowl);
  // The deepest crater within the taps: a lone pock's skirt is scorched
  // earth, a real crater's is its thrown soil.
  let peak = max(max(max(s.x, xp.x), max(xm.x, yp.x)), max(ym.x, near.x)) * k.x;
  let soil = smoothstep(${tuning.scars.pock[0]}, ${tuning.scars.pock[1]}, peak);
  let ring = skirt * soil;
  let scorched = crossing(depth, ${tuning.scars.pockEdge[0]} + ${tuning.scars.pockEdge[1]} * grain, pixelDepth) * (1.0 - bowl);
  // Darkest at its heart, browner toward its edge, as a burn is.
  let heart = ${tuning.scars.pockEdgeDark} + ${1 - tuning.scars.pockEdgeDark} * smoothstep(${tuning.scars.pockEdge[0]}, ${tuning.scars.ring[0]}, depth);
  let pock = scorched * (1.0 - soil) * ${tuning.scars.pockSoot} * heart;
  // Soot: ash in flecks over the scorch, thickest near the burst, never a
  // solid sheet: a flecked dark is one thing no shadow here is.
  let burnt = saturate(s.y * k.y);
  let fleckAt = valueNoise(xy * ${tuning.scars.ash.scale} + vec2f(8.1, 3.3)) * ${tuning.scars.ash.weights[0]} + valueNoise(xy * ${tuning.scars.ash.scale * tuning.scars.ash.fine}) * ${tuning.scars.ash.weights[1]};
  let ash = crossing(fleckAt, mix(${tuning.scars.ash.edge[0]}, ${tuning.scars.ash.edge[1]}, burnt), footprint * ${tuning.scars.ash.scale}) * step(${tuning.scars.ash.least}, burnt);
  let soot = max(ash * (1.0 - bowl) * (1.0 - ring), pock);
  let wear = 1.0 - (1.0 - saturate(s.zw * k.zw)) * (1.0 - saturate(s.zw * k.zw));
  // Ruts have edges: tracks cross their iso-line within a pixel, ragged.
  let trackSlope = vec2f(xp.z - xm.z, yp.z - ym.z) * k.z / (2.0 * h * cell);
  let rut = crossing(wear.x, ${tuning.scars.rut[0]} + ${tuning.scars.rut[1]} * grain, length(trackSlope) * footprint);
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
  let run = -view.xy / max(view.z, ${tuning.scars.grazing});
  let deep = terrainLayout.$.scarParams.relief.x;
  var at = world.xy;
  for (var i = 0; i < 2; i++) {
    at = world.xy + run * min(scar.relief.w, ${tuning.scars.depthCap}) * deep * 0.5;
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
  let grain = valueNoise(xy * ${tuning.scarSurface.grainScale});
  let fleck = valueNoise(xy * ${tuning.scarSurface.fleckScale} + vec2f(2.2, 9.1));
  albedo = mix(albedo, P.trampled.xyz * mix(${tuning.scarSurface.trampled[0]}, ${tuning.scarSurface.trampled[1]}, grain), w.w * P.trampled.w);
  albedo = mix(albedo, P.tracks.xyz * mix(${tuning.scarSurface.tracks[0]}, ${tuning.scarSurface.tracks[1]}, grain), w.z * P.tracks.w);
  // Soot is uneven: ash grey and char black in flecks and drifts.
  let ash = mix(${tuning.scarSurface.ash[0]}, ${tuning.scarSurface.ash[1]}, fleck * ${tuning.scarSurface.ashWeights[0]} + valueNoise(xy * ${tuning.scarSurface.ashDriftScale} + vec2f(4.4, 0.3)) * ${tuning.scarSurface.ashWeights[1]});
  albedo = mix(albedo, P.scorch.xyz * ash, w.y * P.scorch.w);
  let ring = scar.relief.z * P.ejecta.w;
  albedo = mix(albedo, P.ejecta.xyz * mix(${tuning.scarSurface.soil[0]}, ${tuning.scarSurface.soil[1]}, fleck), ring);
  let bowl = w.x * P.crater.w;
  // The deeper, the darker: the floor sees less sky than the lip.
  let cavity = mix(1.0, ${tuning.scarSurface.cavity}, saturate(scar.relief.w));
  albedo = mix(albedo, P.crater.xyz * mix(${tuning.scarSurface.soil[0]}, ${tuning.scarSurface.soil[1]}, fleck) * cavity, bowl);
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
  let capped = slope * min(1.0, ${tuning.shading.maxSlope} / max(steep, 1e-6));
  return normalize(n - vec3f(capped, 0.0) * n.z);
}`);

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
  let roadWear = true;
  /** The plots `set` last packed, and whether they carry their texture. */
  let fields: PlotLook | null = null;
  let textured = true;
  /** The tables as last uploaded (`terrainTables.ts` reads them). */
  let packed: {
    grid: TerrainSurface["grid"];
    nodes: ArrayBuffer;
    plots: ArrayBuffer;
    surfaces: ArrayBuffer;
    surfaceIndex: ArrayBuffer;
  } | null = null;
  const paramsNow = () =>
    look && { ...look, view: d.vec4u(classView ? 1 : 0, roadWear ? 0 : 1, 0, 0) };
  const writeParams = () => {
    const now = paramsNow();
    if (now) params.write(now);
  };

  const source = {
    /** Draw the ground's classes in place of its lit colour, or not. */
    setClassView(on: boolean) {
      if (on === classView) return;
      classView = on;
      writeParams();
    },
    /** Draw the roads worn (their patches and grain, their shoulders), or
     *  plain: each kind's flat colour to its edge. True when it changed. */
    setRoadWear(on: boolean): boolean {
      if (on === roadWear) return false;
      roadWear = on;
      writeParams();
      return true;
    },
    /** Draw the plots' own texture (grain, broken rows, wheelings), or their
     *  plain rows alone: a paired cost measure, and a check that the texture
     *  leaves a plot's mean colour be. True when it changed, so what grows on
     *  the ground must regrow. */
    setFieldTexture(on: boolean): boolean {
      if (on === textured) return false;
      textured = on;
      if (fields && packed) {
        packed.plots = packPlots(fields, textured);
        plots.current!.write(packed.plots);
      }
      return true;
    },
    /** The words `set` uploaded and the params as last written; null before `set`. */
    uploaded: () => {
      const now = paramsNow();
      return packed && now && { ...packed, params: now };
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
      scars.prepare(region, 2 + (2 * tuning.scars.depthCap * scarLook.relief.x) / cellM);
      writeScarParams();
      if (scars.directory !== priorDirectory) source.group = groupOf();
    },
    scarRegions(): readonly ScarRegion[] {
      return scars.hasMarks() && !scars.globalEligible() ? regions : EMPTY_SCAR_REGIONS;
    },
    set(surface: TerrainSurface) {
      const { plots: tree, site, biome } = surface;
      fields = { plots: tree, biome };
      const field = terrainField(surface);
      packed = {
        grid: surface.grid,
        nodes: packNodes(tree),
        plots: packPlots(fields, textured),
        surfaces: field.records.buffer as ArrayBuffer,
        surfaceIndex: field.index.buffer as ArrayBuffer,
      };
      nodes.set(nodeBuffer(tree.nodes.length / NODE_FLOATS)).write(packed.nodes);
      plots.set(plotBuffer(tree.plots.length)).write(packed.plots);
      surfaces.set(surfaceBuffer(field.records.length / SURFACE_FLOATS)).write(packed.surfaces);
      surfaceIndex.set(indexBuffer(field.index.length)).write(packed.surfaceIndex);
      const rules = biome.field_rules;
      const one = (key: string) => linearRgb(biome.palettes[key][0]);
      const bank = biome.palettes[biome.shore.palette];
      look = {
        region: d.vec4f(...tree.region),
        field: d.vec4f(field.origin[0], field.origin[1], 1 / field.cellM, field.footprintM),
        fieldGrid: d.vec4u(field.cols, field.rows, field.levels, 0),
        shape: d.vec4f(rules.edge_warp_m, 1 / rules.edge_warp_scale_m, 1 / rules.mottle_m, 0),
        verge: d.vec4f(...one(biome.verge.palette), biome.verge.width_m / 2),
        feathers: d.vec4f(
          biome.verge.feather_m,
          rules.size_m[0] * tuning.distant.plotPixels[0],
          rules.size_m[0] * tuning.distant.plotPixels[1],
          0,
        ),
        roads: roadLooks(biome),
        roadOrder: d.vec4u(...roadOrder(biome)),
        ...forestParams(biome.forest_floor, biome.palettes[biome.forest_floor.palette]),
        waterBed: d.vec4f(...one("water_bed"), triangleReach(site)),
        water: d.vec4f(...one("water"), biome.water.opacity[1]),
        waterEdge: d.vec4f(...linearRgb(biome.palettes.water[1]), biome.water.opacity[0]),
        waterLook: d.vec4f(1 / biome.water.shallows_m, biome.water.streak, 0, 0),
        shore: d.vec4f(...linearRgb(bank[0]), biome.shore.wet_m),
        shoreEarth: d.vec4f(...linearRgb(bank[1]), biome.shore.mud_m),
        shoreEdge: d.vec4f(
          biome.shore.wander,
          1 / biome.shore.wander_scale_m,
          biome.shore.lift,
          biome.shore.relief,
        ),
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

/** `surface`'s field as the material reads it: its site's ground rules, with
 *  the paved strokes as they are drawn (`TerrainSurface.strokes`). The
 *  material and any check of its lookups both build this. */
export function terrainField(surface: TerrainSurface): SurfaceField {
  return buildSurfaceField(
    { ...surface.site, surfaceStrokes: surface.strokes },
    terrainReach(surface),
  );
}

/** How far each ground rule of `surface` is read at a pixel `footprint`
 *  metres wide: what its surface field is built with. */
export function terrainReach({ site, biome }: TerrainSurface) {
  const bankM = longestBank(site.rivers) + triangleReach(site);
  return (footprint: number) => groundReach(biome, footprint, bankM);
}

/** How far from its edge each ground rule is read by a pixel `footprint`
 *  metres wide, in metres: the surface field lists what lies within it, and a
 *  distance past it only keeps its side. Each term is one reader:
 *
 *  - paved: the shoulder beside a road (`groundShoulder`: its width at the
 *    widest), which the grass thins across too, or the walk beside a street
 *    (`groundPaved`: its width); the road's own edge
 *    (`groundRoads`: half a feather either side, a pixel wide at least); the
 *    far end of a crossing's bars, and a centre line's dash that ends short
 *    of them, from the edge of the road that crosses (`groundMarks`);
 *  - forest: the floor's ragged verge (`forestVergeInside` moves the edge out
 *    by up to the verge and its warp, `forestFloorWeight` feathers it by a
 *    pixel at least), which `groundDapple` and the grass read too, the grass
 *    thinning past its margin;
 *  - water: the shore's bands (`groundShore`: the bare earth ends within
 *    `mud_m`), the bed's and the water surface's pixel-wide edge, the grass's
 *    margin, and the bank's shading (`groundBank`) out to `bankM` from the
 *    water: the ground's longest bank and a triangle past its top. Inside
 *    the water a stretch lists itself, so the surface's shallows need none. */
export function groundReach(biome: Biome, footprint: number, bankM = 0): SurfaceReach {
  const floor = biome.forest_floor;
  const grass = biome.grass.clear_m;
  return {
    paved: Math.max(
      ...Object.values(biome.roads).map((road) => road.shoulder.width_m),
      ...Object.values(biome.roads).map(({ markings }) =>
        markings
          ? markings.crossing.gap_m +
            markings.crossing.length_m +
            tuning.marks.clearM +
            markings.dash_m[0]
          : 0,
      ),
      Math.max(...Object.values(biome.roads).map((road) => road.walk?.width_m ?? 0)) +
        Math.max(...Object.values(biome.roads).map((road) => road.feather_m), footprint) / 2,
    ),
    forest:
      floor.verge_m +
      floor.verge_warp_m +
      Math.max(footprint, floor.verge_m * tuning.forestVerge.feather, grass.area + GRASS_EDGE_M),
    water: Math.max(
      biome.shore.mud_m,
      footprint / 2,
      bankM + 2 * Math.max(footprint, tuning.bank.easeM),
      grass.area + GRASS_EDGE_M,
    ),
  };
}

const luminance = (c: readonly number[]) =>
  c[0] * LUMA_WEIGHTS[0] + c[1] * LUMA_WEIGHTS[1] + c[2] * LUMA_WEIGHTS[2];

/** The row of the look table for the paved kind tagged `tag`, drawn as
 *  `road`. The patches' colour is scaled to the surface's own luminance: a
 *  patch is a change of hue alone. */
function roadLook(road: Road, tag: number, palettes: Biome["palettes"]) {
  const [core, worn] = palettes[road.palette].map(linearRgb);
  const level = luminance(core) / luminance(worn);
  const row = (kind: string | undefined) =>
    kind === undefined ? tag : SURFACE_AREA_KINDS.indexOf(kind as SurfaceAreaKind);
  const marks = road.markings;
  return {
    core: d.vec4f(...core, road.roughness),
    worn: d.vec4f(worn[0] * level, worn[1] * level, worn[2] * level, road.mottle),
    shape: d.vec4f(road.feather_m, 1 / road.patch_m, road.grain, 1 / road.grain_m),
    join: d.vec4f(road.join_m, 0, road.walk?.width_m ?? 0, row(road.walk?.kind)),
    shoulder: d.vec4f(...linearRgb(palettes[road.shoulder.palette][0]), road.shoulder.width_m),
    edge: d.vec4f(
      road.shoulder.jitter,
      1 / road.shoulder.jitter_m,
      road.shoulder.grass,
      road.shoulder.cover,
    ),
    ruts: d.vec4f(
      road.ruts.offsets_m[0] ?? 0,
      road.ruts.offsets_m[1] ?? 0,
      road.ruts.width_m,
      Math.tan((road.ruts.tilt_deg * Math.PI) / 180),
    ),
    track: d.vec4f(
      road.ruts.tint,
      road.centre_strip.half_width_m,
      road.centre_strip.max_road_width_m / 2,
      isRoad(tag) ? 1 : 0,
    ),
    slabs: d.vec4f(
      road.walk?.slab_m ? 1 / road.walk.slab_m : 0,
      road.walk?.joint ?? 0,
      road.curb?.face_m ?? 0,
      road.curb ? Math.tan((road.curb.tilt_deg * Math.PI) / 180) : 0,
    ),
    curb: road.curb
      ? d.vec4f(...linearRgb(palettes[road.curb.palette][0]), road.curb.width_m)
      : d.vec4f(0),
    stones: d.vec4f(
      road.curb ? 1 / road.curb.stone_m : 0,
      road.curb?.joint ?? 0,
      road.slabs ? 1 / road.slabs.slab_m : 0,
      road.slabs?.joint ?? 0,
    ),
    paint: marks ? d.vec4f(...linearRgb(palettes[marks.palette][0]), marks.cover) : d.vec4f(0),
    marks: marks
      ? d.vec4f(marks.line_m / 2, marks.dash_m[0], marks.dash_m[0] + marks.dash_m[1], marks.wear)
      : d.vec4f(0, 0, 1, 0),
    crossing: marks
      ? d.vec4f(
          marks.crossing.bar_m,
          marks.crossing.length_m,
          marks.crossing.gap_m,
          marks.crossing.inset_m,
        )
      : d.vec4f(1, 0, 0, 0),
  };
}

/** The look table in native tag order, selected by each authored kind. */
export function roadLooks(biome: Biome) {
  return SURFACE_AREA_KINDS.map((kind, tag) => roadLook(roadRow(biome, kind), tag, biome.palettes));
}

/** Paved tags from lowest layer to highest, with native order breaking ties. */
export function roadOrder(biome: Biome): [number, number, number, number] {
  const layer = (tag: number) => roadRow(biome, SURFACE_AREA_KINDS[tag]).layer ?? ROAD_KINDS - tag;
  const [a, b, c, e] = SURFACE_AREA_KINDS.map((_, tag) => tag).sort(
    (x, y) => layer(x) - layer(y) || y - x,
  );
  return [a, b, c, e];
}

/** The forest floor's uniform fields: its palette's litter, moss and humus. */
function forestParams(floor: ForestFloor, [litter, moss, humus]: readonly Rgb[]) {
  return {
    forestLitter: d.vec4f(...linearRgb(litter), 0),
    forestMoss: d.vec4f(...linearRgb(moss), floor.moss),
    forestHumus: d.vec4f(...linearRgb(humus), floor.humus),
    forestDetail: d.vec4f(1 / floor.patch_m, floor.mottle, floor.roughness, floor.roots),
    forestVerge: d.vec4f(
      floor.verge_m,
      floor.verge_warp_m,
      1 / floor.verge_warp_scale_m,
      1 / floor.roots_m,
    ),
    forestDapple: d.vec4f(1 / floor.dapple.size_m, floor.dapple.share, floor.dapple.sun, 0),
    forestLine: d.vec4f(floor.tree_line.taper_m, floor.tree_line.warp_m, 0, 0),
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

/** What the plots' records are packed from. */
type PlotLook = Pick<TerrainSurface, "plots" | "biome">;

/** The plots' records; without `textured`, their rows and dry patches alone. */
function packPlots({ plots: tree, biome }: PlotLook, textured: boolean): ArrayBuffer {
  const f = new Float32Array(Math.max(1, tree.plots.length) * (PLOT_BYTES / 4));
  tree.plots.forEach((plot, k) => {
    const kind = biome.plots[plot.kind];
    f.set(
      [
        ...linearRgb(plot.colour),
        kind.roughness,
        plot.across[0],
        plot.across[1],
        kind.furrow_m,
        kind.furrow_contrast,
        kind.mottle,
        plot.kind,
        textured ? kind.row_break : 0,
        0,
        1 / kind.grain_m,
        textured ? kind.grain : 0,
        1 / kind.grain_stretch,
        0,
        1 / kind.patch_m[0],
        1 / kind.patch_m[1],
        0,
        0,
        textured ? kind.tram.rows : 0,
        kind.tram.width_m / 2,
        kind.tram.contrast,
        0,
      ],
      k * (PLOT_BYTES / 4),
    );
  });
  return f.buffer;
}
