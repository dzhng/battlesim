// WGSL owner: fog.

// @block fog_fogFoliage
fn fogFoliage(q: vec2f) -> vec2f {
  let P = fogParams;
  if (P.foliageNx == 0u || q.x < 0.0 || q.y < 0.0) { return vec2f(0.0); }
  let i = u32(q.x / P.foliageCellM);
  let j = u32(q.y / P.foliageCellM);
  if (i >= P.foliageNx || j >= P.foliageNy) { return vec2f(0.0); }
  let key=j*P.foliageNx+i;
  var lo=0u;
  var hi=arrayLength(&foliage);
  while(lo<hi){
    let mid=(lo+hi)/2u;
    let row=foliage[mid];
    let at=u32(row.y)*P.foliageNx+u32(row.x);
    if(at==key){return row.zw;}
    if(at<key){lo=mid+1u;}else{hi=mid;}
  }
  return vec2f(0.0);
}

// @block fog_fogSeenBy
fn fogSeenBy(ei: u32, p: vec3f, n: vec3f) -> bool {
  let e = eyes[ei];
  let P = fogParams;
  if (p.x < P.playable.x || p.y < P.playable.y ||
      p.x > P.playable.x + P.playable.z || p.y > P.playable.y + P.playable.w) {
    // The visual margin is outside the simulation. Keep it permanently unseen
    // and avoid all range, horizon and foliage work for it.
    return false;
  }
  let face = dot(n, n) > 0.0;
  if (face && dot(n, e.position - p) <= 0.0) { return false; }
  let dxy = p.xy - e.position.xy;
  let dist = length(dxy);
  if (dist < P.firstBinM) { return true; }
  let theta = atan2(dxy.y, dxy.x);
  let range = e.range * fogShape(e.front, e.side, e.rear, theta - e.forward);
  if (dist > range) { return false; }
  let AZ = P.azimuthBins;
  let R = FOG_RADIAL_BINS;
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
  let c0 = fogUnpack(maps[w0]);
  let c1 = fogUnpack(maps[w1]);
  var p0 = vec3f(-1e4, 0.0, 0.0);
  var p1 = vec3f(-1e4, 0.0, 0.0);
  if (k > 0u) {
    p0 = fogUnpack(maps[w0 - 1u]);
    p1 = fogUnpack(maps[w1 - 1u]);
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
}

// @block fog_fogSeenSurface
fn fogSeenSurface(ei: u32, world: vec3f, normal: vec3f, ground: bool) -> bool {
  let p = fogProbePoint(world, normal, ground);
  let n = select(normal, vec3f(0.0), ground);
  if (fogSeenBy(ei, p, n)) { return true; }
  let e = eyes[ei];
  let P = fogParams;
  if (ground || P.roofReachM <= 0.0 || normal.z < 0.7 || world.z <= e.position.z) {
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
}

// @block fog_fogShape
fn fogShape(front: f32, side: f32, rear: f32, off: f32) -> f32 {
  let c = cos(off);
  let end = select(rear, front, c >= 0.0);
  return side * (1.0 - c * c) + end * c * c;
}

// @block fog_FogBox
struct FogBox {
  center: vec2f,
  half: vec2f,
  yaw: f32,
  top: f32,
  base: f32,
}

// @block fog_FogEye
struct FogEye {
  position: vec3f,
  reach: f32,
  forward: f32,
  front: f32,
  side: f32,
  rear: f32,
  range: f32,
  slot: u32,
}

// @block fog_FogLayer
struct FogLayer {
  ground: u32,
  seen: u32,
  painted: u32,
  pad: u32,
}

// @block fog_FogParams
struct FogParams {
  invViewProj: mat4x4f,
  cameraEye: vec3f,
  translucentLiftM: f32,
  azimuthBins: u32,
  terrainAzimuthBins: u32,
  tilesX: u32,
  tileEyesMax: u32,
  eyeCount: u32,
  foliageNx: u32,
  foliageNy: u32,
  heightNx: u32,
  heightNy: u32,
  rebuildCount: u32,
  enabled: u32,
  probeCount: u32,
  firstBinM: f32,
  targetHeightM: f32,
  foliageCellM: f32,
  foliageFullBlock: f32,
  minSightGapM: f32,
  playable: vec4f,
  heightSpacing: f32,
  faceProbeM: f32,
  width: f32,
  height: f32,
  stepMinM: f32,
  stepMaxM: f32,
  stepFraction: f32,
  roofReachM: f32,
  wholeCount: u32,
  wholeNx: u32,
  wholeNy: u32,
  wholeCellM: f32,
  wholeOrigin: vec2f,
  wholeItemsBase: u32,
  wholeBoxesBase: u32,
  wholeFlagsBase: u32,
}

// @block fog_FogProbe
struct FogProbe {
  position: vec3f,
  ground: u32,
  normal: vec3f,
  pad: u32,
}

// @block fog_ColorFog
struct ColorFog {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

// @block fog_boxes
@group(0) @binding(0) var<storage, read> boxes: array<FogBox>;

// @block fog_eyes_buildLayout
@group(0) @binding(1) var<storage, read> eyes: array<FogEye>;

// @block fog_eyes_fogLayout
@group(GROUP) @binding(2) var<storage, read> eyes: array<FogEye>;

// @block fog_fogBinEdge
fn fogBinEdge(k: i32, first: f32, lnr: f32, bins: u32) -> f32 {
  if (k < 0) { return 0.0; }
  return first * exp(f32(k) / f32(bins - 1u) * lnr);
}

// @block fog_fogCoverage
fn fogCoverage(seen: f32, alpha: f32) -> vec4f {
  if (fogParams.enabled == 0u || layer.seen == 1u) {
    return vec4f(0.0, 0.0, 0.0, alpha);
  }
  return vec4f(1.0 - seen, seen, f32(layer.ground), alpha);
}

// @block fog_fogIsGround
fn fogIsGround() -> bool {
  return (layer.ground == 1u);
}

// @block fog_fogPack
fn fogPack(horizon: f32, jump: f32, foliage: f32) -> u32 {
  let h = pack2x16float(vec2f(clamp(horizon, -6e4, 6e4), 0.0)) & 0xffffu;
  return h | (u32(round(clamp(jump, 0.0, 1.0) * 255.0)) << 16u) | (min(u32(round(foliage / FOG_FOLIAGE_STEP)), FOG_FOLIAGE_QUANTA) << 24u);
}

// @block fog_fogProbePoint
fn fogProbePoint(world: vec3f, normal: vec3f, ground: bool) -> vec3f {
  if (ground) { return world + vec3f(0.0, 0.0, fogParams.targetHeightM); }
  return world + normal * fogParams.faceProbeM;
}

// @block fog_fogTerm
fn fogTerm(world: vec3f, normal: vec3f, pixel: vec2f, isGround: bool) -> f32 {
  let P = fogParams;
  if (P.enabled == 0u || layer.seen == 1u) { return 1.0; }
  if (fogWholeSeen(world)) { return 1.0; }
  let tile = u32(pixel.x) / FOG_TILE_PX + (u32(pixel.y) / FOG_TILE_PX) * P.tilesX;
  let count = min(counts[tile], P.tileEyesMax);
  for (var s = 0u; s < count; s++) {
    if (fogSeenSurface(lists[tile * P.tileEyesMax + s], world, normal, isGround)) {
      return 1.0;
    }
  }
  return 0.0;
}

// @block fog_fogUnpack
fn fogUnpack(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * FOG_FOLIAGE_STEP);
}

// @block fog_fogWholeSeen
fn fogWholeSeen(p: vec3f) -> bool {
  let P = fogParams;
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
    let o = P.wholeBoxesBase + b * 8u;
    let q = p.xy - vec2f(bitcast<f32>(fogWholeWord(o)), bitcast<f32>(fogWholeWord(o + 1u)));
    let cs = bitcast<f32>(fogWholeWord(o + 2u));
    let sn = bitcast<f32>(fogWholeWord(o + 3u));
    let local = vec2f(q.x * cs + q.y * sn, -q.x * sn + q.y * cs);
    let half = vec2f(bitcast<f32>(fogWholeWord(o + 4u)), bitcast<f32>(fogWholeWord(o + 5u)));
    let base = bitcast<f32>(fogWholeWord(o + 6u));
    let top = bitcast<f32>(fogWholeWord(o + 7u));
    if (all(abs(local) <= half + vec2f(m)) && p.z >= base - 2.0 && p.z <= top + 3.0) {
      return true;
    }
  }
  return false;
}

// @block fog_fogWholeWord
fn fogWholeWord(i: u32) -> u32 {
  return textureLoad(wholes, vec2u(i % FOG_WHOLE_TEXTURE_WIDTH, i / FOG_WHOLE_TEXTURE_WIDTH), 0).x;
}

// @block fog_layer_fixedGroup
@group(2) @binding(1) var<uniform> layer: FogLayer;

// @block fog_layer_grouped
@group(GROUP) @binding(1) var<uniform> layer: FogLayer;

// @block fog_modelSeen
fn modelSeen(world: vec3f, normal: vec3f, anchor: vec3f, pixel: vec2f) -> f32 {
  if (fogIsGround()) {
    return fogTerm(anchor, vec3f(), pixel, true);
  }
  return fogTerm(world, normal, pixel, false);
}

// @block fog_occluders
@group(0) @binding(4) var<storage, read> occluders: array<FogBox>;

// @block fog_params_fixedGroup
@group(2) @binding(0) var<uniform> fogParams: FogParams;

// @block fog_params_grouped
@group(GROUP) @binding(0) var<uniform> fogParams: FogParams;

// @block fog_points
@group(1) @binding(0) var<storage, read> points: array<FogProbe>;

// @block fog_whole
@group(0) @binding(1) var<uniform> whole: vec4f;

// @block fog_wholeFound
var<workgroup> wholeFound: atomic<u32>;

// @block fog_wholes
@group(GROUP) @binding(8) var wholes: texture_2d<u32>;

// @block fog_counts_fogLayout
@group(GROUP) @binding(5) var<storage, read> counts: array<u32>;

// @block fog_counts_cullLayout
@group(0) @binding(4) var<storage, read_write> counts: array<atomic<u32>>;

// @block fog_cullHi
var<workgroup> cullHi: array<atomic<i32>, 2>;

// @block fog_cullHit
var<workgroup> cullHit: atomic<u32>;

// @block fog_cullLo
var<workgroup> cullLo: array<atomic<i32>, 2>;

// @block fog_flags
@group(0) @binding(2) var<storage, read_write> flags: array<u32>;

// @block fog_foliage
@group(0) @binding(3) var<storage, read> foliage: array<vec4f>;

// @block fog_items
@group(0) @binding(3) var<storage, read> items: array<u32>;

// @block fog_lists_cullLayout
@group(0) @binding(3) var<storage, read_write> lists: array<u32>;

// @block fog_lists_fogLayout
@group(GROUP) @binding(4) var<storage, read> lists: array<u32>;

// @block fog_maps_buildLayout
@group(0) @binding(7) var<storage, read_write> maps: array<u32>;

// @block fog_maps_read
@group(GROUP) @binding(3) var<storage, read> maps: array<u32>;

// @block fog_out_shapeLayout
@group(0) @binding(1) var<storage, read_write> out: array<f32>;

// @block fog_out_probeLayout
@group(1) @binding(1) var<storage, read_write> out: array<u32>;

// @block fog_pastTrunk
fn pastTrunk(d: f32, radius: f32, k: f32) -> f32 {
  let m = abs(d);
  return sign(d) * (min(m, radius) + max(m - radius, 0.0) * k);
}

// @block fog_rebuild
@group(0) @binding(5) var<storage, read> rebuild: array<u32>;

// @block fog_rows_shapeLayout
@group(0) @binding(0) var<storage, read> rows: array<vec4f>;

// @block fog_depth_cullLayout
@group(0) @binding(2) var depth: texture_depth_multisampled_2d;
