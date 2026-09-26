// FogTerm: the one fog-of-war term every world material applies, in the HDR
// world before post. `fogTerm(world, normal, pixel, isGround)` says whether a
// fragment is seen; `unseenLook` is what unseen looks like (slice 15's).
//
// Sight lights (spike 02): each own eye has a polar horizon map, built by
// `FogVisibility` from the terrain and the occluders the side knows; a
// per-frame cull lists, per screen tile, the eyes whose reach touches it. A
// fragment tests only its tile's eyes and stops at the first that sees it:
// - ground probes at the simulation's target height, so drawn fog means what
//   the 8 m sweep means;
// - a face (prop, unit, structure, canopy) probes just outside itself along
//   its normal, and only eyes in front of it count.
// Per eye: the range comes from the published sight shape (`fogShape`, the
// mirror of `sim::sight::multiplier`), applied per fragment, so a turret's
// traverse never rebuilds a map; then the radial bin, the azimuth blend, the
// jump within the bin, and foliage interpolated along the bin.
//
// `fogLayout` is read by the world's fragments and by FogVisibility's probe;
// both call the same `fogSeenBy`.
import { tgpu, d, std } from "typegpu";

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
    forestCount: d.u32,
    heightNx: d.u32,
    heightNy: d.u32,
    rebuildCount: d.u32,
    /** 0: no fog (everything seen). */
    enabled: d.u32,
    /** 1: the world draws the seen/unseen debug mask. */
    mask: d.u32,
    probeCount: d.u32,
    firstBinM: d.f32,
    targetHeightM: d.f32,
    forestAttenuationM: d.f32,
    forestFullBlockM: d.f32,
    heightSpacing: d.f32,
    faceProbeM: d.f32,
    width: d.f32,
    height: d.f32,
    stepMinM: d.f32,
    stepMaxM: d.f32,
    stepFraction: d.f32,
    pad: d.f32,
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

/** What the drawn layer is: 1 for ground, 0 for faces standing on it. */
export const FogLayer = d.struct({ ground: d.u32 });

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
});

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

/** A map word: (horizon slope f16, jump position in the bin 0..1, foliage metres). */
export const fogUnpack = tgpu.fn(
  [d.u32],
  d.vec3f,
)(/* wgsl */ `(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * 0.5);
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
  if (foliage >= P.forestFullBlockM) { return false; }
  if (dist > range * exp(-foliage / P.forestAttenuationM)) { return false; }
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

/** Whether a fragment is seen: 0 unseen, 1 seen. */
export const fogTerm = tgpu
  .fn(
    [d.vec3f, d.vec3f, d.vec2f, d.bool],
    d.f32,
  )(/* wgsl */ `(world: vec3f, normal: vec3f, pixel: vec2f, isGround: bool) -> f32 {
  let P = fogLayout.$.params;
  if (P.enabled == 0u) { return 1.0; }
  let p = fogProbePoint(world, normal, isGround);
  let n = select(normal, vec3f(0.0), isGround);
  let tile = u32(pixel.x) / P.tilePx + (u32(pixel.y) / P.tilePx) * P.tilesX;
  let count = min(fogLayout.$.counts[tile], P.tileEyesMax);
  for (var s = 0u; s < count; s++) {
    if (fogSeenBy(fogLayout.$.lists[tile * P.tileEyesMax + s], p, n)) { return 1.0; }
  }
  return 0.0;
}`)
  .$uses({ fogLayout, fogSeenBy, fogProbePoint });

/** Whether the layer being drawn is ground, for `fogTerm`'s last argument. */
export const fogIsGround = tgpu.fn(
  [],
  d.bool,
)(() => {
  "use gpu";
  return fogLayout.$.layer.ground === 1;
});

/** Whether the world draws the seen/unseen debug mask instead of its colour. */
export const fogMask = tgpu.fn(
  [],
  d.bool,
)(() => {
  "use gpu";
  return fogLayout.$.params.mask === 1;
});

/** Unseen is darker and flatter, but its shading still reads. Slice 15 owns
 *  this look. */
export const unseenLook = tgpu.fn(
  [d.vec3f, d.f32],
  d.vec3f,
)((lit, seen) => {
  "use gpu";
  const grey = std.dot(lit, d.vec3f(0.3, 0.5, 0.2));
  const fogged = std.mul(std.mix(lit, d.vec3f(grey, grey, grey * 1.1), 0.45), 0.68);
  return std.mix(fogged, lit, seen);
});
