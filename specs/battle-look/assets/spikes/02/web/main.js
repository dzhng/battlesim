// Spike 02 (throwaway): per-pixel sight fog prototype. Two candidates on the
// same inputs: (A) sight lights with per-eye polar horizon maps, culled per
// 16x16 screen tile; (B) a per-side 1 m viewshed bitset marched in compute.
const W = 1920, H = 1080, TILE = 16;
const TAU = Math.PI * 2;

// ---------- math (column-major) ----------
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const norm = (a) => { const l = Math.hypot(...a); return [a[0] / l, a[1] / l, a[2] / l]; };
function mul(a, b) {
  const o = new Float32Array(16);
  for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) {
    let s = 0; for (let k = 0; k < 4; k++) s += a[k * 4 + r] * b[c * 4 + k]; o[c * 4 + r] = s;
  }
  return o;
}
function perspective(fovy, aspect, near, far) {
  const f = 1 / Math.tan(fovy / 2), o = new Float32Array(16);
  o[0] = f / aspect; o[5] = f; o[10] = far / (near - far); o[11] = -1; o[14] = (near * far) / (near - far);
  return o;
}
function ortho(l, r, b, t, n, f) {
  const o = new Float32Array(16);
  o[0] = 2 / (r - l); o[5] = 2 / (t - b); o[10] = 1 / (n - f);
  o[12] = (l + r) / (l - r); o[13] = (t + b) / (b - t); o[14] = n / (n - f); o[15] = 1;
  return o;
}
function lookAt(eye, target, up = [0, 0, 1]) {
  const z = norm(sub(eye, target)), x = norm(cross(up, z)), y = cross(z, x);
  return Float32Array.of(x[0], y[0], z[0], 0, x[1], y[1], z[1], 0, x[2], y[2], z[2], 0,
    -dot(x, eye), -dot(y, eye), -dot(z, eye), 1);
}
function invert(m) {
  const inv = new Float32Array(16), a = m;
  inv[0] = a[5]*a[10]*a[15]-a[5]*a[11]*a[14]-a[9]*a[6]*a[15]+a[9]*a[7]*a[14]+a[13]*a[6]*a[11]-a[13]*a[7]*a[10];
  inv[4] = -a[4]*a[10]*a[15]+a[4]*a[11]*a[14]+a[8]*a[6]*a[15]-a[8]*a[7]*a[14]-a[12]*a[6]*a[11]+a[12]*a[7]*a[10];
  inv[8] = a[4]*a[9]*a[15]-a[4]*a[11]*a[13]-a[8]*a[5]*a[15]+a[8]*a[7]*a[13]+a[12]*a[5]*a[11]-a[12]*a[7]*a[9];
  inv[12] = -a[4]*a[9]*a[14]+a[4]*a[10]*a[13]+a[8]*a[5]*a[14]-a[8]*a[6]*a[13]-a[12]*a[5]*a[10]+a[12]*a[6]*a[9];
  inv[1] = -a[1]*a[10]*a[15]+a[1]*a[11]*a[14]+a[9]*a[2]*a[15]-a[9]*a[3]*a[14]-a[13]*a[2]*a[11]+a[13]*a[3]*a[10];
  inv[5] = a[0]*a[10]*a[15]-a[0]*a[11]*a[14]-a[8]*a[2]*a[15]+a[8]*a[3]*a[14]+a[12]*a[2]*a[11]-a[12]*a[3]*a[10];
  inv[9] = -a[0]*a[9]*a[15]+a[0]*a[11]*a[13]+a[8]*a[1]*a[15]-a[8]*a[3]*a[13]-a[12]*a[1]*a[11]+a[12]*a[3]*a[9];
  inv[13] = a[0]*a[9]*a[14]-a[0]*a[10]*a[13]-a[8]*a[1]*a[14]+a[8]*a[2]*a[13]+a[12]*a[1]*a[10]-a[12]*a[2]*a[9];
  inv[2] = a[1]*a[6]*a[15]-a[1]*a[7]*a[14]-a[5]*a[2]*a[15]+a[5]*a[3]*a[14]+a[13]*a[2]*a[7]-a[13]*a[3]*a[6];
  inv[6] = -a[0]*a[6]*a[15]+a[0]*a[7]*a[14]+a[4]*a[2]*a[15]-a[4]*a[3]*a[14]-a[12]*a[2]*a[7]+a[12]*a[3]*a[6];
  inv[10] = a[0]*a[5]*a[15]-a[0]*a[7]*a[13]-a[4]*a[1]*a[15]+a[4]*a[3]*a[13]+a[12]*a[1]*a[7]-a[12]*a[3]*a[5];
  inv[14] = -a[0]*a[5]*a[14]+a[0]*a[6]*a[13]+a[4]*a[1]*a[14]-a[4]*a[2]*a[13]-a[12]*a[1]*a[6]+a[12]*a[2]*a[5];
  inv[3] = -a[1]*a[6]*a[11]+a[1]*a[7]*a[10]+a[5]*a[2]*a[11]-a[5]*a[3]*a[10]-a[9]*a[2]*a[7]+a[9]*a[3]*a[6];
  inv[7] = a[0]*a[6]*a[11]-a[0]*a[7]*a[10]-a[4]*a[2]*a[11]+a[4]*a[3]*a[10]+a[8]*a[2]*a[7]-a[8]*a[3]*a[6];
  inv[11] = -a[0]*a[5]*a[11]+a[0]*a[7]*a[9]+a[4]*a[1]*a[11]-a[4]*a[3]*a[9]-a[8]*a[1]*a[7]+a[8]*a[3]*a[5];
  inv[15] = a[0]*a[5]*a[10]-a[0]*a[6]*a[9]-a[4]*a[1]*a[10]+a[4]*a[2]*a[9]+a[8]*a[1]*a[6]-a[8]*a[2]*a[5];
  let det = a[0]*inv[0]+a[1]*inv[4]+a[2]*inv[8]+a[3]*inv[12];
  det = 1 / det; for (let i = 0; i < 16; i++) inv[i] *= det;
  return inv;
}

// ---------- WGSL ----------
const PRELUDE = /* wgsl */ `
struct Uni {
  viewProj: mat4x4f, invViewProj: mat4x4f, sunViewProj: mat4x4f,
  camPos: vec4f, sunDir: vec4f,
  mode: vec4u,   // tech (0 none, 1 sight lights, 2 viewshed), debug mask, directional, eye count
  tiles: vec4u,  // tilesX, tilesY, maxPerTile, occluding prop count
  mapA: vec4u,   // AZ, R, forest count, viewshed rays per eye (max)
  geo: vec4u,    // height nx, ny, viewshed bW, bH
  g2: vec4u,     // fog cells nx, ny
  f0: vec4f,     // D0, targetH, forestAtt, forestFull
  f1: vec4f,     // height spacing, viewshed cell, map width, map depth
  f2: vec4f,     // dim, cool, screen W, screen H
  f3: vec4f,     // fog cell, render lift for props
};
struct Eye { pos: vec3f, range: f32, fwd: f32, front: f32, side: f32, rear: f32 };
struct Prop { a: vec4f, b: vec4f };
@group(0) @binding(0) var<uniform> U: Uni;
@group(0) @binding(1) var<storage, read> eyes: array<Eye>;
@group(0) @binding(2) var<storage, read> HT: array<f32>;
@group(0) @binding(3) var<storage, read> props: array<Prop>;
@group(0) @binding(4) var<storage, read> forests: array<vec4f>;
const PI = 3.14159265;
const TAU = 6.28318531;

fn inMap(p: vec2f) -> bool { return p.x >= 0.0 && p.y >= 0.0 && p.x <= U.f1.z && p.y <= U.f1.w; }
fn heightAt(x: f32, y: f32) -> f32 {
  let s = U.f1.x; let nx = U.geo.x; let ny = U.geo.y;
  let fx = clamp(x / s, 0.0, f32(nx - 1u)); let fy = clamp(y / s, 0.0, f32(ny - 1u));
  let i = min(u32(fx), nx - 2u); let j = min(u32(fy), ny - 2u);
  let u = fx - f32(i); let v = fy - f32(j);
  let h00 = HT[j * nx + i]; let h11 = HT[(j + 1u) * nx + i + 1u];
  if (u >= v) { let h10 = HT[j * nx + i + 1u]; return h00 + u * (h10 - h00) + v * (h11 - h10); }
  let h01 = HT[(j + 1u) * nx + i]; return h00 + v * (h01 - h00) + u * (h11 - h01);
}
fn canopyAt(x: f32, y: f32) -> f32 {
  var c = 0.0;
  for (var f = 0u; f < U.mapA.z; f++) {
    let r = forests[2u * f];
    if (x >= r.x && y >= r.y && x <= r.x + r.z && y <= r.y + r.w) { c = max(c, forests[2u * f + 1u].x); }
  }
  return c;
}
fn dAtL(k: i32, lnr: f32) -> f32 {
  if (k < 0) { return 0.0; }
  return U.f0.x * exp(f32(k) / f32(U.mapA.y - 1u) * lnr);
}
fn unpackBin(v: u32) -> vec3f {
  return vec3f(unpack2x16float(v & 0xffffu).x, f32((v >> 16u) & 0xffu) / 255.0, f32(v >> 24u) * 0.5);
}
fn shapeMult(e: Eye, theta: f32) -> f32 {
  var a = abs(theta - e.fwd); a = a - floor(a / TAU) * TAU; if (a > PI) { a = TAU - a; }
  let h = PI * 0.5;
  if (a <= h) { return mix(e.front, e.side, a / h); }
  return mix(e.side, e.rear, (a - h) / h);
}
`;

const LOOKUP = /* wgsl */ `
@group(1) @binding(0) var<storage, read> maps: array<u32>;
@group(1) @binding(3) var<storage, read> bits: array<u32>;
// Bin k covers (d_{k-1}, d_k]: horizon h_k (f16) is the max occluder slope up
// to d_k; the horizon jumps from h_{k-1} to h_k at fraction f of the bin
// (8 bits); foliage metres before d_k in 0.5 m steps (8 bits).
// (h before the jump, h after, jump distance, foliage before, foliage after)
fn binAt(base: u32, ai: u32, k: u32, lnr: f32) -> array<f32, 5> {
  let cur = unpackBin(maps[base + ai * U.mapA.y + k]);
  var prev = vec3f(-1e4, 0.0, 0.0);
  if (k > 0u) { prev = unpackBin(maps[base + ai * U.mapA.y + k - 1u]); }
  let tj = mix(dAtL(i32(k) - 1, lnr), dAtL(i32(k), lnr), cur.y);
  return array<f32, 5>(prev.x, cur.x, tj, prev.z, cur.z);
}
fn seenA(ei: u32, p: vec3f, n: vec3f) -> bool {
  let e = eyes[ei];
  // A prop or unit face turned away from this eye is not seen by it.
  if (dot(n, e.pos - p) <= 0.0 && dot(n, n) > 0.0) { return false; }
  let dxy = p.xy - e.pos.xy; let dist = length(dxy);
  if (dist < 0.75) { return true; }
  let theta = atan2(dxy.y, dxy.x);
  var range = e.range;
  if (U.mode.z == 1u) { range = range * shapeMult(e, theta); }
  if (dist > range) { return false; }
  let AZ = U.mapA.x; let R = U.mapA.y; let D0 = U.f0.x;
  let lnr = log(e.range / D0);
  var k = 0u;
  if (dist > D0) { k = min(u32(ceil(log(dist / D0) / lnr * f32(R - 1u))), R - 1u); }
  let a = (theta + PI) / TAU * f32(AZ) - 0.5;
  let af = floor(a); let fa = a - af;
  let i0 = u32(i32(af) + i32(AZ)) % AZ; let i1 = (i0 + 1u) % AZ;
  let base = ei * AZ * R;
  // Blend the two azimuth neighbours' values and jump distance, then pick
  // before/after once: a facade's jump distance varies smoothly with azimuth.
  let b0 = binAt(base, i0, k, lnr); let b1 = binAt(base, i1, k, lnr);
  let eps = 0.1 + (dAtL(i32(k), lnr) - dAtL(i32(k) - 1, lnr)) / 255.0;
  var m: vec2f;
  // How far a planar face's entry distance moves across one azimuth bin at
  // this fragment's incidence (ground: none).
  var tanInc = 0.0;
  if (dot(n, n) > 0.0 && length(n.xy) > 1e-3) {
    let c = clamp(abs(dot(dxy / dist, normalize(n.xy))), 0.017, 1.0);
    tanInc = sqrt(1.0 - c * c) / c;
  }
  let spread = 1.0 + 0.01 * dist + 1.5 * dist * (TAU / f32(AZ)) * tanInc;
  if (abs(b0[2] - b1[2]) < spread) {
    // Same facade on both rays: its jump distance is smooth in azimuth.
    let tj = mix(b0[2], b1[2], fa);
    m = vec2f(mix(b0[0], b1[0], fa), mix(b0[3], b1[3], fa));
    if (dist > tj + eps) { m = vec2f(mix(b0[1], b1[1], fa), mix(b0[4], b1[4], fa)); }
  } else {
    // Different occluders (a silhouette corner): choose per ray, then blend.
    let m0 = select(vec2f(b0[0], b0[3]), vec2f(b0[1], b0[4]), dist > b0[2] + eps);
    let m1 = select(vec2f(b1[0], b1[3]), vec2f(b1[1], b1[4]), dist > b1[2] + eps);
    m = mix(m0, m1, fa);
  }
  // Foliage grows continuously along the ray: interpolate within the bin.
  let lo = dAtL(i32(k) - 1, lnr); let hi = dAtL(i32(k), lnr);
  let u = clamp((dist - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
  let fol = mix(mix(b0[3], b0[4], u), mix(b1[3], b1[4], u), fa);
  if (fol >= U.f0.w) { return false; }
  if (dist > range * exp(-fol / U.f0.z)) { return false; }
  return (p.z - e.pos.z) / dist >= m.x;
}
fn bitB(i: i32, j: i32) -> f32 {
  if (i < 0 || j < 0 || i >= i32(U.geo.z) || j >= i32(U.geo.w)) { return 0.0; }
  let k = u32(j) * U.geo.z + u32(i);
  return f32((bits[k >> 5u] >> (k & 31u)) & 1u);
}
fn seenB(xy: vec2f) -> f32 {
  let g = xy / U.f1.y - 0.5; let f0 = floor(g); let f = g - f0;
  let i = i32(f0.x); let j = i32(f0.y);
  let v = mix(mix(bitB(i, j), bitB(i + 1, j), f.x), mix(bitB(i, j + 1), bitB(i + 1, j + 1), f.x), f.y);
  return step(0.5, v);
}
`;

const RENDER = PRELUDE + LOOKUP + /* wgsl */ `
@group(1) @binding(1) var<storage, read> lists: array<u32>;
@group(1) @binding(2) var<storage, read> counts: array<u32>;
@group(1) @binding(4) var shadowMap: texture_depth_2d;
@group(1) @binding(5) var shadowSampler: sampler_comparison;
struct VOut {
  @invariant @builtin(position) pos: vec4f,
  @location(0) world: vec3f, @location(1) normal: vec3f, @location(2) color: vec3f, @location(3) lift: f32,
};
@vertex fn vs(@location(0) p: vec3f, @location(1) n: vec3f, @location(2) c: vec3f, @location(3) lift: f32) -> VOut {
  var o: VOut; o.pos = U.viewProj * vec4f(p, 1.0); o.world = p; o.normal = n; o.color = c; o.lift = lift; return o;
}
@vertex fn vsShadow(@location(0) p: vec3f) -> @builtin(position) vec4f { return U.sunViewProj * vec4f(p, 1.0); }

fn fogSeen(p: vec3f, n: vec3f, pix: vec2f) -> f32 {
  if (U.mode.x == 1u) {
    let tile = u32(pix.x) / 16u + (u32(pix.y) / 16u) * U.tiles.x;
    let cnt = min(counts[tile], U.tiles.z);
    for (var s = 0u; s < cnt; s++) { if (seenA(lists[tile * U.tiles.z + s], p, n)) { return 1.0; } }
    return 0.0;
  }
  if (U.mode.x == 2u) { return seenB(p.xy + n.xy * 0.65); }
  return 1.0;
}
fn sunShadow(world: vec3f, n: vec3f) -> f32 {
  let sp = U.sunViewProj * vec4f(world + n * 0.08, 1.0);
  let uv = sp.xy * vec2f(0.5, -0.5) + 0.5;
  if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) { return 1.0; }
  let texel = 1.0 / 4096.0;
  var s = 0.0;
  for (var dy = -1; dy <= 1; dy++) { for (var dx = -1; dx <= 1; dx++) {
    s += textureSampleCompareLevel(shadowMap, shadowSampler, uv + vec2f(f32(dx), f32(dy)) * texel, sp.z - 0.0008);
  } }
  return s / 9.0;
}
@fragment fn fs(v: VOut) -> @location(0) vec4f {
  let n = normalize(v.normal);
  let ndl = max(dot(n, U.sunDir.xyz), 0.0);
  let sh = sunShadow(v.world, n);
  let sky = 0.55 + 0.45 * n.z;
  var col = v.color * (vec3f(0.30, 0.36, 0.46) * sky + vec3f(1.05, 0.95, 0.80) * ndl * sh * 1.25);
  // Ground: the simulation's target height above it. Prop and unit faces: a
  // probe just outside the face, and only eyes in front of the face count.
  let isGround = v.lift > 0.5;
  let probe = select(v.world + n * U.f3.y, v.world + vec3f(0.0, 0.0, U.f0.y), isGround);
  let seen = fogSeen(probe, select(n, vec3f(0.0), isGround), v.pos.xy);
  if (U.mode.y == 1u) { return vec4f(vec3f(seen), 1.0); }
  let l = dot(col, vec3f(0.2126, 0.7152, 0.0722));
  let unseen = mix(col, vec3f(l) * vec3f(0.55, 0.70, 1.25), U.f2.y) * U.f2.x;
  col = mix(unseen, col, seen);
  return vec4f(pow(col / (col + vec3f(0.9)) * 1.9, vec3f(1.0 / 2.2)), 1.0);
}
`;

const CULL = PRELUDE + /* wgsl */ `
@group(1) @binding(0) var depthTex: texture_depth_2d;
@group(1) @binding(1) var<storage, read_write> lists: array<u32>;
@group(1) @binding(2) var<storage, read_write> counts: array<atomic<u32>>;
var<workgroup> mn: array<atomic<i32>, 2>;
var<workgroup> mx: array<atomic<i32>, 2>;
var<workgroup> hit: atomic<u32>;
@compute @workgroup_size(16, 16) fn cull(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32, @builtin(global_invocation_id) g: vec3u) {
  if (li == 0u) {
    atomicStore(&mn[0], 2147483647); atomicStore(&mn[1], 2147483647);
    atomicStore(&mx[0], -2147483647); atomicStore(&mx[1], -2147483647); atomicStore(&hit, 0u);
  }
  workgroupBarrier();
  let Wd = u32(U.f2.z); let Hd = u32(U.f2.w);
  if (g.x < Wd && g.y < Hd) {
    let d = textureLoad(depthTex, vec2i(g.xy), 0);
    if (d < 1.0) {
      let ndc = vec4f((f32(g.x) + 0.5) / f32(Wd) * 2.0 - 1.0, 1.0 - (f32(g.y) + 0.5) / f32(Hd) * 2.0, d, 1.0);
      let w = U.invViewProj * ndc; let p = w.xyz / w.w;
      atomicMin(&mn[0], i32(floor(p.x * 4.0))); atomicMin(&mn[1], i32(floor(p.y * 4.0)));
      atomicMax(&mx[0], i32(ceil(p.x * 4.0))); atomicMax(&mx[1], i32(ceil(p.y * 4.0)));
      atomicStore(&hit, 1u);
    }
  }
  workgroupBarrier();
  let any = atomicLoad(&hit);
  let lo = vec2f(f32(atomicLoad(&mn[0])), f32(atomicLoad(&mn[1]))) * 0.25;
  let hi = vec2f(f32(atomicLoad(&mx[0])), f32(atomicLoad(&mx[1]))) * 0.25;
  if (any == 0u) { return; }
  let tile = wg.x + wg.y * U.tiles.x;
  for (var e = li; e < U.mode.w; e += 256u) {
    let ey = eyes[e];
    let q = clamp(ey.pos.xy, lo, hi);
    if (distance(q, ey.pos.xy) <= ey.range) {
      let s = atomicAdd(&counts[tile], 1u);
      if (s < U.tiles.z) { lists[tile * U.tiles.z + s] = e; }
    }
  }
}
`;

const BUILD = PRELUDE + /* wgsl */ `
@group(1) @binding(0) var<storage, read_write> mapsW: array<u32>;
const RMAX = 256u;
fn dAt(k: u32, lnr: f32) -> f32 { return U.f0.x * exp(f32(k) / f32(U.mapA.y - 1u) * lnr); }
fn binCeil(t: f32, lnr: f32) -> u32 {
  if (t <= U.f0.x) { return 0u; }
  return u32(ceil(log(t / U.f0.x) / lnr * f32(U.mapA.y - 1u)));
}
PROPS_FN
@compute @workgroup_size(64) fn build(@builtin(global_invocation_id) g: vec3u) {
  let AZ = AZ_EXPR; let R = U.mapA.y;
  let ei = g.x / AZ; let ai = g.x % AZ;
  if (ei >= U.mode.w) { return; }
  let e = eyes[ei];
  let theta = (f32(ai) + 0.5) / f32(AZ) * TAU - PI;
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.range / U.f0.x);
  var pm: array<f32, RMAX>;
  var pt: array<f32, RMAX>;
  for (var k = 0u; k < R; k++) { pm[k] = -1e4; pt[k] = 0.0; }
  if (WITH_PROPS) { propEvents(e, dir, lnr, R, &pm, &pt); }
  BODY
}
`;
const PROPS_FN = /* wgsl */ `
fn propEvents(e: Eye, dir: vec2f, lnr: f32, R: u32, pm: ptr<function, array<f32, RMAX>>, pt: ptr<function, array<f32, RMAX>>) {
  for (var pi = 0u; pi < U.tiles.w; pi++) {
    let pr = props[pi];
    let c = cos(pr.a.z); let s = sin(pr.a.z);
    let o0 = e.pos.xy - pr.a.xy;
    let rad = length(vec2f(pr.a.w, pr.b.x));
    if (length(o0) - rad > e.range) { continue; }
    let o = vec2f(o0.x * c + o0.y * s, -o0.x * s + o0.y * c);
    let d = vec2f(dir.x * c + dir.y * s, -dir.x * s + dir.y * c);
    let h = vec2f(pr.a.w, pr.b.x);
    var tn = -1e9; var tf = 1e9; var miss = false;
    if (abs(d.x) < 1e-7) { if (abs(o.x) > h.x) { miss = true; } }
    else { let t1 = (-h.x - o.x) / d.x; let t2 = (h.x - o.x) / d.x; tn = max(tn, min(t1, t2)); tf = min(tf, max(t1, t2)); }
    if (abs(d.y) < 1e-7) { if (abs(o.y) > h.y) { miss = true; } }
    else { let t1 = (-h.y - o.y) / d.y; let t2 = (h.y - o.y) / d.y; tn = max(tn, min(t1, t2)); tf = min(tf, max(t1, t2)); }
    if (miss || tn > tf || tf <= 0.0 || tn < 0.0 || tn > e.range) { continue; }
    let rel = pr.b.y - e.pos.z;
    // Above the eye the steepest point is the entry; below it, the exit.
    var te = tn; var sl = rel / max(tn, 0.25);
    if (rel < 0.0) { te = min(tf, e.range); sl = rel / max(te, 0.25); }
    let k0 = binCeil(te, lnr);
    if (k0 >= R) { continue; }
    if (sl > (*pm)[k0]) { (*pm)[k0] = sl; (*pt)[k0] = te; }
  }
}
`;
const MARCH_BODY = /* wgsl */ `
  var hor = -1e4; var hk = -1e4; var fol = 0.0; var k = 0u; var t = 0.0; var next8 = U.f3.x;
  // Largest increase inside the current bin and where it happened.
  var jump = 0.0; var jt = 0.0;
  let base = ei * AZ * R + ai * R;
  loop {
    let st = clamp(t * 0.01, 0.5, 4.0);
    t = t + st;
    var outside = false;
    let p = e.pos.xy + dir * t;
    if (!inMap(p)) { outside = true; t = 1e9; }
    while (k < R && dAtL(i32(k), lnr) <= t) {
      let lo = dAtL(i32(k) - 1, lnr); let hi = dAtL(i32(k), lnr);
      var v = max(hk, hor);
      if (v - hk > jump) { jump = v - hk; jt = hi; }
      if (pm[k] > v) { if (pm[k] - hk > jump) { jump = pm[k] - hk; jt = pt[k]; } v = pm[k]; }
      let f = clamp((jt - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
      let hb = pack2x16float(vec2f(clamp(v, -6e4, 6e4), 0.0)) & 0xffffu;
      let fb = u32(round(f * 255.0));
      let lb = min(u32(round(fol * 2.0)), 255u);
      mapsW[base + k] = hb | (fb << 16u) | (lb << 24u);
      hk = v; hor = max(hor, v);
      jump = 0.0; jt = hi;
      k++;
    }
    if (k >= R || outside) { break; }
    let gz = heightAt(p.x, p.y);
    let ts = (gz + U.f0.y - e.pos.z) / t;
    if (U.f3.z > 0.0) {
      // The sweep's rule: whole fog cells of foliage at each cell-step point.
      let cell = U.f3.x;
      loop {
        if (next8 > t) { break; }
        let q = e.pos.xy + dir * next8;
        let qz = heightAt(q.x, q.y);
        let qc = canopyAt(q.x, q.y);
        let qs = (qz + U.f0.y - e.pos.z) / next8;
        if (qc > 0.0 && e.pos.z + max(hor, qs) * next8 < qz + qc) { fol += cell; }
        next8 += cell;
      }
    } else {
      let can = canopyAt(p.x, p.y);
      if (can > 0.0 && e.pos.z + max(hor, ts) * t < gz + can) { fol += st; }
    }
    let nh = max(hor, (gz - e.pos.z) / t);
    if (nh - hor > jump) { jump = nh - hor; jt = t; }
    hor = nh;
  }
`;
// Merge: fine-azimuth prop events over the coarse terrain map.
const MERGE_BODY = /* wgsl */ `
  let AZT = U.g2.z;
  let a = (theta + PI) / TAU * f32(AZT) - 0.5; let af = floor(a); let fa = a - af;
  let j0 = u32(i32(af) + i32(AZT)) % AZT; let j1 = (j0 + 1u) % AZT;
  let tb = ei * AZT * R;
  let base = ei * AZ * R + ai * R;
  var hk = -1e4;
  for (var k = 0u; k < R; k++) {
    let c0 = unpackBin(terr[tb + j0 * R + k]); let c1 = unpackBin(terr[tb + j1 * R + k]);
    let tv = mix(c0.x, c1.x, fa);
    var f = select(c0.y, c1.y, fa > 0.5);
    let fol = mix(c0.z, c1.z, fa);
    var v = max(hk, tv);
    if (pm[k] > v) {
      let lo = dAtL(i32(k) - 1, lnr); let hi = dAtL(i32(k), lnr);
      f = clamp((pt[k] - lo) / max(hi - lo, 1e-6), 0.0, 1.0); v = pm[k];
    }
    hk = v;
    let hb = pack2x16float(vec2f(clamp(v, -6e4, 6e4), 0.0)) & 0xffffu;
    mapsW[base + k] = hb | (u32(round(f * 255.0)) << 16u) | (min(u32(round(fol * 2.0)), 255u) << 24u);
  }
`;
const buildVariant = (az, props, body, extra = "") => BUILD.replace("PROPS_FN", PROPS_FN + extra)
  .replace("AZ_EXPR", az).replace("WITH_PROPS", props).replace("BODY", body);

const SWEEP = PRELUDE + /* wgsl */ `
@group(1) @binding(0) var<storage, read> raster: array<f32>;
@group(1) @binding(1) var<storage, read_write> bitsW: array<atomic<u32>>;
fn mark(i: u32, j: u32) { let idx = j * U.geo.z + i; atomicOr(&bitsW[idx >> 5u], 1u << (idx & 31u)); }
@compute @workgroup_size(64) fn sweep(@builtin(global_invocation_id) g: vec3u) {
  let rmax = U.mapA.w;
  let ei = g.x / rmax; let r = g.x % rmax;
  if (ei >= U.mode.w) { return; }
  let e = eyes[ei]; let cell = U.f1.y;
  let rays = max(64u, u32(ceil(TAU * e.range / cell)));
  if (r >= rays) { return; }
  if (r == 0u && inMap(e.pos.xy)) { mark(u32(e.pos.x / cell), u32(e.pos.y / cell)); }
  let theta = f32(r) / f32(rays) * TAU;
  let dir = vec2f(cos(theta), sin(theta));
  var ray = e.range;
  if (U.mode.z == 1u) { ray = ray * shapeMult(e, theta); }
  let steps = u32(ceil(ray / cell));
  var hor = -1e4; var fol = 0.0;
  for (var k = 1u; k <= steps; k++) {
    let dist = f32(k) * cell;
    let p = e.pos.xy + dir * dist;
    if (!inMap(p)) { break; }
    let i = u32(p.x / cell); let j = u32(p.y / cell);
    if (i >= U.geo.z || j >= U.geo.w) { break; }
    let gz = heightAt(p.x, p.y);
    let slope = (gz + U.f0.y - e.pos.z) / dist;
    if (fol >= U.f0.w || dist > ray * exp(-fol / U.f0.z)) { break; }
    if (slope >= hor) { mark(i, j); }
    let can = canopyAt(p.x, p.y);
    if (can > 0.0 && e.pos.z + max(hor, slope) * dist < gz + can) { fol += cell; }
    hor = max(hor, (max(gz, raster[j * U.geo.z + i]) - e.pos.z) / dist);
  }
}
`;

const AGREE = PRELUDE + LOOKUP + /* wgsl */ `
@group(1) @binding(2) var<storage, read_write> outCells: array<u32>;
@compute @workgroup_size(64) fn agree(@builtin(global_invocation_id) g: vec3u) {
  let fnx = U.g2.x; let fny = U.g2.y;
  let i = g.x % fnx; let j = g.x / fnx;
  if (j >= fny) { return; }
  let c = U.f3.x;
  let x = (f32(i) + 0.5) * c; let y = (f32(j) + 0.5) * c;
  if (!inMap(vec2f(x, y))) { outCells[g.x] = 0u; return; }
  let p = vec3f(x, y, heightAt(x, y) + U.f0.y);
  var seen = 0u;
  if (U.mode.x == 1u) {
    for (var e = 0u; e < U.mode.w; e++) { if (seenA(e, p, vec3f(0.0))) { seen = 1u; break; } }
  } else {
    seen = u32(seenB(p.xy) > 0.5);
  }
  outCells[g.x] = seen;
}
`;

// ---------- state ----------
const S = {
  tech: 0, debug: 0, directional: 1, AZ: 4096, R: 64, AZT: 512, split: 1, D0: 1, maxPerTile: 256,
  dim: 0.42, cool: 0.55, bCell: 1, liftProps: 0.1,
  cam: { eye: [0, 0, 100], target: [100, 100, 0], fov: 40 }, shadowHalf: 200,
};
let device, data, res = {}, pipes = {}, layouts = {};

function log(...a) { console.log("[spike02]", ...a); }

async function init() {
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
  const L = adapter.limits;
  device = await adapter.requestDevice({
    requiredFeatures: adapter.features.has("timestamp-query") ? ["timestamp-query"] : [],
    requiredLimits: {
      maxStorageBufferBindingSize: L.maxStorageBufferBindingSize,
      maxBufferSize: L.maxBufferSize,
      maxStorageBuffersPerShaderStage: L.maxStorageBuffersPerShaderStage,
      maxComputeWorkgroupsPerDimension: L.maxComputeWorkgroupsPerDimension,
    },
  });
  device.addEventListener("uncapturederror", (e) => console.error("GPU", e.error.message));
  window.spike.info = { vendor: adapter.info?.vendor, architecture: adapter.info?.architecture,
    timestamps: adapter.features.has("timestamp-query"), maxStorage: L.maxStorageBufferBindingSize };
  res.uni = device.createBuffer({ size: 384, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
  res.qs = device.createQuerySet({ type: "timestamp", count: 32 });
  res.qResolve = device.createBuffer({ size: 256, usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC });
  res.qRead = device.createBuffer({ size: 256, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
  res.bqs = device.createQuerySet({ type: "timestamp", count: 4096 });
  res.bResolve = device.createBuffer({ size: 4096 * 8, usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC });
  res.bRead = device.createBuffer({ size: 4096 * 8, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
  res.color = device.createTexture({ size: [W, H], format: "rgba8unorm", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
  res.depth = device.createTexture({ size: [W, H], format: "depth32float", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING });
  res.shadow = device.createTexture({ size: [4096, 4096], format: "depth32float", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING });
  res.shadowSampler = device.createSampler({ compare: "less", magFilter: "linear", minFilter: "linear" });
  res.pixRead = device.createBuffer({ size: W * H * 4, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
  const tilesX = Math.ceil(W / TILE), tilesY = Math.ceil(H / TILE);
  S.tilesX = tilesX; S.tilesY = tilesY;
  res.counts = device.createBuffer({ size: tilesX * tilesY * 4, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC });
  res.lists = device.createBuffer({ size: tilesX * tilesY * S.maxPerTile * 4, usage: GPUBufferUsage.STORAGE });
  res.countsRead = device.createBuffer({ size: tilesX * tilesY * 4, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });

  const V = GPUShaderStage.VERTEX, F = GPUShaderStage.FRAGMENT, C = GPUShaderStage.COMPUTE;
  const ro = { type: "read-only-storage" }, rw = { type: "storage" };
  layouts.g0 = device.createBindGroupLayout({ entries: [
    { binding: 0, visibility: V | F | C, buffer: { type: "uniform" } },
    ...[1, 2, 3, 4].map((b) => ({ binding: b, visibility: V | F | C, buffer: ro })),
  ] });
  layouts.render = device.createBindGroupLayout({ entries: [
    { binding: 0, visibility: F, buffer: ro }, { binding: 1, visibility: F, buffer: ro },
    { binding: 2, visibility: F, buffer: ro }, { binding: 3, visibility: F, buffer: ro },
    { binding: 4, visibility: F, texture: { sampleType: "depth" } },
    { binding: 5, visibility: F, sampler: { type: "comparison" } },
  ] });
  layouts.cull = device.createBindGroupLayout({ entries: [
    { binding: 0, visibility: C, texture: { sampleType: "depth" } },
    { binding: 1, visibility: C, buffer: rw }, { binding: 2, visibility: C, buffer: rw },
  ] });
  layouts.build = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: C, buffer: rw }] });
  layouts.sweep = device.createBindGroupLayout({ entries: [
    { binding: 0, visibility: C, buffer: ro }, { binding: 1, visibility: C, buffer: rw }] });
  layouts.agree = device.createBindGroupLayout({ entries: [
    { binding: 0, visibility: C, buffer: ro }, { binding: 2, visibility: C, buffer: rw }, { binding: 3, visibility: C, buffer: ro }] });
  const pl = (g1) => device.createPipelineLayout({ bindGroupLayouts: g1 ? [layouts.g0, g1] : [layouts.g0] });
  const mod = (code) => {
    const m = device.createShaderModule({ code });
    m.getCompilationInfo().then((i) => i.messages.forEach((x) => console.error("WGSL", x.lineNum, x.message)));
    return m;
  };
  const renderMod = mod(RENDER);
  const vbuf = [{ arrayStride: 40, attributes: [
    { shaderLocation: 0, offset: 0, format: "float32x3" }, { shaderLocation: 1, offset: 12, format: "float32x3" },
    { shaderLocation: 2, offset: 24, format: "float32x3" }, { shaderLocation: 3, offset: 36, format: "float32" }] }];
  const vbufPos = [{ arrayStride: 40, attributes: [{ shaderLocation: 0, offset: 0, format: "float32x3" }] }];
  pipes.prepass = device.createRenderPipeline({ layout: pl(layouts.render),
    vertex: { module: renderMod, entryPoint: "vs", buffers: vbuf },
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: { format: "depth32float", depthWriteEnabled: true, depthCompare: "less" } });
  pipes.shadow = device.createRenderPipeline({ layout: pl(null),
    vertex: { module: renderMod, entryPoint: "vsShadow", buffers: vbufPos },
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: { format: "depth32float", depthWriteEnabled: true, depthCompare: "less", depthBias: 2, depthBiasSlopeScale: 2 } });
  pipes.main = device.createRenderPipeline({ layout: pl(layouts.render),
    vertex: { module: renderMod, entryPoint: "vs", buffers: vbuf },
    fragment: { module: renderMod, entryPoint: "fs", targets: [{ format: "rgba8unorm" }] },
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: { format: "depth32float", depthWriteEnabled: false, depthCompare: "less-equal" } });
  pipes.cull = device.createComputePipeline({ layout: pl(layouts.cull), compute: { module: mod(CULL), entryPoint: "cull" } });
  pipes.build = device.createComputePipeline({ layout: pl(layouts.build), compute: { module: mod(buildVariant("U.mapA.x", "true", MARCH_BODY)), entryPoint: "build" } });
  pipes.buildT = device.createComputePipeline({ layout: pl(layouts.build), compute: { module: mod(buildVariant("U.g2.z", "false", MARCH_BODY)), entryPoint: "build" } });
  layouts.merge = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: C, buffer: rw }, { binding: 1, visibility: C, buffer: ro }] });
  pipes.merge = device.createComputePipeline({ layout: pl(layouts.merge), compute: { module: mod(buildVariant("U.mapA.x", "true", MERGE_BODY,
    "@group(1) @binding(1) var<storage, read> terr: array<u32>;\n")), entryPoint: "build" } });
  pipes.sweep = device.createComputePipeline({ layout: pl(layouts.sweep), compute: { module: mod(SWEEP), entryPoint: "sweep" } });
  pipes.agree = device.createComputePipeline({ layout: pl(layouts.agree), compute: { module: mod(AGREE), entryPoint: "agree" } });
}

function buf(data, usage = GPUBufferUsage.STORAGE) {
  const b = device.createBuffer({ size: Math.max(16, Math.ceil(data.byteLength / 4) * 4), usage: usage | GPUBufferUsage.COPY_DST });
  device.queue.writeBuffer(b, 0, data);
  return b;
}

// ---------- scene geometry ----------
function heightAtJS(x, y) {
  const s = data.spacing, nx = data.nx, ny = data.ny, h = data.heights;
  const fx = Math.min(Math.max(x / s, 0), nx - 1), fy = Math.min(Math.max(y / s, 0), ny - 1);
  const i = Math.min(Math.floor(fx), nx - 2), j = Math.min(Math.floor(fy), ny - 2);
  const u = fx - i, v = fy - j, h00 = h[j * nx + i], h11 = h[(j + 1) * nx + i + 1];
  if (u >= v) { const h10 = h[j * nx + i + 1]; return h00 + u * (h10 - h00) + v * (h11 - h10); }
  const h01 = h[(j + 1) * nx + i]; return h00 + v * (h01 - h00) + u * (h11 - h01);
}
function distToPolyline(pts, x, y) {
  let best = Infinity;
  for (let k = 0; k + 1 < pts.length; k++) {
    const [ax, ay] = pts[k], [bx, by] = pts[k + 1];
    const dx = bx - ax, dy = by - ay, l2 = dx * dx + dy * dy;
    const t = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / l2));
    best = Math.min(best, Math.hypot(x - ax - t * dx, y - ay - t * dy));
  }
  return best;
}
function hash(x, y) { const s = Math.sin(x * 12.9898 + y * 78.233) * 43758.5453; return s - Math.floor(s); }

function buildGeometry() {
  const verts = [], idx = [];
  const push = (p, n, c, lift) => { verts.push(p[0], p[1], p[2], n[0], n[1], n[2], c[0], c[1], c[2], lift); return verts.length / 10 - 1; };
  // Terrain: the simulation's own vertex grid, finer-subdivided x2 for colour detail.
  const { nx, ny, spacing: s, heights } = data;
  const sub = 2, gx = (nx - 1) * sub + 1, gy = (ny - 1) * sub + 1, step = s / sub;
  const base = verts.length / 10;
  const roads = data.roads ?? [];
  for (let j = 0; j < gy; j++) for (let i = 0; i < gx; i++) {
    const x = i * step, y = j * step, z = heightAtJS(x, y);
    const e = 0.5;
    const n = norm([heightAtJS(x - e, y) - heightAtJS(x + e, y), heightAtJS(x, y - e) - heightAtJS(x, y + e), 2 * e]);
    let c;
    const road = roads.some((r) => distToPolyline(r.points, x, y) < r.width_m / 2);
    const forest = data.forests.some((f) => x >= f[0] && y >= f[1] && x <= f[0] + f[2] && y <= f[1] + f[3]);
    const v = hash(Math.floor(x / 6), Math.floor(y / 6)) * 0.25 + hash(Math.floor(x / 23), Math.floor(y / 23)) * 0.3;
    if (road) c = [0.20 + v * 0.1, 0.18 + v * 0.1, 0.15 + v * 0.08];
    else if (forest) c = [0.05 + v * 0.03, 0.10 + v * 0.05, 0.03];
    else c = [0.13 + v * 0.12, 0.22 + v * 0.12, 0.05 + v * 0.03];
    push([x, y, z], n, c, 1);
  }
  for (let j = 0; j + 1 < gy; j++) for (let i = 0; i + 1 < gx; i++) {
    const sw = base + j * gx + i, se = sw + 1, nw = sw + gx, ne = nw + 1;
    idx.push(sw, se, ne, sw, ne, nw);
  }
  // Boxes.
  const box = (cx, cy, yaw, hx, hy, z0, z1, color, lift = 0) => {
    const c = Math.cos(yaw), sn = Math.sin(yaw);
    const w = (lx, ly, z) => [cx + lx * c - ly * sn, cy + lx * sn + ly * c, z];
    const faces = [
      [[1, 0, 0], [[hx, -hy], [hx, hy]]], [[-1, 0, 0], [[-hx, hy], [-hx, -hy]]],
      [[0, 1, 0], [[hx, hy], [-hx, hy]]], [[0, -1, 0], [[-hx, -hy], [hx, -hy]]],
    ];
    for (const [ln, [[ax, ay], [bx, by]]] of faces) {
      const n = [ln[0] * c - ln[1] * sn, ln[0] * sn + ln[1] * c, 0];
      const a = push(w(ax, ay, z0), n, color, lift), b = push(w(bx, by, z0), n, color, lift);
      const d = push(w(bx, by, z1), n, color, lift), e = push(w(ax, ay, z1), n, color, lift);
      idx.push(a, b, d, a, d, e);
    }
    const n = [0, 0, 1];
    const a = push(w(-hx, -hy, z1), n, color, lift), b = push(w(hx, -hy, z1), n, color, lift);
    const d = push(w(hx, hy, z1), n, color, lift), e = push(w(-hx, hy, z1), n, color, lift);
    idx.push(a, b, d, a, d, e);
  };
  for (const [occ, cx, cy, yaw, hx, hy, hz, bz, kind] of data.props) {
    const col = kind === 0 ? [0.55, 0.50, 0.42] : kind === 1 ? [0.10, 0.07, 0.04] : kind === 2 ? [0.09, 0.09, 0.09] : [0.4, 0.38, 0.33];
    box(cx, cy, yaw, hx, hy, bz, bz + 2 * hz, col);
  }
  // Tree crowns over the trunks (drawn, not occluding: the sim's foliage rule).
  for (const [occ, cx, cy, yaw, hx, hy, hz, bz, kind] of data.props) {
    if (kind !== 1) continue;
    box(cx, cy, 0.4, 2.0, 2.0, bz + 4, bz + 9, [0.05, 0.12, 0.03]);
  }
  for (const [side, kind, x, y, z, yaw, hx, hy, hz, fwd] of data.units) {
    const col = side === 0 ? [0.12, 0.2, 0.55] : [0.55, 0.1, 0.08];
    box(x, y, yaw, hx, hy, z, z + 2 * hz, col);
    if (kind === 3) {
      box(x, y, fwd, 1.4, 1.2, z + 2 * hz, z + 2 * hz + 0.9, col.map((v) => v * 0.8));
      const c = Math.cos(fwd), s2 = Math.sin(fwd);
      box(x + c * 3.2, y + s2 * 3.2, fwd, 2.2, 0.12, z + 2 * hz + 0.35, z + 2 * hz + 0.6, [0.05, 0.05, 0.05]);
    }
  }
  return { verts: new Float32Array(verts), idx: new Uint32Array(idx) };
}

function buildOccluders() {
  const occ = data.props.filter((p) => p[0] === 1);
  const a = new Float32Array(occ.length * 8);
  occ.forEach(([o, cx, cy, yaw, hx, hy, hz, bz], i) => a.set([cx, cy, yaw, hx, hy, bz + 2 * hz, bz, 0], i * 8));
  return { buf: a, count: occ.length };
}
function buildRaster(cell) {
  const bW = Math.ceil(data.size[0] / cell), bH = Math.ceil(data.size[1] / cell);
  const r = new Float32Array(bW * bH).fill(-1e4);
  for (const [o, cx, cy, yaw, hx, hy, hz, bz] of data.props) {
    if (!o) continue;
    const rad = Math.hypot(hx, hy), c = Math.cos(yaw), s = Math.sin(yaw), top = bz + 2 * hz;
    const i0 = Math.max(0, Math.floor((cx - rad) / cell)), i1 = Math.min(bW - 1, Math.floor((cx + rad) / cell));
    const j0 = Math.max(0, Math.floor((cy - rad) / cell)), j1 = Math.min(bH - 1, Math.floor((cy + rad) / cell));
    for (let j = j0; j <= j1; j++) for (let i = i0; i <= i1; i++) {
      const px = (i + 0.5) * cell - cx, py = (j + 0.5) * cell - cy;
      const lx = px * c + py * s, ly = -px * s + py * c;
      if (Math.abs(lx) <= hx && Math.abs(ly) <= hy) r[j * bW + i] = Math.max(r[j * bW + i], top);
    }
  }
  return { r, bW, bH };
}

async function load(name) {
  for (const k of ["eyes", "heights", "props", "forests", "vbuf", "ibuf", "raster", "bits", "cells", "cellsRead"]) {
    res[k]?.destroy(); res[k] = null;
  }
  data = await (await fetch(`/data/${name}.json`)).json();
  const eyes = new Float32Array(Math.max(1, data.eyes.length) * 8);
  data.eyes.forEach((e, i) => eyes.set(e.slice(0, 8), i * 8));
  S.eyeCount = data.eyes.length;
  res.eyes = buf(eyes);
  res.heights = buf(new Float32Array(data.heights));
  const occ = buildOccluders();
  res.props = buf(occ.buf); S.propCount = occ.count;
  const f = new Float32Array(Math.max(1, data.forests.length) * 8);
  data.forests.forEach((x, i) => f.set([x[0], x[1], x[2], x[3], x[4], 0, 0, 0], i * 8));
  res.forests = buf(f);
  const g = buildGeometry();
  res.vbuf = buf(g.verts, GPUBufferUsage.VERTEX); res.ibuf = buf(g.idx, GPUBufferUsage.INDEX); res.icount = g.idx.length;
  const ras = buildRaster(S.bCell);
  res.raster = buf(ras.r); S.bW = ras.bW; S.bH = ras.bH;
  res.bits = device.createBuffer({ size: Math.ceil((ras.bW * ras.bH) / 32) * 4, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
  const cells = data.fog.nx * data.fog.ny;
  res.cells = device.createBuffer({ size: cells * 4, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC });
  res.cellsRead = device.createBuffer({ size: cells * 4, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
  res.g0 = device.createBindGroup({ layout: layouts.g0, entries: [
    { binding: 0, resource: { buffer: res.uni } }, { binding: 1, resource: { buffer: res.eyes } },
    { binding: 2, resource: { buffer: res.heights } }, { binding: 3, resource: { buffer: res.props } },
    { binding: 4, resource: { buffer: res.forests } }] });
  allocMaps();
  return { eyes: S.eyeCount, occluders: occ.count, props: data.props.length, tris: g.idx.length / 3, bW: ras.bW, bH: ras.bH };
}

function allocMaps() {
  res.maps?.destroy();
  const bytes = Math.max(16, S.eyeCount * S.AZ * S.R * 4);
  res.maps = device.createBuffer({ size: bytes, usage: GPUBufferUsage.STORAGE });
  S.mapBytes = bytes;
  res.renderGroup = device.createBindGroup({ layout: layouts.render, entries: [
    { binding: 0, resource: { buffer: res.maps } }, { binding: 1, resource: { buffer: res.lists } },
    { binding: 2, resource: { buffer: res.counts } }, { binding: 3, resource: { buffer: res.bits } },
    { binding: 4, resource: res.shadow.createView() }, { binding: 5, resource: res.shadowSampler }] });
  res.cullGroup = device.createBindGroup({ layout: layouts.cull, entries: [
    { binding: 0, resource: res.depth.createView() }, { binding: 1, resource: { buffer: res.lists } },
    { binding: 2, resource: { buffer: res.counts } }] });
  res.buildGroup = device.createBindGroup({ layout: layouts.build, entries: [{ binding: 0, resource: { buffer: res.maps } }] });
  res.terr?.destroy();
  res.terr = device.createBuffer({ size: Math.max(16, S.eyeCount * S.AZT * S.R * 4), usage: GPUBufferUsage.STORAGE });
  S.terrBytes = S.eyeCount * S.AZT * S.R * 4;
  res.terrGroup = device.createBindGroup({ layout: layouts.build, entries: [{ binding: 0, resource: { buffer: res.terr } }] });
  res.mergeGroup = device.createBindGroup({ layout: layouts.merge, entries: [
    { binding: 0, resource: { buffer: res.maps } }, { binding: 1, resource: { buffer: res.terr } }] });
  res.sweepGroup = device.createBindGroup({ layout: layouts.sweep, entries: [
    { binding: 0, resource: { buffer: res.raster } }, { binding: 1, resource: { buffer: res.bits } }] });
  res.agreeGroup = device.createBindGroup({ layout: layouts.agree, entries: [
    { binding: 0, resource: { buffer: res.maps } }, { binding: 2, resource: { buffer: res.cells } },
    { binding: 3, resource: { buffer: res.bits } }] });
}

function writeUniforms() {
  const b = new ArrayBuffer(384), f = new Float32Array(b), u = new Uint32Array(b);
  const { eye, target, fov } = S.cam;
  const view = lookAt(eye, target);
  const proj = perspective((fov * Math.PI) / 180, W / H, 0.5, 6000);
  const vp = mul(proj, view);
  f.set(vp, 0); f.set(invert(vp), 16);
  const el = (S.sunElevation ?? 30) * Math.PI / 180, az = (S.sunAzimuth ?? 215) * Math.PI / 180;
  const sunDir = [Math.cos(el) * Math.cos(az), Math.cos(el) * Math.sin(az), Math.sin(el)];
  const half = S.shadowHalf, center = S.shadowCenter ?? target;
  const sv = lookAt([center[0] + sunDir[0] * 1500, center[1] + sunDir[1] * 1500, center[2] + sunDir[2] * 1500], center,
    Math.abs(sunDir[2]) > 0.99 ? [0, 1, 0] : [0, 0, 1]);
  f.set(mul(ortho(-half, half, -half, half, 1, 3000), sv), 32);
  f.set([...eye, 0], 48); f.set([...sunDir, 0], 52);
  u.set([S.tech, S.debug, S.directional, S.eyeCount], 56);
  u.set([S.tilesX, S.tilesY, S.maxPerTile, S.propCount], 60);
  u.set([S.AZ, S.R, data.forests.length, Math.ceil(TAU * Math.max(...data.eyes.map((e) => e[3]), 1) / S.bCell)], 64);
  u.set([data.nx, data.ny, S.bW, S.bH], 68);
  u.set([data.fog.nx, data.fog.ny, S.AZT, 0], 72);
  f.set([S.D0, data.sensors.targetH, data.sensors.forestAtt, data.sensors.forestFull], 76);
  f.set([data.spacing, S.bCell, data.size[0], data.size[1]], 80);
  f.set([S.dim, S.cool, W, H], 84);
  f.set([data.sensors.cell, S.liftProps, S.simFoliage ?? 0, 0], 88);
  device.queue.writeBuffer(res.uni, 0, b);
}

// ---------- passes ----------
async function timed(record) {
  const enc = device.createCommandEncoder();
  const names = [];
  const tw = (name) => { const q = names.length * 2; names.push(name);
    return { querySet: res.qs, beginningOfPassWriteIndex: q, endOfPassWriteIndex: q + 1 }; };
  record(enc, tw);
  if (names.length) {
    enc.resolveQuerySet(res.qs, 0, names.length * 2, res.qResolve, 0);
    enc.copyBufferToBuffer(res.qResolve, 0, res.qRead, 0, names.length * 16);
  }
  device.queue.submit([enc.finish()]);
  const out = {};
  if (names.length) {
    await res.qRead.mapAsync(GPUMapMode.READ);
    const t = new BigUint64Array(res.qRead.getMappedRange().slice(0));
    res.qRead.unmap();
    names.forEach((n, i) => { out[n] = Number(t[2 * i + 1] - t[2 * i]) / 1e6; });
  } else await device.queue.onSubmittedWorkDone();
  return out;
}

function drawGeometry(pass) {
  pass.setVertexBuffer(0, res.vbuf); pass.setIndexBuffer(res.ibuf, "uint32"); pass.drawIndexed(res.icount);
}

async function shadowPass() {
  writeUniforms();
  return timed((enc, tw) => {
    const p = enc.beginRenderPass({ colorAttachments: [], depthStencilAttachment: {
      view: res.shadow.createView(), depthClearValue: 1, depthLoadOp: "clear", depthStoreOp: "store" }, timestampWrites: tw("shadow") });
    p.setPipeline(pipes.shadow); p.setBindGroup(0, res.g0); drawGeometry(p); p.end();
  });
}

function encodeBuild(enc, tw) {
  const dispatch = (p, az) => { const n = Math.ceil((S.eyeCount * az) / 64), X = Math.min(n, 65535); p.dispatchWorkgroups(X, Math.ceil(n / X)); };
  if (!S.split) {
    const p = enc.beginComputePass({ timestampWrites: tw("buildA") });
    p.setPipeline(pipes.build); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.buildGroup); dispatch(p, S.AZ); p.end();
    return;
  }
  let p = enc.beginComputePass({ timestampWrites: tw("buildT") });
  p.setPipeline(pipes.buildT); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.terrGroup); dispatch(p, S.AZT); p.end();
  p = enc.beginComputePass({ timestampWrites: tw("merge") });
  p.setPipeline(pipes.merge); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.mergeGroup); dispatch(p, S.AZ); p.end();
}
async function buildA() {
  writeUniforms();
  const r = await timed((enc, tw) => encodeBuild(enc, tw));
  if (S.split) r.buildA = r.buildT + r.merge;
  return r;
}
async function sweepB() {
  writeUniforms();
  const rmax = Math.ceil(TAU * Math.max(...data.eyes.map((e) => e[3])) / S.bCell);
  return timed((enc, tw) => {
    enc.clearBuffer(res.bits);
    const p = enc.beginComputePass({ timestampWrites: tw("sweepB") });
    p.setPipeline(pipes.sweep); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.sweepGroup);
    const n = Math.ceil((S.eyeCount * rmax) / 64);
    p.dispatchWorkgroups(n); p.end();
  });
}

async function frame() {
  writeUniforms();
  return timed((enc, tw) => {
    let p = enc.beginRenderPass({ colorAttachments: [], depthStencilAttachment: {
      view: res.depth.createView(), depthClearValue: 1, depthLoadOp: "clear", depthStoreOp: "store" }, timestampWrites: tw("prepass") });
    p.setPipeline(pipes.prepass); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.renderGroup); drawGeometry(p); p.end();
    if (S.tech === 1) {
      enc.clearBuffer(res.counts);
      const c = enc.beginComputePass({ timestampWrites: tw("cull") });
      c.setPipeline(pipes.cull); c.setBindGroup(0, res.g0); c.setBindGroup(1, res.cullGroup);
      c.dispatchWorkgroups(S.tilesX, S.tilesY); c.end();
    }
    p = enc.beginRenderPass({ colorAttachments: [{ view: res.color.createView(), clearValue: [0.52, 0.6, 0.7, 1], loadOp: "clear", storeOp: "store" }],
      depthStencilAttachment: { view: res.depth.createView(), depthLoadOp: "load", depthStoreOp: "store" }, timestampWrites: tw("main") });
    p.setPipeline(pipes.main); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.renderGroup); drawGeometry(p); p.end();
  });
}

function encodeFrame(enc, tw) {
  let p = enc.beginRenderPass({ colorAttachments: [], depthStencilAttachment: {
    view: res.depth.createView(), depthClearValue: 1, depthLoadOp: "clear", depthStoreOp: "store" }, timestampWrites: tw("prepass") });
  p.setPipeline(pipes.prepass); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.renderGroup); drawGeometry(p); p.end();
  if (S.tech === 1) {
    enc.clearBuffer(res.counts);
    const c = enc.beginComputePass({ timestampWrites: tw("cull") });
    c.setPipeline(pipes.cull); c.setBindGroup(0, res.g0); c.setBindGroup(1, res.cullGroup);
    c.dispatchWorkgroups(S.tilesX, S.tilesY); c.end();
  }
  p = enc.beginRenderPass({ colorAttachments: [{ view: res.color.createView(), clearValue: [0.52, 0.6, 0.7, 1], loadOp: "clear", storeOp: "store" }],
    depthStencilAttachment: { view: res.depth.createView(), depthLoadOp: "load", depthStoreOp: "store" }, timestampWrites: tw("main") });
  p.setPipeline(pipes.main); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.renderGroup); drawGeometry(p); p.end();
}

/** `n` rounds of back-to-back frames, one per tech in `techs` each round
 *  (GPU kept busy, techs interleaved so clock drift is shared); plus one
 *  full sight-light build and one viewshed sweep per round when asked.
 *  Returns per tech, per pass, all samples in ms. */
async function batch(n, techs = [0, 1, 2], { build = false } = {}) {
  const slots = []; let q = 0;
  const saved = S.tech;
  for (let i = 0; i < n; i++) {
    for (const tech of techs) {
      S.tech = tech; writeUniforms();
      const enc = device.createCommandEncoder();
      const tw = (name) => { slots.push({ tech, name, q }); const w = { querySet: res.bqs, beginningOfPassWriteIndex: q, endOfPassWriteIndex: q + 1 }; q += 2; return w; };
      encodeFrame(enc, tw);
      device.queue.submit([enc.finish()]);
    }
    if (build) {
      const enc = device.createCommandEncoder();
      const tw = (name) => { slots.push({ tech: "update", name, q }); const w = { querySet: res.bqs, beginningOfPassWriteIndex: q, endOfPassWriteIndex: q + 1 }; q += 2; return w; };
      encodeBuild(enc, tw);
      enc.clearBuffer(res.bits);
      let p;
      const rmax = Math.ceil(TAU * Math.max(...data.eyes.map((e) => e[3])) / S.bCell);
      p = enc.beginComputePass({ timestampWrites: tw("sweepB") });
      p.setPipeline(pipes.sweep); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.sweepGroup);
      p.dispatchWorkgroups(Math.ceil((S.eyeCount * rmax) / 64)); p.end();
      device.queue.submit([enc.finish()]);
    }
    if (q > 4000) throw new Error("too many queries");
  }
  S.tech = saved; writeUniforms();
  const enc = device.createCommandEncoder();
  enc.resolveQuerySet(res.bqs, 0, q, res.bResolve, 0);
  enc.copyBufferToBuffer(res.bResolve, 0, res.bRead, 0, q * 8);
  device.queue.submit([enc.finish()]);
  await res.bRead.mapAsync(GPUMapMode.READ);
  const t = new BigUint64Array(res.bRead.getMappedRange().slice(0, q * 8));
  res.bRead.unmap();
  const out = {};
  for (const { tech, name, q: k } of slots) {
    ((out[tech] ??= {})[name] ??= []).push(Number(t[k + 1] - t[k]) / 1e6);
  }
  return out;
}

async function capture() {
  await frame();
  const enc = device.createCommandEncoder();
  enc.copyTextureToBuffer({ texture: res.color }, { buffer: res.pixRead, bytesPerRow: W * 4 }, [W, H]);
  device.queue.submit([enc.finish()]);
  await res.pixRead.mapAsync(GPUMapMode.READ);
  const px = new Uint8Array(res.pixRead.getMappedRange().slice(0));
  res.pixRead.unmap();
  let s = ""; const CH = 0x8000;
  for (let i = 0; i < px.length; i += CH) s += String.fromCharCode.apply(null, px.subarray(i, i + CH));
  return btoa(s);
}

async function tileStats() {
  const enc = device.createCommandEncoder();
  enc.copyBufferToBuffer(res.counts, 0, res.countsRead, 0, S.tilesX * S.tilesY * 4);
  device.queue.submit([enc.finish()]);
  await res.countsRead.mapAsync(GPUMapMode.READ);
  const c = new Uint32Array(res.countsRead.getMappedRange().slice(0));
  res.countsRead.unmap();
  const nz = [...c].filter((x) => x > 0).sort((a, b) => a - b);
  return { tiles: c.length, nonEmpty: nz.length, mean: nz.reduce((a, b) => a + b, 0) / Math.max(1, nz.length),
    p50: nz[Math.floor(nz.length / 2)] ?? 0, max: nz[nz.length - 1] ?? 0, overflow: nz.filter((x) => x > S.maxPerTile).length };
}

/** GPU fog at every 8 m cell centre vs the simulation's field. */
async function agreement(which = "sim") {
  writeUniforms();
  const cells = data.fog.nx * data.fog.ny;
  const enc = device.createCommandEncoder();
  const p = enc.beginComputePass();
  p.setPipeline(pipes.agree); p.setBindGroup(0, res.g0); p.setBindGroup(1, res.agreeGroup);
  p.dispatchWorkgroups(Math.ceil(cells / 64)); p.end();
  enc.copyBufferToBuffer(res.cells, 0, res.cellsRead, 0, cells * 4);
  device.queue.submit([enc.finish()]);
  await res.cellsRead.mapAsync(GPUMapMode.READ);
  const gpu = new Uint32Array(res.cellsRead.getMappedRange().slice(0));
  res.cellsRead.unmap();
  const ref = data.fog[which], nx = data.fog.nx, ny = data.fog.ny;
  const bit = (i, j) => (i < 0 || j < 0 || i >= nx || j >= ny) ? -1 : (ref[(j * nx + i) >> 5] >>> ((j * nx + i) & 31)) & 1;
  let band = 0, outside = 0, dis = 0, falseSeen = 0, falseHidden = 0, simSeen = 0, disAll = 0, forestCells = 0, forestDis = 0;
  const diffMap = new Uint8Array(cells);
  for (let j = 0; j < ny; j++) for (let i = 0; i < nx; i++) {
    const s = bit(i, j), g = gpu[j * nx + i];
    if (s !== g) { disAll++; diffMap[j * nx + i] = g ? 2 : 1; }
    let edge = false;
    for (let dj = -1; dj <= 1 && !edge; dj++) for (let di = -1; di <= 1; di++) {
      const b = bit(i + di, j + dj); if (b !== -1 && b !== s) { edge = true; break; }
    }
    if (edge) { band++; continue; }
    outside++; if (s) simSeen++;
    const cx = (i + 0.5) * data.sensors.cell, cy = (j + 0.5) * data.sensors.cell;
    const inForest = data.forests.some((f) => cx >= f[0] && cy >= f[1] && cx <= f[0] + f[2] && cy <= f[1] + f[3]);
    if (inForest) { forestCells++; if (s !== g) forestDis++; }
    if (s !== g) { dis++; if (g) falseSeen++; else falseHidden++; }
  }
  S.lastDiff = diffMap;
  return { cells, band, outside, disagree: dis, pctOutside: (100 * dis) / outside, pctOfSeen: (100 * dis) / Math.max(1, simSeen),
    falseSeen, falseHidden, disagreeAll: disAll, pctAll: (100 * disAll) / cells, simSeen,
    forestCells, forestDis, forestPct: (100 * forestDis) / Math.max(1, forestCells) };
}

/** Replace the eye list (rows as in data.eyes); null restores the dump's. */
function setEyes(rows) {
  const list = rows ?? data.eyes;
  const eyes = new Float32Array(Math.max(1, list.length) * 8);
  list.forEach((e, i) => eyes.set(e.slice(0, 8), i * 8));
  S.eyeCount = list.length;
  res.eyes.destroy();
  res.eyes = buf(eyes);
  res.g0 = device.createBindGroup({ layout: layouts.g0, entries: [
    { binding: 0, resource: { buffer: res.uni } }, { binding: 1, resource: { buffer: res.eyes } },
    { binding: 2, resource: { buffer: res.heights } }, { binding: 3, resource: { buffer: res.props } },
    { binding: 4, resource: { buffer: res.forests } }] });
  allocMaps();
  return S.eyeCount;
}
function eyes() { return data.eyes; }

function set(o) {
  const realloc = (o.AZ && o.AZ !== S.AZ) || (o.R && o.R !== S.R) || (o.AZT && o.AZT !== S.AZT);
  Object.assign(S, o);
  if (o.cam) S.cam = { ...S.cam, ...o.cam };
  if (realloc) allocMaps();
  return { mapBytes: S.mapBytes, terrBytes: S.terrBytes };
}

window.spike = { init, load, set, setEyes, eyes, frame, batch, diff: () => [...S.lastDiff], fogDims: () => [data.fog.nx, data.fog.ny], buildA, sweepB, shadowPass, capture, agreement, tileStats,
  state: () => ({ ...S, lastDiff: undefined }), bytes: () => ({ maps: S.mapBytes, terr: S.split ? S.terrBytes : 0, bits: Math.ceil((S.bW * S.bH) / 32) * 4,
    raster: S.bW * S.bH * 4, lists: S.tilesX * S.tilesY * S.maxPerTile * 4, counts: S.tilesX * S.tilesY * 4 }) };
window.spikeReady = true;
