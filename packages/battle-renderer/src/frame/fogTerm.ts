// FogTerm: the one fog-of-war term every world material applies.
// `fogTerm(world, normal, pixel, isGround)` says whether a fragment is seen,
// and `fogCoverage` writes that into the frame's fog mask (the world pass's
// second target) beside the lit colour. The fog mask pass (fogMaskPass.ts)
// then gives unseen pixels the frame's FogStyle, softens the edge and draws
// the rim; a material never shades fog itself.
//
// Sight lights (spike 02): each own eye has a polar horizon map, built by
// `FogVisibility` from the terrain and the occluders the side knows; a
// per-frame cull lists, per screen tile, the eyes whose reach touches it. A
// fragment tests only its tile's eyes and stops at the first that sees it:
// - ground probes at the simulation's target height, so drawn fog means what
//   the 8 m sweep means;
// - a face (prop, unit, structure, canopy) probes just outside itself along
//   its normal, and only eyes in front of it count.
// - a surface facing up and standing above an eye (a roof, a canopy top) that
//   the face test leaves unseen counts as seen when the air at its own height,
//   pulled up to `roof_reach_m` toward that eye, is seen: a roof reads as its
//   building's near side does, not as the hidden plane it truly is;
// - units' own faces are always seen: a unit is drawn by identification, so
//   its back is never fog;
// - an occluding structure (a building, a ruin, a wall: every box the side
//   knows hides what is behind it) takes fog whole: when any of its sampled
//   surface is seen (`FogVisibility`'s whole pass), everything drawn inside
//   its box (walls, roof, interior, and the ground and grass of its
//   footprint, such as a courtyard) is seen. A structure none of whose
//   samples is seen falls back to the per-fragment test, so a seen pixel is
//   never fogged.
// Per eye: the range comes from the published sight shape (`fogShape`, the
// mirror of `sim::sight::multiplier`), applied per fragment, so a turret's
// traverse never rebuilds a map; then the radial bin, the azimuth blend, the
// jump within the bin, and foliage interpolated along the bin.
//
// `fogLayout` is read by the world's fragments and by FogVisibility's probe;
// both call the same `fogSeenBy`.
import { FOLIAGE_STEP, WHOLE_BOX_WORDS, WHOLE_TEXTURE_WIDTH } from "./fogInputs";
import { tgpu, d } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";

/** Per-frame fog parameters: the map resolution, the rule numbers, the
 *  screen tiles and the camera the cull unprojects depth with. */
export const FogParams = d
  .struct({
    invViewProj: d.mat4x4f,
    cameraEye: d.vec3f,
    /** Height of the tallest translucent surface (canopy) above the ground. */
    translucentLiftM: d.f32,
    azimuthBins: d.u32,
    terrainAzimuthBins: d.u32,
    radialBins: d.u32,
    tilePx: d.u32,
    tilesX: d.u32,
    tilesY: d.u32,
    tileEyesMax: d.u32,
    eyeCount: d.u32,
    occluderCount: d.u32,
    /** The foliage grid's cells across and down (0: no foliage). */
    foliageNx: d.u32,
    foliageNy: d.u32,
    heightNx: d.u32,
    heightNy: d.u32,
    rebuildCount: d.u32,
    /** 0: no fog (everything seen). */
    enabled: d.u32,
    probeCount: d.u32,
    firstBinM: d.f32,
    targetHeightM: d.f32,
    foliageCellM: d.f32,
    /** Foliage depth that blocks a ground ray outright (`sensors.foliage_full_block`). */
    foliageFullBlock: d.f32,
    heightSpacing: d.f32,
    faceProbeM: d.f32,
    width: d.f32,
    height: d.f32,
    stepMinM: d.f32,
    stepMaxM: d.f32,
    stepFraction: d.f32,
    /** How far toward an eye a roof or canopy top looks for seen air. */
    roofReachM: d.f32,
    /** The structures that take fog whole (`fogWholeSeen`): how many, and
     *  their lookup grid in the `wholes` texture: cells across and down, the
     *  cell's size and the grid's origin, and where the item lists, boxes and
     *  seen flags start (words). 0 structures: none. */
    wholeCount: d.u32,
    wholeNx: d.u32,
    wholeNy: d.u32,
    wholeCellM: d.f32,
    wholeOrigin: d.vec2f,
    wholeItemsBase: d.u32,
    wholeBoxesBase: d.u32,
    wholeFlagsBase: d.u32,
  })
  .$name("FogParams");

/** One eye as its map was built: `position` is the map's origin and `reach`
 *  its radius; forward, shape and range are this publication's. */
export const FogEyeRecord = d
  .struct({
    position: d.vec3f,
    reach: d.f32,
    forward: d.f32,
    front: d.f32,
    side: d.f32,
    rear: d.f32,
    range: d.f32,
    /** Which map slot holds this eye's maps. */
    slot: d.u32,
    pad0: d.u32,
    pad1: d.u32,
  })
  .$name("FogEyeRecord");

/** What the drawn layer is: `ground` 1 for ground, 0 for faces standing on
 *  it; `seen` 1 for layers fog never covers (units); `painted` 1 for the
 *  layers the ground paint lies on (`groundPaint`): the terrain, its grass,
 *  the backdrop, the water and the props movers stand on (a bridge deck),
 *  never a body. */
export const FogLayer = d.struct({ ground: d.u32, seen: d.u32, painted: d.u32, pad: d.u32 });

/** How the ground paint looks on a painted layer (`PaintStyle`): its
 *  reflectance as a share of its colour, its emissive, how much of the
 *  ground's fog it takes, and the light it throws up the grass over it
 *  (strength, and the height it falls off over). */
/** The paint target holds colour over this range (rgba8 reaches 1), so a
 *  mark can glow brighter than its hue's full value (the selection). */
export const PAINT_RANGE = 2;

export const GroundPaintStyle = d
  .struct({
    albedo: d.f32,
    emissive: d.f32,
    fogKeep: d.f32,
    grassGlow: d.f32,
    grassFalloff: d.f32,
    saturation: d.f32,
    pad1: d.f32,
    pad2: d.f32,
  })
  .$name("GroundPaintStyle");

const words = (n: number) => d.arrayOf(d.u32, n);
export const fogLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["fragment", "compute"] },
  layer: { uniform: FogLayer, visibility: ["fragment", "compute"] },
  eyes: {
    storage: (n: number) => d.arrayOf(FogEyeRecord, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  maps: { storage: words, access: "readonly", visibility: ["fragment", "compute"] },
  lists: { storage: words, access: "readonly", visibility: ["fragment", "compute"] },
  counts: { storage: words, access: "readonly", visibility: ["fragment", "compute"] },
  paintStyle: { uniform: GroundPaintStyle, visibility: ["fragment"] },
  /** The frame's ground paint (`paintedMarks.ts`): premultiplied display
   *  colour per screen pixel, the marks drawn at the ground in view. */
  paint: { texture: d.texture2d(d.f32), visibility: ["fragment"] },
  /** The structures that take fog whole, as words (`WHOLE_TEXTURE_WIDTH` a
   *  row): the grid's cells, their item lists, the boxes and each box's seen
   *  flag (`FogVisibility`). A texture, not a storage buffer: the terrain's
   *  fragment already binds the default eight storage buffers. */
  wholes: { texture: d.texture2d(d.u32), visibility: ["fragment", "compute"] },
});

/** A structure's whole fog reaches this far above its box's top (a pitched
 *  roof's ridge over the simulation's box) and below its base (the ground of
 *  its footprint where the terrain dips). */
const WHOLE_ABOVE_M = "3.0";
const WHOLE_BELOW_M = "2.0";

/** One word of the `wholes` texture. */
const fogWholeWord = tgpu
  .fn(
    [d.u32],
    d.u32,
  )(/* wgsl */ `(i: u32) -> u32 {
  return textureLoad(fogLayout.$.wholes, vec2u(i % ${WHOLE_TEXTURE_WIDTH}u, i / ${WHOLE_TEXTURE_WIDTH}u), 0).x;
}`)
  .$uses({ fogLayout });

/**
 * Whether `p` lies inside a structure that takes fog whole and is seen: its
 * box, grown by the face probe (so its outer faces count) and reaching a
 * little above its top and below its base (a roof's ridge, the ground of its
 * footprint).
 */
export const fogWholeSeen = tgpu
  .fn(
    [d.vec3f],
    d.bool,
  )(/* wgsl */ `(p: vec3f) -> bool {
  let P = fogLayout.$.params;
  if (P.wholeCount == 0u) { return false; }
  let c = (p.xy - P.wholeOrigin) / P.wholeCellM;
  if (c.x < 0.0 || c.y < 0.0 || c.x >= f32(P.wholeNx) || c.y >= f32(P.wholeNy)) { return false; }
  let cell = fogWholeWord(u32(c.y) * P.wholeNx + u32(c.x));
  let start = cell >> 8u;
  let n = cell & 0xffu;
  let m = P.faceProbeM * 2.0;
  for (var k = 0u; k < n; k++) {
    let b = fogWholeWord(P.wholeItemsBase + start + k);
    if (fogWholeWord(P.wholeFlagsBase + b) == 0u) { continue; }
    let o = P.wholeBoxesBase + b * ${WHOLE_BOX_WORDS}u;
    let q = p.xy - vec2f(bitcast<f32>(fogWholeWord(o)), bitcast<f32>(fogWholeWord(o + 1u)));
    let cs = bitcast<f32>(fogWholeWord(o + 2u));
    let sn = bitcast<f32>(fogWholeWord(o + 3u));
    let local = vec2f(q.x * cs + q.y * sn, -q.x * sn + q.y * cs);
    let half = vec2f(bitcast<f32>(fogWholeWord(o + 4u)), bitcast<f32>(fogWholeWord(o + 5u)));
    let base = bitcast<f32>(fogWholeWord(o + 6u));
    let top = bitcast<f32>(fogWholeWord(o + 7u));
    if (all(abs(local) <= half + vec2f(m)) && p.z >= base - ${WHOLE_BELOW_M} && p.z <= top + ${WHOLE_ABOVE_M}) {
      return true;
    }
  }
  return false;
}`)
  .$uses({ fogLayout, fogWholeWord });

/** The sight shape's multiplier `off` radians from forward:
 *  `side·sin² + (front or rear)·cos²`. Mirrors `sim::sight::multiplier`,
 *  pinned against its oracle vectors. */
export const fogShape = tgpu.fn(
  [d.f32, d.f32, d.f32, d.f32],
  d.f32,
)(/* wgsl */ `(front: f32, side: f32, rear: f32, off: f32) -> f32 {
  let c = cos(off);
  let end = select(rear, front, c >= 0.0);
  return side * (1.0 - c * c) + end * c * c;
}`);

/** A map word: (horizon slope f16, jump position in the bin 0..1, foliage depth). */
export const fogUnpack = tgpu.fn(
  [d.u32],
  d.vec3f,
)(/* wgsl */ `(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * ${FOLIAGE_STEP});
}`);

/** The far edge of radial bin `k` (0 for k < 0): log-spaced from `first` to
 *  `first·e^lnr`, the eye's reach, over `bins` bins. */
export const fogBinEdge = tgpu.fn(
  [d.i32, d.f32, d.f32, d.u32],
  d.f32,
)(/* wgsl */ `(k: i32, first: f32, lnr: f32, bins: u32) -> f32 {
  if (k < 0) { return 0.0; }
  return first * exp(f32(k) / f32(bins - 1u) * lnr);
}`);

/**
 * Whether eye `ei` sees probe point `p`. `n` is the face's normal, or zero
 * for ground (no facing test). The spike's fixes are built in:
 * - the horizon switches from the previous bin's to this bin's where it
 *   jumped inside the bin, plus a tenth of a metre (landmine 1);
 * - only eyes in front of a face count (landmine 2);
 * - where both neighbouring rays meet the same facade, the jump distance is
 *   interpolated across them before choosing once, with a tolerance that
 *   grows with the grazing angle; at a silhouette corner each ray chooses and
 *   the choices blend (landmine 4);
 * - foliage is interpolated linearly inside the bin (landmine 5).
 */
export const fogSeenBy = tgpu
  .fn(
    [d.u32, d.vec3f, d.vec3f],
    d.bool,
  )(/* wgsl */ `(ei: u32, p: vec3f, n: vec3f) -> bool {
  let e = fogLayout.$.eyes[ei];
  let P = fogLayout.$.params;
  let face = dot(n, n) > 0.0;
  if (face && dot(n, e.position - p) <= 0.0) { return false; }
  let dxy = p.xy - e.position.xy;
  let dist = length(dxy);
  if (dist < P.firstBinM) { return true; }
  let theta = atan2(dxy.y, dxy.x);
  let range = e.range * fogShape(e.front, e.side, e.rear, theta - e.forward);
  if (dist > range) { return false; }
  let AZ = P.azimuthBins;
  let R = P.radialBins;
  let lnr = log(e.reach / P.firstBinM);
  let k = min(u32(ceil(log(dist / P.firstBinM) / lnr * f32(R - 1u))), R - 1u);
  let lo = fogBinEdge(i32(k) - 1, P.firstBinM, lnr, R);
  let hi = fogBinEdge(i32(k), P.firstBinM, lnr, R);
  let a = (theta + 3.14159265) / 6.28318531 * f32(AZ) - 0.5;
  let af = floor(a);
  let fa = a - af;
  let i0 = u32(i32(af) + i32(AZ)) % AZ;
  let i1 = (i0 + 1u) % AZ;
  let w0 = (e.slot * AZ + i0) * R + k;
  let w1 = (e.slot * AZ + i1) * R + k;
  let c0 = fogUnpack(fogLayout.$.maps[w0]);
  let c1 = fogUnpack(fogLayout.$.maps[w1]);
  var p0 = vec3f(-1e4, 0.0, 0.0);
  var p1 = vec3f(-1e4, 0.0, 0.0);
  if (k > 0u) {
    p0 = fogUnpack(fogLayout.$.maps[w0 - 1u]);
    p1 = fogUnpack(fogLayout.$.maps[w1 - 1u]);
  }
  let t0 = mix(lo, hi, c0.y);
  let t1 = mix(lo, hi, c1.y);
  let eps = 0.1 + (hi - lo) / 255.0;
  var tanInc = 0.0;
  if (face && length(n.xy) > 1e-3) {
    let c = clamp(abs(dot(dxy / dist, normalize(n.xy))), 0.017, 1.0);
    tanInc = sqrt(1.0 - c * c) / c;
  }
  let spread = 1.0 + 0.01 * dist + 1.5 * dist * (6.28318531 / f32(AZ)) * tanInc;
  var horizon = 0.0;
  if (abs(t0 - t1) < spread) {
    let tj = mix(t0, t1, fa);
    horizon = select(mix(p0.x, p1.x, fa), mix(c0.x, c1.x, fa), dist > tj + eps);
  } else {
    let h0 = select(p0.x, c0.x, dist > t0 + eps);
    let h1 = select(p1.x, c1.x, dist > t1 + eps);
    horizon = mix(h0, h1, fa);
  }
  let u = clamp((dist - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
  let foliage = mix(mix(p0.z, c0.z, u), mix(p1.z, c1.z, u), fa);
  if (foliage >= P.foliageFullBlock) { return false; }
  if (dist > range * exp(-foliage)) { return false; }
  return (p.z - e.position.z) / dist >= horizon;
}`)
  .$uses({ fogLayout, fogShape, fogUnpack, fogBinEdge });

/** Where a fragment probes: ground at the simulation's target height, a face
 *  just outside itself along its normal. */
export const fogProbePoint = tgpu
  .fn(
    [d.vec3f, d.vec3f, d.bool],
    d.vec3f,
  )(/* wgsl */ `(world: vec3f, normal: vec3f, ground: bool) -> vec3f {
  if (ground) { return world + vec3f(0.0, 0.0, fogLayout.$.params.targetHeightM); }
  return world + normal * fogLayout.$.params.faceProbeM;
}`)
  .$uses({ fogLayout });

/** Surfaces facing at least this far up count as roofs or canopy tops. */
const ROOF_NORMAL_Z = 0.7;

/**
 * Whether eye `ei` sees a drawn surface at `world` with `normal`: the probe
 * point and facing test above, then the roof rule. A roof or canopy top above
 * the eye that the face test leaves unseen is tested again as the air at its
 * own height pulled `roofReachM` toward the eye (no facing test), within the
 * eye's range at the roof itself. A reach of 0 turns the rule off.
 */
export const fogSeenSurface = tgpu
  .fn(
    [d.u32, d.vec3f, d.vec3f, d.bool],
    d.bool,
  )(/* wgsl */ `(ei: u32, world: vec3f, normal: vec3f, ground: bool) -> bool {
  let p = fogProbePoint(world, normal, ground);
  let n = select(normal, vec3f(0.0), ground);
  if (fogSeenBy(ei, p, n)) { return true; }
  let e = fogLayout.$.eyes[ei];
  let P = fogLayout.$.params;
  if (ground || P.roofReachM <= 0.0 || normal.z < ${ROOF_NORMAL_Z} || world.z <= e.position.z) {
    return false;
  }
  let dxy = world.xy - e.position.xy;
  let dist = length(dxy);
  if (dist <= P.firstBinM) { return true; }
  let range = e.range * fogShape(e.front, e.side, e.rear, atan2(dxy.y, dxy.x) - e.forward);
  if (dist > range) { return false; }
  let pull = min(P.roofReachM, dist - P.firstBinM);
  let q = vec3f(world.xy - dxy / dist * pull, world.z + P.faceProbeM);
  return fogSeenBy(ei, q, vec3f(0.0));
}`)
  .$uses({ fogLayout, fogSeenBy, fogProbePoint, fogShape });

/** Whether a fragment is seen: 0 unseen, 1 seen. Inside a seen structure
 *  that takes fog whole, seen; otherwise the tile's eyes decide. */
export const fogTerm = tgpu
  .fn(
    [d.vec3f, d.vec3f, d.vec2f, d.bool],
    d.f32,
  )(/* wgsl */ `(world: vec3f, normal: vec3f, pixel: vec2f, isGround: bool) -> f32 {
  let P = fogLayout.$.params;
  if (P.enabled == 0u || fogLayout.$.layer.seen == 1u) { return 1.0; }
  if (fogWholeSeen(world)) { return 1.0; }
  let tile = u32(pixel.x) / P.tilePx + (u32(pixel.y) / P.tilePx) * P.tilesX;
  let count = min(fogLayout.$.counts[tile], P.tileEyesMax);
  for (var s = 0u; s < count; s++) {
    if (fogSeenSurface(fogLayout.$.lists[tile * P.tileEyesMax + s], world, normal, isGround)) {
      return 1.0;
    }
  }
  return 0.0;
}`)
  .$uses({ fogLayout, fogSeenSurface, fogWholeSeen });

/** Whether the layer being drawn is ground, for `fogTerm`'s last argument. */
export const fogIsGround = tgpu.fn(
  [],
  d.bool,
)(() => {
  "use gpu";
  return fogLayout.$.layer.ground === 1;
});

/**
 * A fragment's fog mask sample, `(unseen, seen, ground, alpha)`: 1 in the
 * channel its `seen` names, and whether the layer is ground (the mask pass
 * draws the edge only across the ground); or `(0, 0, 0)` where fog
 * never applies (units, or no fog at all), which the mask pass leaves as lit
 * and never rims. `alpha` is the fragment's own, so a translucent surface
 * blends its mask as it blends its colour.
 */
export const fogCoverage = tgpu
  .fn(
    [d.f32, d.f32],
    d.vec4f,
  )(/* wgsl */ `(seen: f32, alpha: f32) -> vec4f {
  if (fogLayout.$.params.enabled == 0u || fogLayout.$.layer.seen == 1u) {
    return vec4f(0.0, 0.0, 0.0, alpha);
  }
  return vec4f(1.0 - seen, seen, f32(fogLayout.$.layer.ground), alpha);
}`)
  .$uses({ fogLayout });

/** The ground paint on a ground point (a terrain fragment's own position,
 *  the ground under a blade fragment, as paint sprayed from above lands):
 *  premultiplied display colour and coverage, read at the pixel the point
 *  projects to (the marks were drawn at the ground). Nothing on a layer that
 *  isn't painted. */
export const groundPaint = tgpu
  .fn(
    [d.vec3f],
    d.vec4f,
  )(/* wgsl */ `(ground: vec3f) -> vec4f {
  if (fogLayout.$.layer.painted == 0u) { return vec4f(0.0); }
  let c = typegpuCameraLayout.$.cam.viewProj * vec4f(ground, 1.0);
  if (c.w <= 0.0) { return vec4f(0.0); }
  let size = vec2i(textureDimensions(fogLayout.$.paint));
  let ndc = c.xy / c.w;
  let px = vec2i(floor(vec2f(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * vec2f(size)));
  if (any(px < vec2i(0)) || any(px >= size)) { return vec4f(0.0); }
  let p = textureLoad(fogLayout.$.paint, px, 0);
  return vec4f(p.xyz * ${PAINT_RANGE}, p.w);
}`)
  .$uses({ fogLayout, typegpuCameraLayout });

/** The ground paint as drawn at a screen pixel (a blade fragment's own):
 *  the stroke where it shows, whatever stands in front of the ground there.
 *  Nothing on a layer that isn't painted. */
export const groundPaintAtPixel = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pixel: vec2f) -> vec4f {
  if (fogLayout.$.layer.painted == 0u) { return vec4f(0.0); }
  let size = vec2i(textureDimensions(fogLayout.$.paint));
  let p = textureLoad(fogLayout.$.paint, clamp(vec2i(pixel), vec2i(0), size - 1), 0);
  return vec4f(p.xyz * ${PAINT_RANGE}, p.w);
}`)
  .$uses({ fogLayout });

/** The paint's colour, linear, pushed from its grey by the style's
 *  `saturation`: the tone mapper and the grade pull a bright hue toward
 *  white, and this gives it back, so the paint reads with the overlay's
 *  colour. */
const paintColour = tgpu
  .fn(
    [d.vec4f],
    d.vec3f,
  )(/* wgsl */ `(paint: vec4f) -> vec3f {
  let c = pow(max(paint.xyz / max(paint.w, 1e-4), vec3f(0.0)), vec3f(2.2));
  let grey = dot(c, vec3f(0.2126, 0.7152, 0.0722));
  return max(mix(vec3f(grey), c, fogLayout.$.paintStyle.saturation), vec3f(0.0));
}`)
  .$uses({ fogLayout });

/** A painted surface's albedo: the paint's colour (linear, at the style's
 *  reflectance) over `albedo` by its coverage. */
export const paintedAlbedo = tgpu
  .fn(
    [d.vec3f, d.vec4f],
    d.vec3f,
  )(/* wgsl */ `(albedo: vec3f, paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return albedo; }
  let colour = min(paintColour(paint), vec3f(1.0));
  return mix(albedo, colour * fogLayout.$.paintStyle.albedo, paint.w);
}`)
  .$uses({ fogLayout, paintColour });

/** The paint's own light: its colour times the style's emissive, by its
 *  coverage (the world's bloom carries it). */
export const paintGlow = tgpu
  .fn(
    [d.vec4f],
    d.vec3f,
  )(/* wgsl */ `(paint: vec4f) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  return paintColour(paint) * fogLayout.$.paintStyle.emissive * paint.w;
}`)
  .$uses({ fogLayout, paintColour });

/** How seen a painted fragment reads: the paint takes only `fogKeep` of the
 *  ground's fog, so a mark in unseen ground still reads through the fog. */
export const paintedSeen = tgpu
  .fn(
    [d.f32, d.vec4f],
    d.f32,
  )(/* wgsl */ `(seen: f32, paint: vec4f) -> f32 {
  return mix(seen, mix(1.0, seen, fogLayout.$.paintStyle.fogKeep), paint.w);
}`)
  .$uses({ fogLayout });

/** The paint's light on the grass over it: a blade is lit from below in the
 *  paint's colour, strongest at its root and falling off up it over
 *  `grassFalloff` metres (`height` over the ground). The blade's own surface
 *  is never the paint's: the grass reads as lit, not painted. */
export const grassPaintGlow = tgpu
  .fn(
    [d.vec4f, d.f32],
    d.vec3f,
  )(/* wgsl */ `(paint: vec4f, height: f32) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  let s = fogLayout.$.paintStyle;
  let colour = paintColour(paint);
  return colour * s.grassGlow * paint.w * exp(-max(height, 0.0) / max(s.grassFalloff, 1e-3));
}`)
  .$uses({ fogLayout, paintColour });
