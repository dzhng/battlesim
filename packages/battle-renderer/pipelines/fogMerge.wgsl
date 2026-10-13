struct FogParams {
  invViewProj: mat4x4f,
  cameraEye: vec3f,
  translucentLiftM: f32,
  azimuthBins: u32,
  terrainAzimuthBins: u32,
  radialBins: u32,
  tilePx: u32,
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

@group(0) @binding(0) var<uniform> params: FogParams;

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

@group(0) @binding(1) var<storage, read> eyes: array<FogEye>;

@group(0) @binding(5) var<storage, read> rebuild: array<u32>;

struct FogBox {
  center: vec2f,
  half: vec2f,
  yaw: f32,
  top: f32,
  base: f32,
}

@group(0) @binding(4) var<storage, read> occluders: array<FogBox>;

fn fogUnpackBuild(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * 0.005);
}

@group(0) @binding(6) var<storage, read_write> terrain: array<u32>;

fn fogBinEdge(k: i32, first: f32, lnr: f32, bins: u32) -> f32 {
  if (k < 0) { return 0.0; }
  return first * exp(f32(k) / f32(bins - 1u) * lnr);
}

@group(0) @binding(7) var<storage, read_write> maps: array<u32>;

fn fogPack(horizon: f32, jump: f32, foliage: f32) -> u32 {
  let h = pack2x16float(vec2f(clamp(horizon, -6e4, 6e4), 0.0)) & 0xffffu;
  return h | (u32(round(clamp(jump, 0.0, 1.0) * 255.0)) << 16u) | (min(u32(round(foliage / 0.005)), 255u) << 24u);
}

@compute @workgroup_size(64) fn item(@builtin(global_invocation_id) gid: vec3u) {
  let P = params;
  let AZ = P.azimuthBins;
  let AZT = P.terrainAzimuthBins;
  let R = P.radialBins;
  let r = gid.x / AZ;
  let ai = gid.x % AZ;
  if (r >= P.rebuildCount) { return; }
  let e = eyes[rebuild[r * 3u]];
  let theta = (f32(ai) + 0.5) / f32(AZ) * 6.28318531 - 3.14159265;
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.reach / P.firstBinM);
  // A ray squeezing past boxes on both sides with less than the least sight
  // gap between them is blocked where it passes the second (as the
  // simulation's `squeezed`; here the closest box on each side counts).
  let gap = P.minSightGapM;
  var left = vec3f(1e9, 0.0, 0.0);
  var right = vec3f(1e9, 0.0, 0.0);
  var pm: array<f32, 64>;
  var pt: array<f32, 64>;
  for (var k = 0u; k < R; k++) { pm[k] = -1e4; pt[k] = 0.0; }
  let sectors = rebuild[r * 3u + 2u];
  let sector = min(u32((f32(ai) + 0.5) / f32(AZ) * f32(sectors)), sectors - 1u);
  let row = rebuild[r * 3u + 1u] + sector * 2u;
  let start = rebuild[row];
  let count = rebuild[row + 1u];
  for (var i = 0u; i < count; i++) {
    let b = occluders[rebuild[start + i]];
    let o0 = e.position.xy - b.center;
    if (length(o0) - length(b.half) > e.reach) { continue; }
    let c = cos(b.yaw);
    let s = sin(b.yaw);
    let o = vec2f(o0.x * c + o0.y * s, -o0.x * s + o0.y * c);
    let dl = vec2f(dir.x * c + dir.y * s, -dir.x * s + dir.y * c);
    var tn = -1e9;
    var tf = 1e9;
    var miss = false;
    if (abs(dl.x) < 1e-7) {
      if (abs(o.x) > b.half.x) { miss = true; }
    } else {
      let t1 = (-b.half.x - o.x) / dl.x;
      let t2 = (b.half.x - o.x) / dl.x;
      tn = max(tn, min(t1, t2));
      tf = min(tf, max(t1, t2));
    }
    if (abs(dl.y) < 1e-7) {
      if (abs(o.y) > b.half.y) { miss = true; }
    } else {
      let t1 = (-b.half.y - o.y) / dl.y;
      let t2 = (b.half.y - o.y) / dl.y;
      tn = max(tn, min(t1, t2));
      tf = min(tf, max(t1, t2));
    }
    let crossed = !(miss || tn > tf || tf <= 0.0);
    if (!crossed && gap > 0.0 && length(max(abs(o) - b.half, vec2f(0.0))) >= gap * 0.5) {
      // Room beside the ray: the nearest corner ahead, (offset, distance, top).
      var near = vec3f(1e9, 0.0, b.top);
      var side = 0.0;
      for (var q = 0u; q < 4u; q++) {
        let v = vec2f(select(-1.0, 1.0, (q & 1u) == 0u), select(-1.0, 1.0, q < 2u)) * b.half - o;
        let t = dot(dl, v);
        if (t <= 0.0 || t >= e.reach) { continue; }
        let off = dl.x * v.y - dl.y * v.x;
        side = sign(off);
        if (abs(off) < near.x) { near = vec3f(abs(off), t, b.top); }
      }
      if (side > 0.0 && near.x < left.x) { left = near; }
      if (side < 0.0 && near.x < right.x) { right = near; }
    }
    // An eye inside a box (a garrison slot) sees out of it.
    if (!crossed || tn < 0.0 || tn > e.reach) { continue; }
    let rel = b.top - e.position.z;
    // Above the eye the steepest point is the entry; below it, the exit.
    // A quarter metre floors the distance of a box touching the eye.
    var te = tn;
    var sl = rel / max(tn, 0.25);
    if (rel < 0.0) {
      te = min(tf, e.reach);
      sl = rel / max(te, 0.25);
    }
    var k0 = 0u;
    if (te > P.firstBinM) { k0 = u32(ceil(log(te / P.firstBinM) / lnr * f32(R - 1u))); }
    if (k0 >= R) { continue; }
    if (sl > pm[k0]) { pm[k0] = sl; pt[k0] = te; }
  }
  if (left.x + right.x < gap) {
    let te = max(left.y, right.y);
    let sl = (min(left.z, right.z) - e.position.z) / max(te, 0.25);
    var k0 = 0u;
    if (te > P.firstBinM) { k0 = u32(ceil(log(te / P.firstBinM) / lnr * f32(R - 1u))); }
    if (k0 < R && sl > pm[k0]) { pm[k0] = sl; pt[k0] = te; }
  }
  let a = (theta + 3.14159265) / 6.28318531 * f32(AZT) - 0.5;
  let af = floor(a);
  let fa = a - af;
  let j0 = u32(i32(af) + i32(AZT)) % AZT;
  let j1 = (j0 + 1u) % AZT;
  let tb = e.slot * AZT * R;
  let base = (e.slot * AZ + ai) * R;
  var hk = -1e4;
  for (var k = 0u; k < R; k++) {
    let c0 = fogUnpackBuild(terrain[tb + j0 * R + k]);
    let c1 = fogUnpackBuild(terrain[tb + j1 * R + k]);
    var f = select(c0.y, c1.y, fa > 0.5);
    var v = max(hk, mix(c0.x, c1.x, fa));
    if (pm[k] > v) {
      let lo = fogBinEdge(i32(k) - 1, P.firstBinM, lnr, R);
      let hi = fogBinEdge(i32(k), P.firstBinM, lnr, R);
      f = clamp((pt[k] - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
      v = pm[k];
    }
    hk = v;
    maps[base + k] = fogPack(v, f, mix(c0.z, c1.z, fa));
  }
}