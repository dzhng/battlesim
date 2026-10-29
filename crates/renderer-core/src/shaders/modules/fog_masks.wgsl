// WGSL owner: fog_masks.

// @block fog_masks_FogLook
struct FogLook {
  tint: vec3f,
  dim: f32,
  cool: f32,
  saturation: f32,
  veil: f32,
  lineStrength: f32,
  lineSpacingPx: f32,
  lineWidthPx: f32,
  lineAngle: f32,
  lineFloor: f32,
  edgeSoftnessPx: f32,
  rimColor: vec3f,
  rimAlpha: f32,
  rimWidthPx: f32,
  reachPx: f32,
}

// @block fog_masks_FogMaskView
struct FogMaskView {
  mask: u32,
}

// @block fog_masks_GrassParams
struct GrassParams {
  planes: array<vec4f, 4>,
  eye: vec4f,
  window: vec4f,
  grid: vec4u,
  density: vec4f,
  tiers: vec4f,
  shading: vec4f,
  clear: vec4f,
  counts: vec4u,
  ground: vec4f,
  sway: vec4f,
  growth: array<vec4f, 16>,
  patches: array<vec4f, 16>,
  mixes: array<vec4f, 64>,
  patchScales: vec4f,
  grain: vec4f,
  kinds: array<vec4f, 16>,
  bases: array<vec4u, 16>,
}

// @block fog_masks_composeFog
fn composeFog(pixel: vec2f) -> vec4f {
  let p = vec2i(pixel);
  let lit = textureLoad(lit, p, 0);
  let m = textureLoad(mask, p, 0).xy;
  if (view.mask == 1u) {
    // Unseen (more than half the pixel's samples) black; untouched (none)
    // white; partly unseen grey.
    let v = select(select(0.35, 16.0, m.x <= 0.0), 0.0, m.x > 0.5);
    return vec4f(vec3f(v), 1.0);
  }
  if (view.mask == 2u) {
    // Mostly ground (more than half the pixel's samples) white, else black.
    let ground = textureLoad(mask, p, 0).z > 0.5;
    return vec4f(vec3f(select(0.0, 16.0, ground)), 1.0);
  }
  if (view.mask == 3u) {
    // The terrain's bytes where every sample of the pixel is ground; black
    // where anything else covers any of it.
    let ground = textureLoad(mask, p, 0).z >= 0.999;
    return vec4f(select(vec3f(0.0), lit.xyz, ground), 1.0);
  }
  if (m.x <= 0.0) { return lit; }
  let s = look;
  var unseen = m.x;
  if (s.edgeSoftnessPx > 0.0) {
    let toSeen = textureLoad(distance_1, p, 0).y * 16.0;
    unseen *= smoothstep(0.0, 1.0, (toSeen - 0.5) / s.edgeSoftnessPx);
  }
  return vec4f(mix(lit.xyz, fogLook(lit.xyz, pixel, s), unseen), lit.w);
}

// @block fog_masks_fogFoliage
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

// @block fog_masks_fogSeenBy_alternate
fn fogSeenBy(ei: u32, p: vec3f, n: vec3f) -> bool {
  let e = eyes[ei];
  let P = params_1;
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

// @block fog_masks_fogSeenSurface_alternate
fn fogSeenSurface(ei: u32, world: vec3f, normal: vec3f, ground: bool) -> bool {
  let p = fogProbePoint(world, normal, ground);
  let n = select(normal, vec3f(0.0), ground);
  if (fogSeenBy(ei, p, n)) { return true; }
  let e = eyes[ei];
  let P = params_1;
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

// @block fog_masks_fogShape
fn fogShape(front: f32, side: f32, rear: f32, off: f32) -> f32 {
  let c = cos(off);
  let end = select(rear, front, c >= 0.0);
  return side * (1.0 - c * c) + end * c * c;
}

// @block fog_masks_grassVertexOf
fn grassVertexOf(vid: u32, iid: u32) -> GrassVertex {
  let P = &params;
  let tier = tier;
  let c = clumps[tier.y + iid];
  let row = (*P).kinds[c.kind];
  var out: GrassVertex;
  if (vid / tier.z >= u32(row.y)) {
    // A blade this kind does not have: outside the depth range, clipped.
    out.clip = vec4f(0.0, 0.0, 2.0, 1.0);
    return out;
  }
  let base = select((*P).bases[c.kind].y, (*P).bases[c.kind].x, tier.x == 0u);
  let s = shapes[base + vid];
  let scale = c.height / max(row.x, 1e-4);
  let sn = c.yawSin;
  let cs = c.yawCos;
  let sp = s.spine.xyz * scale;
  let sd = s.side.xyz * scale * c.width;
  let spine = vec3f(sp.x * cs - sp.y * sn, sp.x * sn + sp.y * cs, sp.z);
  let side = vec3f(sd.x * cs - sd.y * sn, sd.x * sn + sd.y * cs, sd.z);
  // The one wind: a steady lean, gust fronts rolling downwind, a flutter.
  let t = cam.time;
  let wind = cameraWind();
  let dir = wind.toward.xy;
  let along = dot(c.root.xy, dir);
  let front = sin(6.2831853 * (along - t * wind.gustMps) * wind.gustPerM);
  let gust = smoothstep(0.2, 1.0, front);
  let flutter = (*P).sway.z * sin((*P).sway.w * t + s.side.w * 6.2831853 + along * 0.35);
  let bend = s.spine.w * s.spine.w;
  let packed = unpack4x8unorm(c.colour);
  // Laid over by tracks or trampling: the clump sinks and its blades lean
  // out its own way, still in the wind but less. A stiff kind answers the
  // wind less (its shape's own response).
  let flat = 1.0 - packed.w;
  let answer = s.tint.w * (1.0 - flat);
  let lean = ((*P).sway.x + (*P).sway.y * gust + flutter) * answer;
  let push = dir * lean + vec2f(-dir.y, dir.x) * flutter * 0.5 * answer;
  let laid = vec2f(cs, sn) * flat * 1.1;
  var world = c.root + vec3f(spine.xy, spine.z * (1.0 - flat * 0.6)) + side + vec3f((push + laid) * c.height * bend, 0.0);
  world.z -= 0.5 * dot(push, push) * c.height * bend;
  let n = s.normal.xyz;
  let colour = packed.xyz;
  out.clip = cam.viewProj * vec4f(world, 1.0);
  // Blades cross each other: where two meet at (nearly) one depth the winner
  // is left to the hardware and changes frame to frame. A per-blade nudge of
  // up to a two-thousandth of the depth (5 cm at 100 m, a few millimetres
  // near the eye) separates every such pair the same way each frame.
  let tie = ((bitcast<u32>(c.root.x) * 2654435761u) ^ (bitcast<u32>(c.root.y) * 2246822519u)) + (vid / tier.z) * 374761393u;
  out.clip.z *= 1.0 + 1e-3 * (f32(tie >> 8u) / 16777216.0 - 0.5);
  out.world = world;
  out.root = c.root;
  out.normal = vec3f(n.x * cs - n.y * sn, n.x * sn + n.y * cs, n.z);
  // A clump a few pixels tall shows its tops, not its roots: drawn with them,
  // its dark roots and gaps read as speckle, so its tint and its lighting
  // give way to the ground's.
  let footprint = (*P).eye.w * distance(cam.eye, c.root);
  let plain = (*P).shading.x * smoothstep((*P).tiers.z, (*P).tiers.w, footprint);
  out.albedo = colour * colour * mix(s.tint.xyz, vec3f(1.0), plain);
  out.ground = mix(1.0 - (*P).shading.y, 1.0, plain);
  return out;
}

// @block fog_masks_view
@group(0) @binding(1) var<uniform> view: FogMaskView;
