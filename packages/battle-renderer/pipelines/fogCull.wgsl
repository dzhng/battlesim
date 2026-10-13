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

var<workgroup> cullLo: array<atomic<i32>, 2>;

var<workgroup> cullHi: array<atomic<i32>, 2>;

var<workgroup> cullHit: atomic<u32>;

@group(0) @binding(2) var depth: texture_depth_multisampled_2d;

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

@group(0) @binding(4) var<storage, read_write> counts: array<atomic<u32>>;

@group(0) @binding(3) var<storage, read_write> lists: array<u32>;

@compute @workgroup_size(16, 16) fn item(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32, @builtin(global_invocation_id) gid: vec3u) {
  let P = params;
  if (li == 0u) {
    atomicStore(&cullLo[0], 2147483647);
    atomicStore(&cullLo[1], 2147483647);
    atomicStore(&cullHi[0], -2147483647);
    atomicStore(&cullHi[1], -2147483647);
    atomicStore(&cullHit, 0u);
  }
  workgroupBarrier();
  if (f32(gid.x) < P.width && f32(gid.y) < P.height) {
    // Reverse-Z: 0 is the far clear, the sky.
    let z = textureLoad(depth, vec2i(gid.xy), 0);
    if (z > 0.0) {
      let ndc = vec4f((f32(gid.x) + 0.5) / P.width * 2.0 - 1.0, 1.0 - (f32(gid.y) + 0.5) / P.height * 2.0, z, 1.0);
      let h = P.invViewProj * ndc;
      let g = h.xyz / h.w;
      let up = P.cameraEye - g;
      let q = g + up * min(1.0, P.translucentLiftM / max(up.z, 1e-3));
      // Quarter metres, as integers the workgroup can reduce atomically.
      atomicMin(&cullLo[0], i32(floor(min(g.x, q.x) * 4.0)));
      atomicMin(&cullLo[1], i32(floor(min(g.y, q.y) * 4.0)));
      atomicMax(&cullHi[0], i32(ceil(max(g.x, q.x) * 4.0)));
      atomicMax(&cullHi[1], i32(ceil(max(g.y, q.y) * 4.0)));
      atomicStore(&cullHit, 1u);
    }
  }
  workgroupBarrier();
  if (atomicLoad(&cullHit) == 0u) { return; }
  let lo = vec2f(f32(atomicLoad(&cullLo[0])), f32(atomicLoad(&cullLo[1]))) * 0.25;
  let hi = vec2f(f32(atomicLoad(&cullHi[0])), f32(atomicLoad(&cullHi[1]))) * 0.25;
  let tile = wg.x + wg.y * P.tilesX;
  for (var e = li; e < P.eyeCount; e += 256u) {
    let eye = eyes[e];
    let near = clamp(eye.position.xy, lo, hi);
    if (distance(near, eye.position.xy) <= eye.reach) {
      let s = atomicAdd(&counts[tile], 1u);
      if (s < P.tileEyesMax) { lists[tile * P.tileEyesMax + s] = e; }
    }
  }
}