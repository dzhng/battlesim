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
// (`waterSurface`) and the side's learned scars (`groundScars.ts`).
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
// `presentation.terrain` in `fixtures/game.json` (`crates/presentation`'s
// `ground::tuning` documents each), spliced into the WGSL as a constant.
//
// Rewritten from reading ~/dev/game
// battle-renderer/src/shaders/terrainMaterial.ts: the mottle, drift and
// feathered road-edge ideas; not its baked distance texture or rock layers.
import { tgpu, d, std } from "typegpu";
import { pcgHash } from "../shaders/pcgHash";
import {
  CUT_A,
  CUT_B,
  SURFACE_CELL_WORDS,
  SURFACE_KIND_SHIFT,
  SURFACE_LEVEL_WORDS,
  SURFACE_NEW_SHAPE,
  SURFACE_RECORD_MASK,
  SURFACE_RECT,
  SURFACE_STROKE,
  SURFACE_TRIANGLE,
} from "../terrain/surfaceField";
import {
  CLASS_EDGE,
  CLASS_FOREST_SHIFT,
  CLASS_HASH_MASK,
  CLASS_KIND_MAX,
  CLASS_KIND_SHIFT,
  CLASS_STEPS_PER_M,
} from "../terrain/groundClasses";
import game from "@fixtures/game.json";
import { MAX_PLOT_DEPTH, ROAD_KINDS, terrainLayout } from "./terrainLayout";
const tuning = game.presentation.terrain;
/** Rec. 709's weights: a linear colour's luminance is its dot with them. */
const LUMA_WEIGHTS = [0.2126, 0.7152, 0.0722] as const;
const LUMA = d.vec3f(...LUMA_WEIGHTS);
const [dryR, dryG, dryB] = tuning.mottle.dry;
const MOTTLE_DRY = d.vec3f(dryR, dryG, dryB);
/** A number as WGSL float text: a whole number keeps its `.0`. */
export const wgslFloat = (n: number) => (Number.isInteger(n) ? n.toFixed(1) : `${n}`);

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
    ((valueNoise(std.add(at, d.vec2f(tuning.mottle.slope_step, 0))) - streak) /
      tuning.mottle.slope_step) *
      reach.x,
    ((valueNoise(std.add(at, d.vec2f(0, tuning.mottle.slope_step))) - streak) /
      tuning.mottle.slope_step) *
      reach.y,
  );
  const inside = (streak - tuning.mottle.patch_cut) / std.max(std.length(slope), 1e-4);
  const edge = std.max(footprint, tuning.mottle.patch_edge_m) * 0.5;
  const parched = std.smoothstep(-edge, edge, inside);
  const fine = valueNoise(std.add(std.mul(xy, shape.z), d.vec2f(37.1, 11.3))) - 0.5;
  const fineShown =
    1 - std.smoothstep(tuning.mottle.fine_fade[0], tuning.mottle.fine_fade[1], footprint * shape.z);
  return d.vec2f(
    fine * tuning.mottle.fine_value * fineShown,
    parched * tuning.mottle.patch_hue + std.max(fine, 0) * tuning.mottle.fine_hue * fineShown,
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
  if (nth != 0.0 && nth != ${wgslFloat(tuning.wheeling.gauge_rows)}) { return 0.0; }
  let off = abs(phase - furrow) * rows.z;
  let pixel = max(footprint, ${tuning.wheeling.least_pixel_m});
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
    let along = grain.x * grain.z / ${wgslFloat(tuning.rows.break_grains)};
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
 *  `detail` is the stretch's `(half width, tag, cuts)`; `ground::shapes`'
 *  `stroke_inside` is this on the CPU. An uncut stretch keeps `inside` as its
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
    valueNoise(std.add(std.mul(xy, tuning.forest_verge.patch_scale), d.vec2f(5.1, 91.7))) - 0.5;
  return forest + verge.x * tuning.forest_verge.inset + wander * verge.y + patches * verge.x;
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
    terrainLayout.$.params.forestVerge.x * tuning.forest_verge.feather,
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
      const feather = std.max(footprint, params.forestVerge.x * tuning.forest_verge.feather);
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
    tuning.forest_floor.moss.cut[0],
    tuning.forest_floor.moss.cut[1],
    valueNoise(std.add(turned, d.vec2f(13.3, 7.7))) * tuning.forest_floor.moss.weights[0] +
      valueNoise(std.add(std.mul(across, tuning.forest_floor.moss.fine_scale), d.vec2f(4.1, 9.2))) *
        tuning.forest_floor.moss.weights[1],
  );
  const humus = std.smoothstep(
    tuning.forest_floor.humus.cut[0],
    tuning.forest_floor.humus.cut[1],
    valueNoise(std.add(std.mul(across, tuning.forest_floor.humus.scales[0]), d.vec2f(2.9, 41.1))) *
      tuning.forest_floor.humus.weights[0] +
      valueNoise(
        std.add(std.mul(turned, tuning.forest_floor.humus.scales[1]), d.vec2f(17.3, 5.9)),
      ) *
        tuning.forest_floor.humus.weights[1],
  );
  let floor = std.mix(params.forestLitter.xyz, params.forestMoss.xyz, moss * params.forestMoss.w);
  floor = std.mix(floor, params.forestHumus.xyz, humus * params.forestHumus.w);
  // Roots: thin dark lines where a noise field crosses its middle, broken
  // into short runs by a second field, fading to their mean below a few
  // pixels a line.
  const rootsAt = std.mul(xy, params.forestVerge.w);
  const ridge = std.abs(valueNoise(std.add(rootsAt, d.vec2f(29.1, 3.3))) - 0.5);
  const runs = std.smoothstep(
    tuning.forest_floor.roots.runs[0],
    tuning.forest_floor.roots.runs[1],
    valueNoise(std.add(std.mul(rootsAt, tuning.forest_floor.roots.run_scale), d.vec2f(8.3, 17.9))),
  );
  const shown =
    1 -
    std.smoothstep(
      tuning.forest_floor.roots.fade[0],
      tuning.forest_floor.roots.fade[1],
      footprint * params.forestVerge.w,
    );
  const line =
    (1 -
      std.smoothstep(tuning.forest_floor.roots.line[0], tuning.forest_floor.roots.line[1], ridge)) *
    runs;
  const root = line * shown + tuning.forest_floor.roots.mean * (1 - shown);
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
 *  (`Ground::reach`), and keep their side beyond. */
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
 *  point of the centreline (`ground::water`'s `stretch_inside`, and the
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
  let ease=max(footprint,${tuning.bank.ease_m});
  let sloped=(1.0-smoothstep(run-ease,run+ease,-inside))*(1.0-smoothstep(0.0,ease,inside));
  let top=run+terrainLayout.$.params.waterBed.w;
  let weight=exp(clamp(inside,-${tuning.bank.blend_reach_m},${tuning.bank.blend_reach_m})/${tuning.bank.blend_m});
  let shown=min(grade*terrainLayout.$.params.shoreEdge.w,${tuning.shading.max_slope});
  sum+=vec4f(away/max(off,1e-4)*shown*sloped,1.0-smoothstep(top,top+2.0*ease,-inside),1.0)*weight;
 }
 if(sum.w<=0.0){return vec3f(0.0);}
 return sum.xyz/sum.w;
}`)
  .$uses({ terrainLayout });

/** `xy` on a lattice turned from the map's axes by (cos, sin) `turn`. */
const turned = tgpu.fn(
  [d.vec2f, d.vec2f],
  d.vec2f,
)((xy, turn) => {
  "use gpu";
  return d.vec2f(xy.x * turn.x + xy.y * turn.y, xy.y * turn.x - xy.x * turn.y);
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
  const t = params.shoreTuning;
  const broad = valueNoise(std.mul(turned(xy, t.turns.xy), params.shoreEdge.y));
  const scale = params.shoreEdge.y * t.bands.z;
  const fine = std.mix(
    valueNoise(std.add(std.mul(turned(xy, t.turns.zw), scale), d.vec2f(41.7, 13.1))),
    0.5,
    std.smoothstep(t.fade.x, t.fade.y, footprint * scale),
  );
  const line = reach * (1 - params.shoreEdge.x * std.mix(broad, fine, t.bands.w));
  return d.vec3f(
    1 - std.smoothstep(wet * t.bands.x, wet, out),
    1 - std.smoothstep(std.mix(wet, line, t.bands.y), line, out),
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
 let push=vec2f(valueNoise(at*${tuning.wander.push_scale}+vec2f(11.3,47.1)),valueNoise(at*${tuning.wander.push_scale}+vec2f(83.9,5.7)))-0.5;
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
 let shown=vec4f(1.0)-smoothstep(vec4f(${tuning.road_grain.fade[0]}),vec4f(${tuning.road_grain.fade[1]}),pixel*vec4f(1.0,${tuning.road_grain.octaves.join(",")}));
 var grain=0.0;
 if(shown.x>0.0){grain+=(valueNoise(vec2f(0.87*p.x-0.5*p.y,0.5*p.x+0.87*p.y))-0.5)*${tuning.road_grain.weights[0]}*shown.x;}
 if(shown.y>0.0){grain+=(valueNoise(vec2f(0.6*p.x+0.8*p.y,-0.8*p.x+0.6*p.y)*${tuning.road_grain.octaves[0]}+vec2f(19.1,7.7))-0.5)*${tuning.road_grain.weights[1]}*shown.y;}
 if(shown.z>0.0){grain+=(valueNoise(vec2f(0.96*p.x-0.28*p.y,0.28*p.x+0.96*p.y)*${tuning.road_grain.octaves[1]}+vec2f(3.3,41.9))-0.5)*${tuning.road_grain.weights[2]}*shown.z;}
 if(shown.w>0.0){
  let stone=valueNoise(p*${tuning.road_grain.octaves[2]}+vec2f(71.9,13.7));
  grain+=(smoothstep(${tuning.road_grain.stones[0]},${tuning.road_grain.stones[1]},stone)-smoothstep(${tuning.road_grain.stones[0]},${tuning.road_grain.stones[1]},1.0-stone))*${tuning.road_grain.weights[3]}*shown.w;
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
 let reach=half*(1.0+(wanderNoise(xy*${1 / tuning.strip.wander_m}+vec2f(41.3,9.7))-0.5)*${tuning.strip.wander});
 let soft=max(footprint,${tuning.strip.edge_m});
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
 let faint=mix(${tuning.ruts.faint},1.0,smoothstep(${tuning.ruts.break_cut[0]},${tuning.ruts.break_cut[1]},wanderNoise(xy*${1 / tuning.ruts.break_m}+vec2f(13.9,57.3))))*shown;
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
  let tufts=1.0-across*(1.0-smoothstep(${tuning.shoulder.tufts[0]},${tuning.shoulder.tufts[1]},wanderNoise(xy*${1 / tuning.shoulder.tuft_m}+vec2f(5.9,63.1))));
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
 let shown=1.0-smoothstep(${tuning.slabs.joint_pixels[0]},${tuning.slabs.joint_pixels[1]},footprint/width);
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
 return strokeJoint(paved.run.z,road.slabs.x,footprint,${tuning.slabs.joint_m})*road.slabs.y;
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
 let soft=max(footprint,${tuning.curb.edge_m})*0.5;
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
  if(abs(dot(ab,paved.run.xy))>${tuning.marks.crossing_cos}*len){continue;}
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
 let soft=max(footprint,${tuning.marks.edge_m})*0.5;
 let centre=(1.0-smoothstep(line-soft,line+soft,paved.lane.z))
  *(1.0-smoothstep(look.marks.y*0.5-soft,look.marks.y*0.5+soft,abs(dash)))
  *(1.0-step(-far-${tuning.marks.clear_m},near));
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
 *  joints, square to its outline's dominant bearing (the ground's `area_bearings`). Last a
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
   if(!plain){inside+=(wanderNoise(xy*${1 / tuning.surface.join_drift_m}+vec2f(7.9,52.3))-0.5)*2.0*carried;}
  }
  if(inside<=-feather){continue;}
  var core=look.core.xyz;
  if(!plain){
   let hue=smoothstep(${tuning.surface.patch_cut[0]},${tuning.surface.patch_cut[1]},wanderNoise(xy*look.shape.y+vec2f(61.7,17.3)))*look.worn.w;
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
     let border=1.0-smoothstep(0.0,footprint,out-min(1.0/look.stones.z,${tuning.slabs.area_border_m}));
     let w=${tuning.slabs.area_joint_m};
     let along=vec2f(cos(paved.bearing),sin(paved.bearing));
     // The grid fades out before a slab is a few pixels across, so far
     // paving reads as its colour rather than a shimmering mesh.
     let near=(1.0-smoothstep(${tuning.slabs.area_slab_pixels[0]},${tuning.slabs.area_slab_pixels[1]},footprint*look.stones.z))*(1.0-border);
     let joint=near*max(areaJoint(dot(xy,along),look.stones.z,footprint,w),areaJoint(dot(xy,vec2f(-along.y,along.x)),look.stones.z,footprint,w));
     let drawn=max(w,footprint);
     let edge=(1.0-smoothstep(drawn-footprint*0.5,drawn+footprint*0.5,out))*(w/drawn);
     core*=1.0-max(joint,edge*${tuning.slabs.area_edge_depth})*look.stones.w;
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
  let worn=1.0-look.marks.w*smoothstep(${tuning.marks.wear_cut[0]},${tuning.marks.wear_cut[1]},wanderNoise(xy*${1 / tuning.marks.wear_m}+vec2f(33.7,71.1)));
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  surface=mix(surface,vec4f(look.paint.xyz*(1.0+look.shape.z*grain),look.core.w),paint*look.paint.w*worn);
 }
 let stones=groundCurb(footprint,paved).z;
 if(stones>0.0){
  let look=terrainLayout.$.params.roads[u32(paved.run.w)];
  let grain=roadGrain(xy*look.shape.w,footprint*look.shape.w);
  let joint=strokeJoint(paved.run.z,look.stones.x,footprint,${tuning.slabs.joint_m})*look.stones.y;
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
    1 - std.smoothstep(0, tuning.distant.fade_m, inRegion),
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
      std.smoothstep(0, params.shoreTuning.ground.y, site.y),
    );
    const least = field * params.shoreEdge.z;
    const earth = std.mul(params.shoreEarth.xyz, 1 + params.shoreTuning.ground.x * noise);
    albedo = std.mix(albedo, atLeast(earth, least), shore.y);
    roughness = std.mix(roughness, params.shoreTuning.ground.z, shore.y);
    albedo = std.mix(albedo, atLeast(params.shore.xyz, least), shore.x);
    roughness = std.mix(roughness, params.shoreTuning.ground.w, shore.x);
  }

  // The roads, over all but water.
  const paving = groundRoads(xy, footprint, paved, d.vec4f(albedo, roughness));

  // The water bed, under the simulation's rivers (water wins over road).
  const bed = std.smoothstep(-aa, std.max(aa, params.shoreTuning.fade.z), water);
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
