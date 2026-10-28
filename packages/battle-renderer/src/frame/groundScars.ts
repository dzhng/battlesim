// The side's learned scars on the ground, as the terrain and the grass read
// them: the scar cache's directory and pages (`scarTexture.ts` builds
// them), the linear and cubic filters over its cells, and the hard-edged
// marks they cut. Every lookup here samples the bindings `terrainLayout`
// owns.
import { tgpu, d } from "typegpu";
import game from "@fixtures/game.json";
import { scarFilterQuanta, scarFilterPosition } from "./scarFilter";
import { valueNoise, wgslFloat } from "./terrainMaterial";
import { terrainLayout } from "./terrainLayout";
import { SCAR_DENSE, SCAR_KEY_MASK, SCAR_UNIFORM } from "../records.gen";
const tuning = game.presentation.terrain;

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
  if(e.x==0u){break;}
  if((e.x&${SCAR_KEY_MASK}u)==key+1u){return e;}at=(at+1u)&P.pages.z;}
 return vec2u(${SCAR_UNIFORM}u,P.defaultWord.x);
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
/** The cell `c` (in the grid) from its tile's directory entry `e`. */
const scarCell = tgpu
  .fn(
    [d.vec2u, d.vec2u],
    d.vec4f,
  )(/* wgsl */ `(c:vec2u,e:vec2u)->vec4f {
 if(e.x==0u){return vec4f(0.0);}if((e.x&${SCAR_UNIFORM}u)!=0u){return scarWord(e.y);}
 let local=(c.y%16u)*16u+c.x%16u;
 if((e.x&${SCAR_DENSE}u)!=0u){return scarPoolWord(e.y+local);}
 let offset=e.y>>8u;var lo=0u;var hi=e.y&255u;
 while(lo<hi){let mid=(lo+hi)/2u;let bytes=vec4u(round(scarPoolWord(offset+mid*2u)*255.0));let end=bytes.x+bytes.y*256u;if(end<=local){lo=mid+1u;}else{hi=mid;}}
 return scarPoolWord(offset+lo*2u+1u);
}`)
  .$uses({ scarWord, scarPoolWord });
/** How far from a point, in cells along either axis, `groundScars`' taps
 *  land: the rim's or the slopes' reach, or the cubic's two, and one more
 *  for the filter's Q8 position. Each tap reads that cell and the next. */
const scarTapReach = Math.max(tuning.scars.rim_reach_cells, tuning.scars.slope_tap_cells, 2) + 1;
// The cells read span at most two tiles a side (`scarTiles`).
if (Math.ceil(2 * scarTapReach) + 2 > 17)
  throw new Error("presentation.terrain.scars: the taps reach past two scar tiles");
const SCAR_TAP_REACH = wgslFloat(scarTapReach);
/** The directory entries of the two-by-two tiles from `lo`, each looked up
 *  once for every cell a point's taps read (a tile repeats where the cells
 *  span one). Probing the directory is a loop, so a point's taps share
 *  these lookups rather than probe for every corner. */
const ScarTiles = d
  .struct({ lo: d.vec2u, e00: d.vec2u, e10: d.vec2u, e01: d.vec2u, e11: d.vec2u })
  .$name("ScarTiles");
/** The tiles holding cells `lo` to `hi` (in the grid), which span at most
 *  two tiles a side. */
const scarTiles = tgpu
  .fn(
    [d.vec2u, d.vec2u],
    ScarTiles,
  )(/* wgsl */ `(lo:vec2u,hi:vec2u)->ScarTiles {
 let a=lo/16u;let span=hi/16u-a;var e=array<vec2u,4>();
 // One probe site in a loop, as for the corners in \`scarLinear\`; a tile
 // that repeats where the cells span one is copied, not probed again.
 for(var k=0u;k<4u;k++){
  let o=min(vec2u(k&1u,k>>1u),span);let j=o.x+o.y*2u;
  if(j<k){e[k]=e[j];}else{e[k]=scarEntry(a+o);}
 }
 return ScarTiles(a,e[0],e[1],e[2],e[3]);
}`)
  .$uses({ ScarTiles, scarEntry });
/** The directory entry of `tile`, one of `t`'s: the reach check above keeps
 *  every tap's cells in them, so no tap probes the directory itself. */
const scarTileEntry = tgpu
  .fn(
    [ScarTiles, d.vec2u],
    d.vec2u,
  )(/* wgsl */ `(t:ScarTiles,tile:vec2u)->vec2u {
 let o=tile-t.lo;
 return select(select(t.e00,t.e10,o.x==1u),select(t.e01,t.e11,o.x==1u),o.y==1u);
}`)
  .$uses({ ScarTiles });
/** Whether every cell of `t`'s tiles is unmarked: outside the cache, or one
 *  word throughout and that word zero. */
const scarTilesBlank = tgpu
  .fn(
    [ScarTiles],
    d.bool,
  )(/* wgsl */ `(t:ScarTiles)->bool {
 let x=vec4u(t.e00.x,t.e10.x,t.e01.x,t.e11.x);let y=vec4u(t.e00.y,t.e10.y,t.e01.y,t.e11.y);
 let single=(x&vec4u(${SCAR_UNIFORM}u))!=vec4u(0u);
 return all((x==vec4u(0u))|(single&(y==vec4u(0u))));
}`)
  .$uses({ ScarTiles });
/** The original four-neighbor linear rule across every representation join,
 *  its corners' tiles among `tiles`. The corners share one lookup in a
 *  loop: a shader compiler inlines every call, and the search is a loop it
 *  cannot fold away. */
const scarLinear = tgpu
  .fn(
    [d.vec2f, ScarTiles],
    d.vec4f,
  )(/* wgsl */ `(uv:vec2f,tiles:ScarTiles)->vec4f {
 let P=terrainLayout.$.scarParams;let size=vec2f(P.pages.xy);let p=scarFilterPosition(uv,size);
 let cell=vec2i(floor(p));let f=fract(p);var q=array<vec4u,4>();
 for(var k=0u;k<4u;k++){
  let c=vec2u(clamp(cell+vec2i(i32(k&1u),i32(k>>1u)),vec2i(0),vec2i(P.pages.xy)-1));let e=scarTileEntry(tiles,c/16u);
  // The cell's own tile, one word throughout, with all four corners in it.
  if(k==0u&&(e.x==0u||(e.x&${SCAR_UNIFORM}u)!=0u)&&all(c%16u<vec2u(15u))){return scarWord(e.y);}
  q[k]=vec4u(round(scarCell(c,e)*255.0));
 }
 let w=scarFilterQuanta(q[0],q[1],q[2],q[3],f);
 return vec4f(scarNormalized(w.x),scarNormalized(w.y),scarNormalized(w.z),scarNormalized(w.w));
}`)
  .$uses({
    terrainLayout,
    ScarTiles,
    scarTileEntry,
    scarWord,
    scarCell,
    scarNormalized,
    scarFilterQuanta,
    scarFilterPosition,
  });
/** The original four-neighbor linear rule at `uv` alone. */
export const sampleGroundLinear = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(uv:vec2f)->vec4f {
 let P=terrainLayout.$.scarParams;let last=vec2i(P.pages.xy)-1;
 let cell=vec2i(floor(scarFilterPosition(uv,vec2f(P.pages.xy))));
 return scarLinear(uv,scarTiles(vec2u(clamp(cell,vec2i(0),last)),vec2u(clamp(cell+1,vec2i(0),last))));
}`)
  .$uses({ terrainLayout, scarTiles, scarLinear, scarFilterPosition });

/** A cubic B-spline over the 1 m cells at `uv` as four bilinear taps: `taps`
 *  their two columns' and two rows' coordinates (x0, y0, x1, y1), `weights`
 *  the columns' and rows' (x0, y0, x1, y1). */
const GroundCubic = d.struct({ taps: d.vec4f, weights: d.vec4f }).$name("GroundCubic");
const groundCubic = tgpu
  .fn(
    [d.vec2f],
    GroundCubic,
  )(/* wgsl */ `(uv: vec2f) -> GroundCubic {
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
  return GroundCubic(vec4f((i - 0.5 + w1 / g0) / size, (i + 1.5 + w3 / g1) / size), vec4f(g0, g1));
}`)
  .$uses({ terrainLayout, GroundCubic });
/** Tap `k` (0 to 3: x0y0, x1y0, x0y1, x1y1) of the spline `cubic`. */
const groundCubicTap = tgpu
  .fn(
    [GroundCubic, d.u32],
    d.vec2f,
  )(/* wgsl */ `(cubic: GroundCubic, k: u32) -> vec2f {
  return vec2f(select(cubic.taps.x, cubic.taps.z, (k & 1u) == 1u), select(cubic.taps.y, cubic.taps.w, k >= 2u));
}`)
  .$uses({ GroundCubic });
/** The spline `cubic` from its four taps, in `groundCubicTap`'s order. */
const groundCubicBlend = tgpu
  .fn(
    [GroundCubic, d.vec4f, d.vec4f, d.vec4f, d.vec4f],
    d.vec4f,
  )(/* wgsl */ `(cubic: GroundCubic, a: vec4f, b: vec4f, c: vec4f, e: vec4f) -> vec4f {
  let g0 = cubic.weights.xy;
  let g1 = cubic.weights.zw;
  // Keep the original hardware cubic outer contraction explicit after inlining.
  return fma(vec4f(g0.y), g0.x * a + g1.x * b, g1.y * (g0.x * c + g1.x * e));
}`)
  .$uses({ GroundCubic });

/** The four channels at `uv`, reconstructed by a cubic B-spline over the
 *  1 m cells (four bilinear taps): a crater's footprint comes back round
 *  and smooth, so its thresholded edges are circles, not diamonds. */
export const sampleGroundCubic = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(uv: vec2f) -> vec4f {
  let cubic = groundCubic(uv);
  var t = array<vec4f, 4>();
  for (var k = 0u; k < 4u; k++) { t[k] = sampleGroundLinear(groundCubicTap(cubic, k)); }
  return groundCubicBlend(cubic, t[0], t[1], t[2], t[3]);
}`)
  .$uses({ groundCubic, groundCubicTap, groundCubicBlend, sampleGroundLinear });

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
  // Every cell the taps below read is in these tiles: where none is marked,
  // every tap is zero, and the cheap reject would return the same.
  let centre = uv * vec2f(P.pages.xy) - 0.5;
  let last = vec2i(P.pages.xy) - 1;
  let tiles = scarTiles(
    vec2u(clamp(vec2i(floor(centre - ${SCAR_TAP_REACH})), vec2i(0), last)),
    vec2u(clamp(vec2i(floor(centre + ${SCAR_TAP_REACH})) + 1, vec2i(0), last)));
  if (scarTilesBlank(tiles)) { return out; }
  // Twelve bilinear taps through one call, as a shader compiler inlines each
  // call: the rim's four diagonals (a, b, c, e), the cubic's four, then
  // either side of \`uv\` along x and y for the slopes (xp, xm, yp, ym).
  let r = ${tuning.scars.rim_reach_cells};
  let h = ${tuning.scars.slope_tap_cells};
  let cubic = groundCubic(uv);
  var taps = array<vec4f, 12>();
  for (var t = 0u; t < 12u; t++) {
    if (t == 8u) {
      // Cheap reject: nothing marked within reach.
      let near = max(max(taps[0], taps[1]), max(taps[2], taps[3]));
      let s = groundCubicBlend(cubic, taps[4], taps[5], taps[6], taps[7]);
      if (max(max(s.x, s.y), max(s.z, s.w)) + max(max(near.x, near.y), max(near.z, near.w)) <= 0.0) {
        return out;
      }
    }
    var at: vec2f;
    if (t < 4u) {
      at = uv + texel * vec2f(select(-r, r, (t & 1u) == 1u), select(-r, r, t >= 2u));
    } else if (t < 8u) {
      at = groundCubicTap(cubic, t - 4u);
    } else {
      let side = texel * select(vec2f(0.0, h), vec2f(h, 0.0), t < 10u);
      at = select(uv - side, uv + side, (t & 1u) == 0u);
    }
    taps[t] = scarLinear(at, tiles);
  }
  let a = taps[0];
  let b = taps[1];
  let c = taps[2];
  let e = taps[3];
  let near = max(max(a, b), max(c, e));
  let s = groundCubicBlend(cubic, taps[4], taps[5], taps[6], taps[7]);
  let xp = taps[8];
  let xm = taps[9];
  let yp = taps[10];
  let ym = taps[11];
  let detail = 1.0 - smoothstep(${tuning.scars.detail_fade[0]}, ${wgslFloat(tuning.scars.detail_fade[1])}, footprint / cell);
  let k = P.full;
  let depth = min(s.x * k.x, ${tuning.scars.depth_cap});
  let wide = min((a.x + b.x + c.x + e.x) * 0.25 * k.x, ${tuning.scars.depth_cap});
  // Slopes per metre of the crater field, from the taps either side.
  let bowlSlope = vec2f(xp.x - xm.x, yp.x - ym.x) * k.x / (2.0 * h * cell);
  let wideSlope = vec2f((b.x + e.x) - (a.x + c.x), (c.x + e.x) - (a.x + b.x)) * 0.5 * k.x / (2.0 * r * cell);
  // The height: down into the bowl, up onto the rim just outside it.
  let onRim = select(0.0, 1.0, wide > depth);
  let slope = (P.relief.y * onRim * (wideSlope - bowlSlope) - bowlSlope) * P.relief.x * detail;
  // Iso-lines, each crossing within a pixel.
  let grain = valueNoise(xy * ${1 / tuning.scars.lip_wander_m} + vec2f(5.7, 2.1)) - 0.5;
  let pixelDepth = length(bowlSlope) * footprint;
  let bowl = crossing(depth, ${tuning.scars.lip[0]} + ${tuning.scars.lip[1]} * grain, pixelDepth);
  let skirt = crossing(depth, ${tuning.scars.ring[0]} + ${tuning.scars.ring[1]} * grain, pixelDepth) * (1.0 - bowl);
  // The deepest crater within the taps: a lone pock's skirt is scorched
  // earth, a real crater's is its thrown soil.
  let peak = max(max(max(s.x, xp.x), max(xm.x, yp.x)), max(ym.x, near.x)) * k.x;
  let soil = smoothstep(${tuning.scars.pock[0]}, ${tuning.scars.pock[1]}, peak);
  let ring = skirt * soil;
  let scorched = crossing(depth, ${tuning.scars.pock_edge[0]} + ${tuning.scars.pock_edge[1]} * grain, pixelDepth) * (1.0 - bowl);
  // Darkest at its heart, browner toward its edge, as a burn is.
  let heart = ${tuning.scars.pock_edge_dark} + ${1 - tuning.scars.pock_edge_dark} * smoothstep(${tuning.scars.pock_edge[0]}, ${tuning.scars.ring[0]}, depth);
  let pock = scorched * (1.0 - soil) * ${tuning.scars.pock_soot} * heart;
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
    scarTiles,
    scarTilesBlank,
    scarLinear,
    groundCubic,
    groundCubicTap,
    groundCubicBlend,
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
  // One call in the loop, as a shader compiler inlines each call.
  let view = normalize(eye - world);
  let run = -view.xy / max(view.z, ${tuning.scars.grazing});
  let deep = terrainLayout.$.scarParams.relief.x;
  var scar: ScarSample;
  var at = world.xy;
  for (var i = 0; i < 3; i++) {
    scar = groundScars(at, footprint);
    if (i == 0 && scar.relief.w <= 0.0) { break; }
    at = world.xy + run * min(scar.relief.w, ${tuning.scars.depth_cap}) * deep * 0.5;
  }
  return scar;
}`)
  .$uses({ terrainLayout, groundScars, ScarSample });

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
  let grain = valueNoise(xy * ${tuning.scar_surface.grain_scale});
  let fleck = valueNoise(xy * ${tuning.scar_surface.fleck_scale} + vec2f(2.2, 9.1));
  albedo = mix(albedo, P.trampled.xyz * mix(${tuning.scar_surface.trampled[0]}, ${tuning.scar_surface.trampled[1]}, grain), w.w * P.trampled.w);
  albedo = mix(albedo, P.tracks.xyz * mix(${tuning.scar_surface.tracks[0]}, ${tuning.scar_surface.tracks[1]}, grain), w.z * P.tracks.w);
  // Soot is uneven: ash grey and char black in flecks and drifts.
  let ash = mix(${tuning.scar_surface.ash[0]}, ${tuning.scar_surface.ash[1]}, fleck * ${tuning.scar_surface.ash_weights[0]} + valueNoise(xy * ${tuning.scar_surface.ash_drift_scale} + vec2f(4.4, 0.3)) * ${tuning.scar_surface.ash_weights[1]});
  albedo = mix(albedo, P.scorch.xyz * ash, w.y * P.scorch.w);
  let ring = scar.relief.z * P.ejecta.w;
  albedo = mix(albedo, P.ejecta.xyz * mix(${tuning.scar_surface.soil[0]}, ${tuning.scar_surface.soil[1]}, fleck), ring);
  let bowl = w.x * P.crater.w;
  // The deeper, the darker: the floor sees less sky than the lip.
  let cavity = mix(1.0, ${tuning.scar_surface.cavity}, saturate(scar.relief.w));
  albedo = mix(albedo, P.crater.xyz * mix(${tuning.scar_surface.soil[0]}, ${tuning.scar_surface.soil[1]}, fleck) * cavity, bowl);
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
  let capped = slope * min(1.0, ${tuning.shading.max_slope} / max(steep, 1e-6));
  return normalize(n - vec3f(capped, 0.0) * n.z);
}`);

/** Every terrain fragment belongs to exactly one cache region. */
export const scarRegionContains = tgpu.fn(
  [d.vec2f],
  d.bool,
)((world) => {
  "use gpu";
  const r = terrainLayout.$.scarParams.clip;
  return r.z <= r.x || (world.x >= r.x && world.y >= r.y && world.x < r.z && world.y < r.w);
});
