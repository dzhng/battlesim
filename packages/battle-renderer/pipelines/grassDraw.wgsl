struct GrassVertex {
  clip: vec4f,
  world: vec3f,
  root: vec3f,
  normal: vec3f,
  albedo: vec3f,
  ground: f32,
}

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

@group(1) @binding(0) var<uniform> params: GrassParams;

@group(1) @binding(1) var<uniform> tier: vec4u;

struct GrassClump {
  root: vec3f,
  height: f32,
  colour: u32,
  kind: u32,
  yawSin: f32,
  yawCos: f32,
  width: f32,
}

@group(1) @binding(2) var<storage, read> clumps: array<GrassClump>;

struct GrassShapeVertex {
  spine: vec4f,
  side: vec4f,
  normal: vec4f,
  tint: vec4f,
}

@group(1) @binding(3) var<storage, read> shapes: array<GrassShapeVertex>;

struct Wind {
  toward: vec3f,
  driftMps: f32,
  gustMps: f32,
  gustPerM: f32,
}

struct Camera {
  viewProj: mat4x4f,
  invViewProj: mat4x4f,
  eye: vec3f,
  znear: f32,
  width: f32,
  height: f32,
  time: f32,
  wind: array<Wind, 1>,
  waves: vec4f,
}

@group(0) @binding(0) var<uniform> cam: Camera;

fn cameraWind() -> Wind { return cam.wind[0]; }

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

struct grassVertex_Output {
  @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}

@vertex fn grassVertex(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> grassVertex_Output {
  let g = grassVertexOf(vid, iid);
  return grassVertex_Output(g.clip, g.world, g.root, g.normal, g.albedo, g.ground);
}

struct FogLayer {
  ground: u32,
  seen: u32,
  painted: u32,
  pad: u32,
}

@group(2) @binding(1) var<uniform> layer: FogLayer;

fn fogIsGround() -> bool {
  return (layer.ground == 1u);
}

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

@group(2) @binding(0) var<uniform> params_1: FogParams;

@group(2) @binding(8) var wholes: texture_2d<u32>;

fn fogWholeWord(i: u32) -> u32 {
  return textureLoad(wholes, vec2u(i % 256u, i / 256u), 0).x;
}

fn fogWholeSeen(p: vec3f) -> bool {
  let P = params_1;
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

@group(2) @binding(5) var<storage, read> counts: array<u32>;

fn fogProbePoint(world: vec3f, normal: vec3f, ground: bool) -> vec3f {
  if (ground) { return world + vec3f(0.0, 0.0, params_1.targetHeightM); }
  return world + normal * params_1.faceProbeM;
}

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

@group(2) @binding(2) var<storage, read> eyes: array<FogEye>;

fn fogShape(front: f32, side: f32, rear: f32, off: f32) -> f32 {
  let c = cos(off);
  let end = select(rear, front, c >= 0.0);
  return side * (1.0 - c * c) + end * c * c;
}

fn fogBinEdge(k: i32, first: f32, lnr: f32, bins: u32) -> f32 {
  if (k < 0) { return 0.0; }
  return first * exp(f32(k) / f32(bins - 1u) * lnr);
}

fn fogUnpack(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * 0.005);
}

@group(2) @binding(3) var<storage, read> maps: array<u32>;

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

@group(2) @binding(4) var<storage, read> lists: array<u32>;

fn fogTerm(world: vec3f, normal: vec3f, pixel: vec2f, isGround: bool) -> f32 {
  let P = params_1;
  if (P.enabled == 0u || layer.seen == 1u) { return 1.0; }
  if (fogWholeSeen(world)) { return 1.0; }
  let tile = u32(pixel.x) / P.tilePx + (u32(pixel.y) / P.tilePx) * P.tilesX;
  let count = min(counts[tile], P.tileEyesMax);
  for (var s = 0u; s < count; s++) {
    if (fogSeenSurface(lists[tile * P.tileEyesMax + s], world, normal, isGround)) {
      return 1.0;
    }
  }
  return 0.0;
}

struct Aerial {
  extinction: vec3f,
  clearRadiusKm: f32,
  sunTint: vec3f,
  sunStrength: f32,
  mistColor: vec3f,
  mistColorStrength: f32,
  overviewFromM: f32,
  rangeNearM: f32,
  rangeFarM: f32,
  rangePower: f32,
  rangeStrength: f32,
  mistOpacity: f32,
  sunPower: f32,
  mistHeightM: vec2f,
  mistDistanceM: vec2f,
  horizonFade: vec2f,
}

struct Environment {
  worldToView: mat4x4f,
  observer: vec4f,
  sunDirection: vec4f,
  sunRadiance: vec4f,
  settings: vec4f,
  fill: vec4f,
  aerial: Aerial,
}

struct SunCascade {
  matrix: mat4x4f,
  bias: vec4f,
  interval: vec4f,
}

struct SunShadow {
  cascades: array<SunCascade, 4>,
  control: vec4f,
}

fn shadowPcf(depth: texture_depth_2d_array, compare: sampler_comparison, layer: i32, uv: vec2f, z: f32, pixel: vec2f, radius: f32) -> f32 {
  let phi=fract(52.9829189*fract(dot(pixel,vec2f(0.06711056,0.00583715))))*6.28318530718;
  let cphi=cos(phi);let sphi=sin(phi);
  let scale=radius/f32(textureDimensions(depth).x);
  var visibility=0.0;
  let b0=vec2f(0.31622776601683794,0);let o0=vec2f(b0.x*cphi-b0.y*sphi,b0.x*sphi+b0.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o0,layer,z);let b1=vec2f(-0.40387356772577226,0.36998127154283905);let o1=vec2f(b1.x*cphi-b1.y*sphi,b1.x*sphi+b1.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o1,layer,z);let b2=vec2f(0.06181932279751069,-0.7043992982171811);let o2=vec2f(b2.x*cphi-b2.y*sphi,b2.x*sphi+b2.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o2,layer,z);let b3=vec2f(0.5090564735709379,0.6639740256331726);let o3=vec2f(b3.x*cphi-b3.y*sphi,b3.x*sphi+b3.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o3,layer,z);let b4=vec2f(-0.934181236883857,-0.16524350714671632);let o4=vec2f(b4.x*cphi-b4.y*sphi,b4.x*sphi+b4.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o4,layer,z);
  return visibility*0.2;
}

fn shadowVisibility(depth: texture_depth_2d_array, compare: sampler_comparison, layer: i32, matrix: mat4x4f, settings: vec4f, world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  let clip=matrix*vec4f(world+normal*settings.y,1);
  let coord=clip.xyz/clip.w;
  let uv=vec2f(coord.x*0.5+0.5,0.5-coord.y*0.5);
  let z=coord.z-settings.x;
  let inside=all(uv>=vec2f(0))&&all(uv<=vec2f(1))&&z>=0&&z<=1;
  if (!inside) { return 1.0; }
  return shadowPcf(depth,compare,layer,uv,z,pixel,settings.z);
}

fn inheritedSunShadow(environment: Environment, sunShadow: SunShadow, sunDepth: texture_depth_2d_array, sunCompare: sampler_comparison, world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  let viewZ=(environment.worldToView*vec4f(world,1.0)).z;
  // Breaks are fractions of the receiver range [control.z, control.x]
  // (split near, capped far); the receiver normalises its depth the same way.
  // The range is sampled on a ray grid, so a sliver of map can sit just
  // nearer than it: the first cascade takes that too.
  let linearDepth=max((-viewZ-sunShadow.control.z)/(sunShadow.control.x-sunShadow.control.z),0.0);
  var shade=1.0;
  {
    let interval=sunShadow.cascades[0].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,0,sunShadow.cascades[0].matrix,sunShadow.cascades[0].bias,world,normal,pixel))*select(1.0,ratio,linearDepth>centre);
    }
  }
  {
    let interval=sunShadow.cascades[1].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,1,sunShadow.cascades[1].matrix,sunShadow.cascades[1].bias,world,normal,pixel))*ratio;
    }
  }
  {
    let interval=sunShadow.cascades[2].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,2,sunShadow.cascades[2].matrix,sunShadow.cascades[2].bias,world,normal,pixel))*ratio;
    }
  }
  {
    let interval=sunShadow.cascades[3].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,3,sunShadow.cascades[3].matrix,sunShadow.cascades[3].bias,world,normal,pixel))*ratio;
    }
  }
  return shade;
}

@group(3) @binding(0) var<uniform> data: Environment;

@group(3) @binding(6) var<uniform> sun: SunShadow;

@group(3) @binding(7) var sunDepth: texture_depth_2d_array;

@group(3) @binding(8) var sunCompare: sampler_comparison;

fn sampleSunShadow(world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  return inheritedSunShadow(data, sun, sunDepth, sunCompare, world, normal, pixel);
}

@group(2) @binding(7) var paint: texture_2d<f32>;

fn groundPaint(ground: vec3f) -> vec4f {
  if (layer.painted == 0u) { return vec4f(0.0); }
  let c = cam.viewProj * vec4f(ground, 1.0);
  if (c.w <= 0.0) { return vec4f(0.0); }
  let size = vec2i(textureDimensions(paint));
  let ndc = c.xy / c.w;
  let px = vec2i(floor(vec2f(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * vec2f(size)));
  if (any(px < vec2i(0)) || any(px >= size)) { return vec4f(0.0); }
  let p = textureLoad(paint, px, 0);
  return vec4f(p.xyz * 2, p.w);
}

fn groundPaintAtPixel(pixel: vec2f) -> vec4f {
  if (layer.painted == 0u) { return vec4f(0.0); }
  let size = vec2i(textureDimensions(paint));
  let p = textureLoad(paint, clamp(vec2i(pixel), vec2i(0), size - 1), 0);
  return vec4f(p.xyz * 2, p.w);
}

struct CastLight {
  position: vec4f,
  color: vec4f,
}

struct CastLights {
  header: vec4f,
  lights: array<CastLight, 24>,
}

@group(3) @binding(1) var<uniform> lights: CastLights;

fn castLight(base: vec3f, normal: vec3f, position: vec3f) -> vec3f {
  let count = u32(lights.header.x);
  let wrap = lights.header.y;
  let n = normalize(normal);
  var sum = vec3f();
  for (var i = 0u; (i < count); i++) {
    let toward = (lights.lights[i].position.xyz - position);
    let d2 = dot(toward, toward);
    let x2 = (d2 * lights.lights[i].position.w);
    if ((x2 < 1f)) {
      let falloff = (((1f - x2) * (1f - x2)) / (1f + (6f * x2)));
      let facing = (dot(n, toward) * inverseSqrt(max(d2, 1e-4f)));
      let wrapped = clamp(((facing + wrap) / (1f + wrap)), 0f, 1f);
      sum = (sum + (lights.lights[i].color.xyz * (falloff * wrapped)));
    }
  }
  let grey = dot(base, vec3f(0.2125999927520752, 0.7152000069618225, 0.0722000002861023));
  let albedo = max(mix(base, vec3f(grey), 0.35f), vec3f(0.10000000149011612));
  return (albedo * (sum * 0.3183098861837907f));
}

fn roughnessMip(r: f32) -> f32 {
  if(r>=0.8){return (1.0-r)/0.2-2.0;}
  if(r>=0.4){return (0.8-r)*3.0/0.4-1.0;}
  if(r>=0.305){return (0.4-r)/0.095+2.0;}
  if(r>=0.21){return (0.305-r)/0.095+3.0;}
  return -2.0*log2(1.16*r);
}

fn cubeFace(d: vec3f) -> f32 {
  let a=abs(d);
  if(a.x>a.z) {
    if(a.x>a.y){return select(3.0,0.0,d.x>0.0);}
    return select(4.0,1.0,d.y>0.0);
  }
  if(a.z>a.y){return select(5.0,2.0,d.z>0.0);}
  return select(4.0,1.0,d.y>0.0);
}

fn cubeUv(d: vec3f, face: f32) -> vec2f {
  var p=vec2f(0);
  if(face==0){p=vec2f(d.z,d.y)/abs(d.x);}
  else if(face==1){p=vec2f(-d.x,-d.z)/abs(d.y);}
  else if(face==2){p=vec2f(-d.x,d.y)/abs(d.z);}
  else if(face==3){p=vec2f(-d.z,d.y)/abs(d.x);}
  else if(face==4){p=vec2f(-d.x,d.z)/abs(d.y);}
  else{p=vec2f(d.x,d.y)/abs(d.z);}
  return 0.5*(p+vec2f(1));
}

fn cubeSample(env: texture_2d<f32>, linear: sampler, d: vec3f, mip: f32, maxMip: f32) -> vec3f {
  var face=cubeFace(d);
  let filterIndex=max(4.0-mip,0.0);
  let size=exp2(max(mip,4.0));
  var uv=cubeUv(d,face)*(size-2.0)+vec2f(1);
  if(face>2.0){uv.y+=size;face-=3.0;}
  uv.x+=face*size;
  uv.x+=filterIndex*48.0;
  uv.y+=4.0*(exp2(maxMip)-size);
  uv/=vec2f(textureDimensions(env));
  return textureSampleGrad(env,linear,uv,vec2f(0),vec2f(0)).rgb;
}

fn samplePmrem(env: texture_2d<f32>, linear: sampler, d: vec3f, r: f32, maxMip: f32) -> vec3f {
  let mip=clamp(roughnessMip(r),-2.0,maxMip);
  let f=fract(mip);
  let a=cubeSample(env,linear,d,floor(mip),maxMip);
  if(f==0){return a;}
  return mix(a,cubeSample(env,linear,d,floor(mip)+1.0,maxMip),f);
}

fn standardPbr(base: vec3f, emissive: vec3f, authoredRoughness: f32, geometryRoughness: f32, metal: f32, ao: f32, n: vec3f, v: vec3f, l: vec3f, sunRadiance: vec3f, shadow: f32, environmentIntensity: vec3f, pmrem: texture_2d<f32>, linear: sampler, maxMip: f32, dfg: texture_2d<f32>, dfgSampler: sampler) -> vec3f {
  let roughness=min(max(authoredRoughness,0.0525)+geometryRoughness,1.0);
  let nl=clamp(dot(n,l),0.0,1.0);
  let nv=clamp(dot(n,v),0.0,1.0);
  let h=normalize(l+v);
  let nh=clamp(dot(n,h),0.0,1.0);
  let vh=clamp(dot(v,h),0.0,1.0);
  let alpha=roughness*roughness;
  let a2=alpha*alpha;
  let fresnel=exp2((-5.55473*vh-6.98316)*vh);
  let f0=mix(vec3f(0.04),base,metal);
  let f=f0*(1.0-fresnel)+vec3f(fresnel);
  let gv=nl*sqrt(a2+(1.0-a2)*nv*nv);
  let gl=nv*sqrt(a2+(1.0-a2)*nl*nl);
  let visibility=0.5/max(gv+gl,0.000001);
  let denominator=1.0-nh*nh*(1.0-a2);
  let distribution=a2/(denominator*denominator)*0.3183098861837907;
  let singleDirect=f*visibility*distribution;
  let fabV=textureSampleLevel(dfg,dfgSampler,vec2f(roughness,nv),0.0).rg;
  let fabL=textureSampleLevel(dfg,dfgSampler,vec2f(roughness,nl),0.0).rg;
  let ssV=f0*fabV.x+vec3f(fabV.y);
  let ssL=f0*fabL.x+vec3f(fabL.y);
  let emsV=1.0-(fabV.x+fabV.y);
  let emsL=1.0-(fabL.x+fabL.y);
  let favg=f0+(vec3f(1)-f0)*0.047619;
  let directMs=ssV*ssL*favg/(vec3f(1)-emsV*emsL*favg*favg+vec3f(0.000001))*(emsV*emsL);
  let diffuseContribution=base*(1.0-metal);
  let irradiance=nl*(sunRadiance*shadow);
  let directDiffuse=irradiance*(diffuseContribution*0.3183098861837907);
  let directSpecular=irradiance*(singleDirect+directMs);

  let dielectricSs=vec3f(0.04)*fabV.x+vec3f(fabV.y);
  let dielectricAvg=vec3f(0.04)+(vec3f(1)-vec3f(0.04))*0.047619;
  let dielectricMs=dielectricSs*dielectricAvg/(vec3f(1)-emsV*dielectricAvg)*emsV;
  let metallicSs=base*fabV.x+vec3f(fabV.y);
  let metallicAvg=base+(vec3f(1)-base)*0.047619;
  let metallicMs=metallicSs*metallicAvg/(vec3f(1)-emsV*metallicAvg)*emsV;
  let singleScatter=mix(dielectricSs,metallicSs,metal);
  let multiScatter=mix(dielectricMs,metallicMs,metal);
  let diffuse=diffuseContribution*(vec3f(1)-(dielectricSs+dielectricMs));
  let reflected=normalize(mix(reflect(-v,n),n,roughness*roughness*roughness*roughness));
  let radiance=samplePmrem(pmrem,linear,vec3f(reflected.x,-reflected.y,reflected.z),roughness,maxMip)*environmentIntensity;
  let iblIrradiance=samplePmrem(pmrem,linear,vec3f(n.x,-n.y,n.z),1.0,maxMip)*3.141592653589793*environmentIntensity;
  let cosineIrradiance=iblIrradiance*0.3183098861837907;
  let aoSpecular=clamp(ao-(1.0-pow(nv+ao,exp2(-(1.0+16.0*roughness)))),0.0,1.0);
  let indirectSpecular=(radiance*singleScatter+multiScatter*cosineIrradiance)*aoSpecular;
  let indirectDiffuse=diffuse*cosineIrradiance*ao;
  return max((directDiffuse+indirectDiffuse)+(directSpecular+indirectSpecular)+emissive,vec3f(0));
}

fn lightAlgorithm(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normalWorld: vec3f, worldPosition: vec3f, shadow: f32, eye: vec3f, sunDirection: vec3f, sunRadiance: vec3f, environmentIntensity: vec3f, maxMip: f32, pmrem: texture_2d<f32>, dfg: texture_2d<f32>, linear: sampler) -> vec3f {
 return standardPbr(base,emissive,roughness,geomRoughness,metal,ao,normalize(normalWorld),normalize(eye-worldPosition),sunDirection,sunRadiance,shadow,environmentIntensity,pmrem,linear,maxMip,dfg,linear);
 }

@group(3) @binding(3) var pmrem: texture_2d<f32>;

@group(3) @binding(4) var dfg: texture_2d<f32>;

@group(3) @binding(5) var linear: sampler;

fn surfaceLight(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec3f {
  let sun_1 = mix(data.settings.y, 1f, shadow);
  return lightAlgorithm(base, (emissive + castLight(base, normal, position)), roughness, geomRoughness, metal, ao, normal, position, sun_1, eye, data.sunDirection.xyz, data.sunRadiance.xyz, data.fill.xyz, data.settings.x, pmrem, dfg, linear);
}

fn equirectUv(direction: vec3f) -> vec2f {
  return vec2f(atan2(direction.z, direction.x) * 1.5915494309189535e-1f + 0.5,
    asin(clamp(direction.y, -1.0, 1.0)) * 3.1830988618379069e-1f + 0.5);
}

fn applyAerial(surface: vec4f, position: vec3f, camera: vec3f, observer: vec3f, sun: vec3f, air: Aerial, lut: texture_2d<f32>, linear: sampler) -> vec4f {
    let reach=position-observer;
    let distM=length(reach)/max(1.0,distance(camera,observer)/air.overviewFromM);
    let distKm=max(distM/1000.0-air.clearRadiusKm,0.0);
    let rangeDepth=pow(max(distM-air.rangeNearM,0.0)/air.rangeFarM,air.rangePower)*air.rangeStrength;
    let heightMist=1.0-smoothstep(air.mistHeightM.x,air.mistHeightM.y,position.z);
    let farMist=smoothstep(air.mistDistanceM.x,air.mistDistanceM.y,distM);
    let mistWeight=heightMist*farMist;
    let transmit=exp(-(air.extinction*distKm+vec3f(rangeDepth)+vec3f(mistWeight*air.mistOpacity)));
    let view=normalize(position-camera);
    let viewSky=textureSample(lut,linear,equirectUv(normalize(vec3f(view.xy,max(view.z,4.0000000000000001e-3f))))).rgb;
    let horizonView=normalize(vec3f(view.x,view.y,4.0000000000000001e-3f));
    let horizonSky=textureSample(lut,linear,equirectUv(horizonView)).rgb;
    let below=smoothstep(air.horizonFade.x,0.0,view.z);
    let above=1.0-smoothstep(0.0,air.horizonFade.y,view.z);
    let sunMie=pow(clamp(dot(view,sun),0.0,1.0),air.sunPower)*air.sunStrength;
    let skyLight=mix(mix(viewSky,horizonSky,below*above),air.sunTint,sunMie);
    let mistSkyLight=mix(skyLight,air.mistColor,mistWeight*air.mistColorStrength);
    return vec4f(surface.rgb*transmit+mistSkyLight*(vec3f(1)-transmit),surface.a);
  }

@group(3) @binding(2) var sky: texture_2d<f32>;

fn air(colour: vec3f, position: vec3f, eye: vec3f) -> vec4f {
  return applyAerial(vec4f(colour, 1f), position, eye, data.observer.xyz, data.sunDirection.xyz, data.aerial, sky, linear);
}

fn shade(base: vec3f, emissive: vec3f, roughness: f32, geomRoughness: f32, metal: f32, ao: f32, normal: vec3f, position: vec3f, shadow: f32, eye: vec3f) -> vec4f {
  return air(surfaceLight(base, emissive, roughness, geomRoughness, metal, ao, normal, position, shadow, eye), position, eye);
}

struct GroundPaintStyle {
  albedo: f32,
  emissive: f32,
  fogKeep: f32,
  grassGlow: f32,
  grassFalloff: f32,
  saturation: f32,
  pad1: f32,
  pad2: f32,
}

@group(2) @binding(6) var<uniform> paintStyle: GroundPaintStyle;

fn paintColour(paint: vec4f) -> vec3f {
  let c = pow(max(paint.xyz / max(paint.w, 1e-4), vec3f(0.0)), vec3f(2.2));
  let grey = dot(c, vec3f(0.2126, 0.7152, 0.0722));
  return max(mix(vec3f(grey), c, paintStyle.saturation), vec3f(0.0));
}

fn grassPaintGlow(paint: vec4f, height: f32) -> vec3f {
  if (paint.w <= 0.0) { return vec3f(0.0); }
  let s = paintStyle;
  let colour = paintColour(paint);
  return colour * s.grassGlow * paint.w * exp(-max(height, 0.0) / max(s.grassFalloff, 1e-3));
}

fn fogCoverage(seen: f32, alpha: f32) -> vec4f {
  if (params_1.enabled == 0u || layer.seen == 1u) {
    return vec4f(0.0, 0.0, 0.0, alpha);
  }
  return vec4f(1.0 - seen, seen, f32(layer.ground), alpha);
}

struct fragment_Output {
  @location(0) color: vec4f,
  @location(1) fog: vec4f,
}

struct fragment_Input {
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}

@fragment fn fragment(_arg_0: fragment_Input, @builtin(position) clip: vec4f) -> fragment_Output {
  let eye = (&cam.eye);
  let up = vec3f(0, 0, 1);
  let seen = fogTerm(vec3f(_arg_0.world.x, _arg_0.world.y, _arg_0.root.z), up, clip.xy, fogIsGround());
  var n = normalize(_arg_0.normal);
  if ((dot(n, ((*eye) - _arg_0.world)) < 0f)) {
    n = -(n);
  }
  let shading = normalize(mix(n, up, _arg_0.ground));
  let sun_1 = sampleSunShadow(_arg_0.world, up, clip.xy);
  let under = groundPaint(vec3f(_arg_0.world.x, _arg_0.world.y, _arg_0.root.z));
  let here = groundPaintAtPixel(clip.xy);
  let lit = shade(_arg_0.albedo, vec3f(), 0.9f, 0f, 0f, 1f, shading, _arg_0.world, sun_1, (*eye));
  return fragment_Output(vec4f((lit.xyz + max(grassPaintGlow(under, (_arg_0.world.z - _arg_0.root.z)), grassPaintGlow(here, 0f))), 1f), fogCoverage(seen, 1f));
}