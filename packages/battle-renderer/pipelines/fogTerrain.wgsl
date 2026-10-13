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

fn fogBinEdge(k: i32, first: f32, lnr: f32, bins: u32) -> f32 {
  if (k < 0) { return 0.0; }
  return first * exp(f32(k) / f32(bins - 1u) * lnr);
}

@group(0) @binding(6) var<storage, read_write> terrain: array<u32>;

fn fogPack(horizon: f32, jump: f32, foliage: f32) -> u32 {
  let h = pack2x16float(vec2f(clamp(horizon, -6e4, 6e4), 0.0)) & 0xffffu;
  return h | (u32(round(clamp(jump, 0.0, 1.0) * 255.0)) << 16u) | (min(u32(round(foliage / 0.005)), 255u) << 24u);
}

fn triangleRuleHeight(h00: f32, h10: f32, h01: f32, h11: f32, u: f32, v: f32) -> f32 {
  if (u >= v) { return h00 + u * (h10 - h00) + v * (h11 - h10); }
  return h00 + v * (h01 - h00) + u * (h11 - h01);
}

@group(0) @binding(2) var<storage, read> heights: array<u32>;

fn readWord(at: u32) -> u32 {
  return heights[at];
}

fn fogSample(i: u32, j: u32) -> f32 {
    let size = readWord(0u);
    let cols = readWord(1u);
    let count = readWord(2u);
    if (count == 0u) { return 0.0; }
    let page = readWord(3u + (j / size) * cols + i / size);
    if (page == 0u) { return 0.0; }
    let at = 3u + count + (page - 1u) * size * size + (j % size) * size + i % size;
    return bitcast<f32>(readWord(at));
  }

fn fogHeight(q: vec2f) -> f32 {
  let P = params;
  let nx = P.heightNx;
  let ny = P.heightNy;
  let fx = clamp(q.x / P.heightSpacing, 0.0, f32(nx - 1u));
  let fy = clamp(q.y / P.heightSpacing, 0.0, f32(ny - 1u));
  let i = min(u32(fx), nx - 2u);
  let j = min(u32(fy), ny - 2u);
  let u = fx - f32(i);
  let v = fy - f32(j);
  return triangleRuleHeight(
    fogSample(i, j), fogSample(i + 1u, j),
    fogSample(i, j + 1u), fogSample(i + 1u, j + 1u), u, v);
}

@group(0) @binding(3) var<storage, read> foliage: array<vec4f>;

fn fogFoliage(q: vec2f) -> vec2f {
  let P = params;
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

@compute @workgroup_size(64) fn terrainMarch(@builtin(global_invocation_id) gid: vec3u) {
  let P = params;
  let AZT = P.terrainAzimuthBins;
  let R = P.radialBins;
  let r = gid.x / AZT;
  let ai = gid.x % AZT;
  if (r >= P.rebuildCount) { return; }
  let e = eyes[rebuild[r * 3u]];
  let theta = (f32(ai) + 0.5) / f32(AZT) * 6.28318531 - 3.14159265;
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.reach / P.firstBinM);
  let base = (e.slot * AZT + ai) * R;
  let playableLo = P.playable.xy;
  let playableHi = playableLo + P.playable.zw;
  var hor = -1e4;
  var hk = -1e4;
  var fol = 0.0;
  var k = 0u;
  var t = 0.0;
  var jump = 0.0;
  var jt = 0.0;
  loop {
    let st = clamp(t * P.stepFraction, P.stepMinM, P.stepMaxM);
    t = t + st;
    let q = e.position.xy + dir * t;
    let outside = q.x < playableLo.x || q.y < playableLo.y ||
      q.x > playableHi.x || q.y > playableHi.y;
    if (outside) { t = 1e9; }
    loop {
      if (k >= R) { break; }
      let hi = fogBinEdge(i32(k), P.firstBinM, lnr, R);
      if (hi > t) { break; }
      let lo = fogBinEdge(i32(k) - 1, P.firstBinM, lnr, R);
      let v = max(hk, hor);
      if (v - hk > jump) { jump = v - hk; jt = hi; }
      let f = clamp((jt - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
      terrain[base + k] = fogPack(v, f, fol);
      hk = v;
      jump = 0.0;
      jt = hi;
      k = k + 1u;
    }
    if (k >= R || outside) { break; }
    let gz = fogHeight(q);
    let can = fogFoliage(q);
    let ts = (gz + P.targetHeightM - e.position.z) / t;
    if (can.x > 0.0 && e.position.z + max(hor, ts) * t < gz + can.x) { fol = fol + st * can.y; }
    let nh = max(hor, (gz - e.position.z) / t);
    if (nh - hor > jump) { jump = nh - hor; jt = t; }
    hor = nh;
  }
}