// WGSL owner: passes_fog.

// @block passes_fog_mergeFogHorizons
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn mergeFogHorizons(@builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
  let AZ = P.azimuthBins;
  let AZT = P.terrainAzimuthBins;
  let R = FOG_RADIAL_BINS;
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
  var pm: array<f32, FOG_RADIAL_BINS>;
  var pt: array<f32, FOG_RADIAL_BINS>;
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
    let c0 = fogUnpack(terrain[tb + j0 * R + k]);
    let c1 = fogUnpack(terrain[tb + j1 * R + k]);
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

// @block passes_fog_composeFogMask
@fragment fn composeFogMask(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return composeFog(pos.xy);
}

// @block passes_fog_cullFogEyes
@compute @workgroup_size(FOG_TILE_PX, FOG_TILE_PX) fn cullFogEyes(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32, @builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
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
  for (var e = li; e < P.eyeCount; e += FOG_TILE_PX * FOG_TILE_PX) {
    let eye = eyes[e];
    let near = clamp(eye.position.xy, lo, hi);
    if (distance(near, eye.position.xy) <= eye.reach) {
      let s = atomicAdd(&counts[tile], 1u);
      if (s < P.tileEyesMax) { lists[tile * P.tileEyesMax + s] = e; }
    }
  }
}

// @block passes_fog_probeFogPoints
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn probeFogPoints(@builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
  if (gid.x >= P.probeCount) { return; }
  let q = points[gid.x];
  var seen = 0u;
  for (var e = 0u; e < P.eyeCount; e++) {
    if (fogSeenSurface(e, q.position, q.normal, q.ground == 1u)) { seen = 1u; break; }
  }
  out[gid.x] = seen;
}

// @block passes_fog_probeFogShapes
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn probeFogShapes(@builtin(global_invocation_id) gid: vec3u) {
  if (gid.x >= arrayLength(&rows)) { return; }
  let r = rows[gid.x];
  out[gid.x] = fogShape(r.x, r.y, r.z, r.w);
}

// @block passes_fog_terrainMarch
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn terrainMarch(@builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
  let AZT = P.terrainAzimuthBins;
  let R = FOG_RADIAL_BINS;
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

// @block passes_fog_probeFogWholes
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn probeFogWholes(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32) {
  let W = whole;
  let item = wg.x + wg.y * 65535u;
  if (item >= u32(W.y)) { return; }
  let index = items[item * 3u];
  let eyesStart = items[item * 3u + 1u];
  let eyesCount = items[item * 3u + 2u];
  if (li == 0u) { atomicStore(&wholeFound, 0u); }
  workgroupBarrier();
  let b = boxes[index];
  let step = W.x;
  let u = vec2f(cos(b.yaw), sin(b.yaw));
  let v = vec2f(-u.y, u.x);
  let cx = u32(ceil(2.0 * b.half.x / step)) + 1u;
  let cy = u32(ceil(2.0 * b.half.y / step)) + 1u;
  let lo = b.base + 0.3;
  let hi = max(b.top - 0.3, lo);
  let cz = u32(ceil((hi - lo) / step)) + 1u;
  let wallsX = cy * cz;
  let wallsY = cx * cz;
  let total = 2u * wallsX + 2u * wallsY + cx * cy;
  let inset = min(vec2f(0.2), b.half * 0.5);
  let reach = length(b.half);
  for (var s = li; s < total; s += FOG_BUILD_WORKGROUP) {
    if (atomicLoad(&wholeFound) != 0u) { break; }
    // Along a side: t ranges [0, 1] over n samples, kept off the corners by the inset.
    var local = vec2f(0.0);
    var z = b.top;
    var n = vec3f(0.0, 0.0, 1.0);
    var r = s;
    if (r < 2u * wallsX) {
      let side = select(-1.0, 1.0, r < wallsX);
      r = r % wallsX;
      let t = f32(r % cy) / f32(max(cy - 1u, 1u));
      local = vec2f(side * b.half.x, mix(-b.half.y + inset.y, b.half.y - inset.y, t));
      z = mix(lo, hi, f32(r / cy) / f32(max(cz - 1u, 1u)));
      n = vec3f(u * side, 0.0);
    } else if (r < 2u * wallsX + 2u * wallsY) {
      r = r - 2u * wallsX;
      let side = select(-1.0, 1.0, r < wallsY);
      r = r % wallsY;
      let t = f32(r % cx) / f32(max(cx - 1u, 1u));
      local = vec2f(mix(-b.half.x + inset.x, b.half.x - inset.x, t), side * b.half.y);
      z = mix(lo, hi, f32(r / cx) / f32(max(cz - 1u, 1u)));
      n = vec3f(v * side, 0.0);
    } else {
      r = r - 2u * wallsX - 2u * wallsY;
      let tx = f32(r % cx) / f32(max(cx - 1u, 1u));
      let ty = f32(r / cx) / f32(max(cy - 1u, 1u));
      local = vec2f(mix(-b.half.x + inset.x, b.half.x - inset.x, tx), mix(-b.half.y + inset.y, b.half.y - inset.y, ty));
    }
    let p = vec3f(b.center + u * local.x + v * local.y, z);
    for (var ei = 0u; ei < eyesCount; ei++) {
      let e = items[eyesStart + ei];
      let eye = eyes[e];
      if (distance(eye.position.xy, b.center) - reach > eye.reach) { continue; }
      if (fogSeenSurface(e, p, n, false)) { atomicStore(&wholeFound, 1u); break; }
    }
  }
  workgroupBarrier();
  if (li == 0u) { flags[index] = atomicLoad(&wholeFound); }
}

// @block passes_fog_fogRimFragment
@fragment fn fogRimFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return rimAt(pos.xy);
}

// @block passes_fog_fogDistanceColumnsFragment
@fragment fn fogDistanceColumnsFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceColumns(pos.xy);
}

// @block passes_fog_fogDistanceRowsFragment
@fragment fn fogDistanceRowsFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return distanceRows(pos.xy);
}
