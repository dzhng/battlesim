// FogVisibility: the GPU half of sight-light fog (spike 02's technique). The
// presentation crate decides everything (`presentation::fog::sight`): each
// frame it hands a `FogUpload`, which this uploads as it is, then runs:
//
// - the rebuilt eyes' horizon maps, `azimuth_bins × radial_bins` words an
//   eye: a coarse terrain march at `terrain_azimuth_bins` rays, then a merge
//   at full azimuth that meets only the known occluders within reach;
// - the structures that take fog whole: one workgroup per reachable
//   occluder samples its walls and roof against every eye in reach and flags
//   it seen if any sample is; the flags join the occluders' boxes and a
//   lookup grid in the `wholes` texture `fogWholeSeen` reads;
// - per frame, a cull over the depth prepass: each screen tile bounds the
//   ground it shows and lists the eyes whose reach touches it.
//
// Every buffer is owned by the frame's registry; the tile lists live in the
// size-dependent scope beside the targets. `probe` evaluates the same lookup
// at given points against every eye (the agreement metric and the lab's
// tests), and `probeShape` runs the WGSL sight shape against Rust's vectors.
// Probes read back: lab only, never in the frame.
import { tgpu, d } from "typegpu";
import type { FogUpload, Presentation } from "@wasm/presentation/presentation_wasm.js";
import type { GpuRegistry, GpuSlot } from "./registry";
import { triangleRuleHeight } from "./triangleRule";
import { terrainSample, type TerrainHeights } from "./terrainHeights";
import type { TerrainGrid } from "../terrain/terrainGrid";
import {
  FogBox,
  FogEye,
  FogParams,
  FOLIAGE_STEP,
  FOG_BOX_WORDS,
  FOG_EYE_WORDS,
  FOG_PARAMS_WORDS,
  WHOLE_TEXTURE_WIDTH,
} from "../records.gen";
import {
  FogLayer,
  GroundPaintStyle,
  fogBinEdge,
  fogLayout,
  fogSeenSurface,
  fogShape,
} from "./fogTerm";

type Root = ReturnType<typeof tgpu.initFromDevice>;

const STORAGE = 0x80; // GPUBufferUsage.STORAGE
const COPY_DST = 0x08;
const COPY_SRC = 0x04;
const MAP_READ = 0x01;
const WORD = 4;
const PROBE_BYTES = 32;
const BUILD_WORKGROUP = 64;

const FogProbe = d
  .struct({ position: d.vec3f, ground: d.u32, normal: d.vec3f, pad: d.u32 })
  .$name("FogProbe");

const words = (n: number) => d.arrayOf(d.u32, n);
const buildLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["compute"] },
  eyes: { storage: (n: number) => d.arrayOf(FogEye, n), access: "readonly" },
  heights: { storage: (n: number) => d.arrayOf(d.u32, n), access: "readonly" },
  foliage: { storage: (n: number) => d.arrayOf(d.vec4f, n), access: "readonly" },
  occluders: { storage: (n: number) => d.arrayOf(FogBox, n), access: "readonly" },
  // Per rebuilt eye: [eye index, sector table offset, sector count], followed
  // by sector [candidate offset, count] pairs and sorted occluder indices.
  rebuild: { storage: words, access: "readonly" },
  terrain: { storage: words, access: "mutable" },
  maps: { storage: words, access: "mutable" },
});
const cullLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["compute"] },
  eyes: { storage: (n: number) => d.arrayOf(FogEye, n), access: "readonly" },
  depth: { texture: d.textureDepthMultisampled2d(), visibility: ["compute"] },
  lists: { storage: words, access: "mutable" },
  counts: { storage: (n: number) => d.arrayOf(d.atomic(d.u32), n), access: "mutable" },
});
const probeLayout = tgpu.bindGroupLayout({
  points: { storage: (n: number) => d.arrayOf(FogProbe, n), access: "readonly" },
  out: { storage: words, access: "mutable" },
});
const wholeLayout = tgpu.bindGroupLayout({
  boxes: { storage: (n: number) => d.arrayOf(FogBox, n), access: "readonly" },
  /** Reachable structures' step and count: (whole_step_m, count, 0, 0). */
  whole: { uniform: d.vec4f },
  flags: { storage: words, access: "mutable" },
  items: { storage: words, access: "readonly" },
});
const shapeLayout = tgpu.bindGroupLayout({
  rows: { storage: (n: number) => d.arrayOf(d.vec4f, n), access: "readonly" },
  out: { storage: (n: number) => d.arrayOf(d.f32, n), access: "mutable" },
});

const fogSample = terrainSample(
  tgpu
    .fn(
      [d.u32],
      d.u32,
    )(/* wgsl */ `(at: u32) -> u32 {
  return buildLayout.$.heights[at];
}`)
    .$uses({ buildLayout }),
);

/** The simulation's ground height: its triangle rule over the grid. */
const fogHeight = tgpu
  .fn(
    [d.vec2f],
    d.f32,
  )(/* wgsl */ `(q: vec2f) -> f32 {
  let P = buildLayout.$.params;
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
}`)
  .$uses({ buildLayout, triangleRuleHeight, fogSample });

/** The foliage over a point: its cell's canopy top above the ground and
 *  foliage depth per metre (0, 0 in the open). */
const fogFoliage = tgpu
  .fn(
    [d.vec2f],
    d.vec2f,
  )(/* wgsl */ `(q: vec2f) -> vec2f {
  let P = buildLayout.$.params;
  if (P.foliageNx == 0u || q.x < 0.0 || q.y < 0.0) { return vec2f(0.0); }
  let i = u32(q.x / P.foliageCellM);
  let j = u32(q.y / P.foliageCellM);
  if (i >= P.foliageNx || j >= P.foliageNy) { return vec2f(0.0); }
  let key=j*P.foliageNx+i;
  var lo=0u;
  var hi=arrayLength(&buildLayout.$.foliage);
  while(lo<hi){
    let mid=(lo+hi)/2u;
    let row=buildLayout.$.foliage[mid];
    let at=u32(row.y)*P.foliageNx+u32(row.x);
    if(at==key){return row.zw;}
    if(at<key){lo=mid+1u;}else{hi=mid;}
  }
  return vec2f(0.0);
}`)
  .$uses({ buildLayout });

/** A map word: horizon slope (f16), jump position in the bin (u8), foliage
 *  depth in steps of `FOLIAGE_STEP` (u8). */
const fogPack = tgpu.fn(
  [d.f32, d.f32, d.f32],
  d.u32,
)(/* wgsl */ `(horizon: f32, jump: f32, foliage: f32) -> u32 {
  let h = pack2x16float(vec2f(clamp(horizon, -6e4, 6e4), 0.0)) & 0xffffu;
  return h | (u32(round(clamp(jump, 0.0, 1.0) * 255.0)) << 16u) | (min(u32(round(foliage / ${FOLIAGE_STEP})), 255u) << 24u);
}`);

const PI = "3.14159265";
const TAU = "6.28318531";

/** Terrain and foliage marched along each coarse ray: the steepest ground
 *  slope so far, where in each bin it last jumped, and the foliage the sight
 *  line crossed below the canopy (the sweep's rule, continuous). */
const terrainMarch = tgpu
  .computeFn({
    in: { gid: d.builtin.globalInvocationId },
    workgroupSize: [BUILD_WORKGROUP],
  })(/* wgsl */ `{
  let P = buildLayout.$.params;
  let AZT = P.terrainAzimuthBins;
  let R = P.radialBins;
  let r = gid.x / AZT;
  let ai = gid.x % AZT;
  if (r >= P.rebuildCount) { return; }
  let e = buildLayout.$.eyes[buildLayout.$.rebuild[r * 3u]];
  let theta = (f32(ai) + 0.5) / f32(AZT) * ${TAU} - ${PI};
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
      buildLayout.$.terrain[base + k] = fogPack(v, f, fol);
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
}`)
  .$uses({ buildLayout, fogBinEdge, fogPack, fogHeight, fogFoliage });

/** Full-azimuth merge: each ray meets its eye’s nearby occluders analytically
 *  (entry distance and top slope; below the eye, the far edge), then takes
 *  the running maximum with the terrain map interpolated between its rays. */
function mergeFn(radialBins: number) {
  return tgpu
    .computeFn({
      in: { gid: d.builtin.globalInvocationId },
      workgroupSize: [BUILD_WORKGROUP],
    })(/* wgsl */ `{
  let P = buildLayout.$.params;
  let AZ = P.azimuthBins;
  let AZT = P.terrainAzimuthBins;
  let R = P.radialBins;
  let r = gid.x / AZ;
  let ai = gid.x % AZ;
  if (r >= P.rebuildCount) { return; }
  let e = buildLayout.$.eyes[buildLayout.$.rebuild[r * 3u]];
  let theta = (f32(ai) + 0.5) / f32(AZ) * ${TAU} - ${PI};
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.reach / P.firstBinM);
  // A ray squeezing past boxes on both sides with less than the least sight
  // gap between them is blocked where it passes the second (as the
  // simulation's \`squeezed\`; here the closest box on each side counts).
  let gap = P.minSightGapM;
  var left = vec3f(1e9, 0.0, 0.0);
  var right = vec3f(1e9, 0.0, 0.0);
  var pm: array<f32, ${radialBins}>;
  var pt: array<f32, ${radialBins}>;
  for (var k = 0u; k < R; k++) { pm[k] = -1e4; pt[k] = 0.0; }
  let sectors = buildLayout.$.rebuild[r * 3u + 2u];
  let sector = min(u32((f32(ai) + 0.5) / f32(AZ) * f32(sectors)), sectors - 1u);
  let row = buildLayout.$.rebuild[r * 3u + 1u] + sector * 2u;
  let start = buildLayout.$.rebuild[row];
  let count = buildLayout.$.rebuild[row + 1u];
  for (var i = 0u; i < count; i++) {
    let b = buildLayout.$.occluders[buildLayout.$.rebuild[start + i]];
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
  let a = (theta + ${PI}) / ${TAU} * f32(AZT) - 0.5;
  let af = floor(a);
  let fa = a - af;
  let j0 = u32(i32(af) + i32(AZT)) % AZT;
  let j1 = (j0 + 1u) % AZT;
  let tb = e.slot * AZT * R;
  let base = (e.slot * AZ + ai) * R;
  var hk = -1e4;
  for (var k = 0u; k < R; k++) {
    let c0 = fogUnpackBuild(buildLayout.$.terrain[tb + j0 * R + k]);
    let c1 = fogUnpackBuild(buildLayout.$.terrain[tb + j1 * R + k]);
    var f = select(c0.y, c1.y, fa > 0.5);
    var v = max(hk, mix(c0.x, c1.x, fa));
    if (pm[k] > v) {
      let lo = fogBinEdge(i32(k) - 1, P.firstBinM, lnr, R);
      let hi = fogBinEdge(i32(k), P.firstBinM, lnr, R);
      f = clamp((pt[k] - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
      v = pm[k];
    }
    hk = v;
    buildLayout.$.maps[base + k] = fogPack(v, f, mix(c0.z, c1.z, fa));
  }
}`)
    .$uses({ buildLayout, fogBinEdge, fogPack, fogUnpackBuild });
}
/** `fogUnpack` for the build's own (mutable) words. */
const fogUnpackBuild = tgpu.fn(
  [d.u32],
  d.vec3f,
)(/* wgsl */ `(w: u32) -> vec3f {
  return vec3f(unpack2x16float(w & 0xffffu).x, f32((w >> 16u) & 0xffu) / 255.0, f32(w >> 24u) * ${FOLIAGE_STEP});
}`);

const cullLo = tgpu.workgroupVar(d.arrayOf(d.atomic(d.i32), 2));
const cullHi = tgpu.workgroupVar(d.arrayOf(d.atomic(d.i32), 2));
const cullHit = tgpu.workgroupVar(d.atomic(d.u32));

/** One workgroup per screen tile: bound the world xy the tile shows (from the
 *  depth prepass, sample 0, plus the tallest translucent surface above that
 *  ground along each pixel's ray) and list the eyes whose reach touches it. */
function cullFn(tilePx: number) {
  return tgpu
    .computeFn({
      in: {
        wg: d.builtin.workgroupId,
        li: d.builtin.localInvocationIndex,
        gid: d.builtin.globalInvocationId,
      },
      workgroupSize: [tilePx, tilePx],
    })(/* wgsl */ `{
  let P = cullLayout.$.params;
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
    let z = textureLoad(cullLayout.$.depth, vec2i(gid.xy), 0);
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
  for (var e = li; e < P.eyeCount; e += ${tilePx * tilePx}u) {
    let eye = cullLayout.$.eyes[e];
    let near = clamp(eye.position.xy, lo, hi);
    if (distance(near, eye.position.xy) <= eye.reach) {
      let s = atomicAdd(&cullLayout.$.counts[tile], 1u);
      if (s < P.tileEyesMax) { cullLayout.$.lists[tile * P.tileEyesMax + s] = e; }
    }
  }
}`)
    .$uses({ cullLayout, cullLo, cullHi, cullHit });
}

/** Fog at given points against every eye: 1 seen, 0 unseen. */
const probeFn = tgpu
  .computeFn({
    in: { gid: d.builtin.globalInvocationId },
    workgroupSize: [BUILD_WORKGROUP],
  })(/* wgsl */ `{
  let P = fogLayout.$.params;
  if (gid.x >= P.probeCount) { return; }
  let q = probeLayout.$.points[gid.x];
  var seen = 0u;
  for (var e = 0u; e < P.eyeCount; e++) {
    if (fogSeenSurface(e, q.position, q.normal, q.ground == 1u)) { seen = 1u; break; }
  }
  probeLayout.$.out[gid.x] = seen;
}`)
  .$uses({ fogLayout, probeLayout, fogSeenSurface });

const WHOLE_WORKGROUP = 64;
const wholeFound = tgpu.workgroupVar(d.atomic(d.u32));

/** One workgroup per reachable structure that takes fog whole: its four walls and its
 *  roof sampled on a grid `step` apart (walls from just above its base to
 *  just below its top, each probing out along its normal; the roof facing
 *  up, under the roof rule), tested against every eye that can reach it,
 *  until one is seen. Its flag: 1 seen, 0 not. */
const wholeFn = tgpu
  .computeFn({
    in: { wg: d.builtin.workgroupId, li: d.builtin.localInvocationIndex },
    workgroupSize: [WHOLE_WORKGROUP],
  })(/* wgsl */ `{
  let W = wholeLayout.$.whole;
  let item = wg.x + wg.y * 65535u;
  if (item >= u32(W.y)) { return; }
  let index = wholeLayout.$.items[item * 3u];
  let eyesStart = wholeLayout.$.items[item * 3u + 1u];
  let eyesCount = wholeLayout.$.items[item * 3u + 2u];
  if (li == 0u) { atomicStore(&wholeFound, 0u); }
  workgroupBarrier();
  let b = wholeLayout.$.boxes[index];
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
  for (var s = li; s < total; s += ${WHOLE_WORKGROUP}u) {
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
      let e = wholeLayout.$.items[eyesStart + ei];
      let eye = fogLayout.$.eyes[e];
      if (distance(eye.position.xy, b.center) - reach > eye.reach) { continue; }
      if (fogSeenSurface(e, p, n, false)) { atomicStore(&wholeFound, 1u); break; }
    }
  }
  workgroupBarrier();
  if (li == 0u) { wholeLayout.$.flags[index] = atomicLoad(&wholeFound); }
}`)
  .$uses({ fogLayout, wholeLayout, wholeFound, fogSeenSurface });

/** The WGSL sight shape at `[front, side, rear, off]` rows. */
const shapeFn = tgpu
  .computeFn({
    in: { gid: d.builtin.globalInvocationId },
    workgroupSize: [BUILD_WORKGROUP],
  })(/* wgsl */ `{
  if (gid.x >= arrayLength(&shapeLayout.$.rows)) { return; }
  let r = shapeLayout.$.rows[gid.x];
  shapeLayout.$.out[gid.x] = fogShape(r.x, r.y, r.z, r.w);
}`)
  .$uses({ shapeLayout, fogShape });

/** A point to probe: ground (at the target height above it) or a face. */
export interface FogProbeInput {
  position: readonly [number, number, number];
  /** Zero for ground. */
  normal?: readonly [number, number, number];
}

/** What fog is drawn from: the scene's presentation, which decides it. */
export type FogSource = Pick<
  Presentation,
  "prepare_fog" | "resend_fog" | "rebuild_fog" | "fog_stats" | "fog_occluders"
>;

/** The pipelines' text constants (`presentation.fog_geometry`): the cull's
 *  screen tile and the merge's radial bins. */
export interface FogShaders {
  tile_px: number;
  radial_bins: number;
}

/** A structure that takes fog whole, as the lab reads it back. */
export interface FogOccluder {
  x: number;
  y: number;
  yaw: number;
  hx: number;
  hy: number;
  base: number;
  top: number;
}

export interface FogVisibilityStats {
  /** Whether fog is drawn. */
  enabled: boolean;
  eyes: number;
  /** Map slots allocated (eyes that fit before the maps grow). */
  slots: number;
  occluders: number;
  /** Eye maps rebuilt last frame, and moved eyes still waiting their turn. */
  rebuilt: number;
  pending: number;
  /** Every rebuild since the maps were allocated. */
  rebuiltTotal: number;
  mapBytes: number;
  terrainMapBytes: number;
  tileListBytes: number;
  tiles: [number, number];
}

/** The tile lists for one frame size, made at its first fogged frame. */
export interface FogTiles {
  scope: GpuRegistry;
  tilesX: number;
  tilesY: number;
  lists: GPUBuffer | null;
  counts: GPUBuffer | null;
  depthView: GPUTextureView;
  /** The frame's ground paint, which the painted layers read. */
  paintView: GPUTextureView;
}

/** `Presentation.fog_occluders`' seven numbers a box, as boxes. */
export function occluderBoxes(n: Float64Array): FogOccluder[] {
  const boxes: FogOccluder[] = [];
  for (let i = 0; i < n.length; i += 7) {
    const [x, y, yaw, hx, hy, base, top] = n.subarray(i, i + 7);
    boxes.push({ x, y, yaw, hx, hy, base, top });
  }
  return boxes;
}

function probeRecords(points: readonly FogProbeInput[]): ArrayBuffer {
  const bytes = new ArrayBuffer(Math.max(1, points.length) * PROBE_BYTES);
  const f = new Float32Array(bytes);
  const u = new Uint32Array(bytes);
  points.forEach((p, i) => {
    const o = (i * PROBE_BYTES) / WORD;
    f.set(p.position, o);
    u[o + 3] = p.normal ? 0 : 1;
    f.set(p.normal ?? [0, 0, 0], o + 4);
  });
  return bytes;
}

/** Ground with no height pages: flat at zero (a scene's bare ground). */
const FLAT: TerrainGrid = {
  nx: 2,
  ny: 2,
  spacing: 1,
  pageSize: 1,
  minHeight: 0,
  pageIds: new Uint32Array(0),
  heights: new Float32Array(0),
};

export async function createFogVisibility(
  root: Root,
  registry: GpuRegistry,
  shaders: FogShaders,
  terrainHeights: TerrainHeights,
) {
  const device = registry.device;
  const pipelines = {
    terrain: root.createComputePipeline({ compute: terrainMarch }).$name("fogTerrain"),
    merge: root.createComputePipeline({ compute: mergeFn(shaders.radial_bins) }).$name("fogMerge"),
    cull: root.createComputePipeline({ compute: cullFn(shaders.tile_px) }).$name("fogCull"),
    probe: root.createComputePipeline({ compute: probeFn }).$name("fogProbe"),
    shape: root.createComputePipeline({ compute: shapeFn }).$name("fogShape"),
    whole: root.createComputePipeline({ compute: wholeFn }).$name("fogWhole"),
  };
  await Promise.all(Object.values(pipelines).map((p) => p.initAsync()));

  const params = registry.own(root.createBuffer(FogParams).$usage("uniform"));
  const layerOf = (ground: number, seen = 0, painted = 0) => {
    const layer = registry.own(root.createBuffer(FogLayer).$usage("uniform"));
    layer.write({ ground, seen, painted, pad: 0 });
    return layer;
  };
  // Ground versus bodies: the one place that says which
  // layers the ground paint lies on. `paintedGround` is the terrain, its
  // grass and the backdrop; `paintedFaces` the surfaces movers stand on that
  // fog takes face by face: the water, and a prop whose body stops no mover
  // (a bridge deck, rubble); `ground` is what lies on the ground but is a
  // body (the fallen); faces and units are bodies.
  const layers = {
    paintedGround: layerOf(1, 0, 1),
    paintedFaces: layerOf(0, 0, 1),
    ground: layerOf(1),
    faces: layerOf(0),
    units: layerOf(0, 1),
  };
  const paintStyle = registry.own(root.createBuffer(GroundPaintStyle).$usage("uniform"));
  const noPaint = registry.texture({
    label: "fog-no-paint",
    size: [1, 1],
    format: "rgba8unorm",
    usage: GPUTextureUsage.TEXTURE_BINDING,
  });
  const storage = (label: string, bytes: number) =>
    device.createBuffer({ label, size: Math.max(16, bytes), usage: STORAGE | COPY_DST });
  const wholeTexture = (rows: number) =>
    device.createTexture({
      label: "fog-wholes",
      size: [WHOLE_TEXTURE_WIDTH, rows],
      format: "r32uint",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });
  /** A flag word per structure, `rows` texture rows of them. */
  const wholeFlagBuffer = (rows: number) =>
    device.createBuffer({
      label: "fog-whole-flags",
      size: rows * WHOLE_TEXTURE_WIDTH * WORD,
      usage: STORAGE | COPY_SRC | COPY_DST,
    });
  const wholeUniform = registry.own(root.createBuffer(d.vec4f).$usage("uniform"));

  const slot = <T extends { destroy(): void }>(initial: T): GpuSlot<T> => {
    const s = registry.slot<T>();
    s.set(initial);
    return s;
  };
  const buffers = {
    // A storage binding holds at least one of its records.
    eyes: slot(storage("fog-eyes", FOG_EYE_WORDS * WORD)),
    maps: slot(storage("fog-maps", WORD)),
    terrain: slot(storage("fog-terrain-maps", WORD)),
    foliage: slot(storage("fog-foliage", 16)),
    occluders: slot(storage("fog-occluders", FOG_BOX_WORDS * WORD)),
    rebuild: slot(storage("fog-rebuild", WORD)),
    wholes: slot(wholeTexture(1)),
    wholeFlags: slot(wholeFlagBuffer(1)),
    wholeItems: slot(storage("fog-whole-items", WORD)),
  };
  /** Each buffer's size; a write that outgrows one replaces it. */
  const sizes = new Map<GpuSlot<GPUBuffer>, number>();
  /** Bind groups are made again when a buffer they bind is replaced. */
  let generation = 0;
  /** Writes `words` into `target`, growing it (by doubling) to fit; an
   *  `exact` buffer is the words' size, for a shader that reads its length. */
  const write = (
    target: GpuSlot<GPUBuffer>,
    label: string,
    words: Uint32Array | Float32Array,
    exact = false,
  ) => {
    const size = sizes.get(target) ?? 0;
    if (exact ? words.byteLength !== size : words.byteLength > size) {
      const bytes = exact ? words.byteLength : Math.max(words.byteLength, 2 * size);
      target.set(storage(label, bytes));
      sizes.set(target, bytes);
      generation++;
    }
    device.queue.writeBuffer(target.current!, 0, words);
  };

  let source: FogSource | null = null;
  let grid: TerrainGrid = FLAT;
  let active = false;
  let mapWords = [0, 0];
  let flagRows = 1;
  let tiles: FogTiles | null = null;
  let groupsFor: { generation: number; tiles: FogTiles | null; value: FogGroups } | null = null;

  /** Upload what the presentation decided; returns the rebuild's
   *  workgroups, if any eye rebuilds. */
  const upload = (up: FogUpload): Uint32Array | null => {
    active = up.active();
    const [maps, terrainMaps] = up.map_words();
    if (maps !== mapWords[0] || terrainMaps !== mapWords[1]) {
      // A new size rebuilds every map: the old ones are not kept.
      buffers.maps.set(storage("fog-maps", maps * WORD));
      buffers.terrain.set(storage("fog-terrain-maps", terrainMaps * WORD));
      mapWords = [maps, terrainMaps];
      generation++;
    }
    const foliage = up.foliage();
    // The foliage lookup searches the whole buffer (`arrayLength`).
    if (foliage) write(buffers.foliage, "fog-foliage", foliage, true);
    const occluders = up.occluders();
    if (occluders) write(buffers.occluders, "fog-occluders", occluders);
    const wholes = up.wholes();
    if (wholes) {
      const rows = wholes.length / WHOLE_TEXTURE_WIDTH;
      buffers.wholes.set(wholeTexture(rows));
      device.queue.writeTexture(
        { texture: buffers.wholes.current! },
        wholes,
        { bytesPerRow: WHOLE_TEXTURE_WIDTH * WORD },
        [WHOLE_TEXTURE_WIDTH, rows],
      );
      generation++;
    }
    const eyes = up.eyes();
    if (eyes.length) write(buffers.eyes, "fog-eyes", eyes);
    const rebuild = up.rebuild();
    if (rebuild.length) write(buffers.rebuild, "fog-rebuild", rebuild);
    device.queue.writeBuffer(root.unwrap(params), 0, up.params());
    return rebuild.length ? up.rebuild_dispatch() : null;
  };

  const buildGroup = () =>
    root.createBindGroup(buildLayout, {
      params,
      eyes: buffers.eyes.current!,
      heights: terrainHeights.forGrid(grid),
      foliage: buffers.foliage.current!,
      occluders: buffers.occluders.current!,
      rebuild: buffers.rebuild.current!,
      terrain: buffers.terrain.current!,
      maps: buffers.maps.current!,
    });
  let build: { generation: number; group: ReturnType<typeof buildGroup> } | null = null;
  /** The rebuilt eyes' terrain march and merge. */
  const rebuildMaps = (encoder: GPUCommandEncoder, dispatch: Uint32Array) => {
    if (!build || build.generation !== generation) build = { generation, group: buildGroup() };
    pipelines.terrain.with(build.group).with(encoder).dispatchWorkgroups(dispatch[0]);
    pipelines.merge.with(build.group).with(encoder).dispatchWorkgroups(dispatch[1]);
  };

  interface FogGroups {
    paintedGround: ReturnType<typeof fragmentGroup>;
    paintedFaces: ReturnType<typeof fragmentGroup>;
    ground: ReturnType<typeof fragmentGroup>;
    faces: ReturnType<typeof fragmentGroup>;
    units: ReturnType<typeof fragmentGroup>;
    cull: ReturnType<typeof cullGroup> | null;
  }
  const fragmentGroup = (layer: typeof layers.ground, t: FogTiles | null) =>
    root.createBindGroup(fogLayout, {
      params,
      layer,
      eyes: buffers.eyes.current!,
      maps: buffers.maps.current!,
      lists: t?.lists ?? buffers.rebuild.current!,
      counts: t?.counts ?? buffers.rebuild.current!,
      paintStyle,
      paint: t?.paintView ?? noPaint.createView(),
      wholes: buffers.wholes.current!.createView(),
    });
  const cullGroup = (t: FogTiles) =>
    root.createBindGroup(cullLayout, {
      params,
      eyes: buffers.eyes.current!,
      depth: t.depthView,
      lists: t.lists!,
      counts: t.counts!,
    });
  const groups = (): FogGroups => {
    if (!groupsFor || groupsFor.generation !== generation || groupsFor.tiles !== tiles) {
      groupsFor = {
        generation,
        tiles,
        value: {
          paintedGround: fragmentGroup(layers.paintedGround, tiles),
          paintedFaces: fragmentGroup(layers.paintedFaces, tiles),
          ground: fragmentGroup(layers.ground, tiles),
          faces: fragmentGroup(layers.faces, tiles),
          units: fragmentGroup(layers.units, tiles),
          cull: tiles?.lists ? cullGroup(tiles) : null,
        },
      };
    }
    return groupsFor.value;
  };

  /** The structures pass: clear the flags, flag each row's structure, copy
   *  the flags into the `wholes` texture's flag rows. */
  const encodeWholes = (encoder: GPUCommandEncoder, up: FogUpload, faces: FogGroups["faces"]) => {
    const [count, x, y, flagsRow, rows] = up.whole();
    if (rows !== flagRows) {
      buffers.wholeFlags.set(wholeFlagBuffer(rows));
      flagRows = rows;
    }
    // An eye leaving a neighbourhood must clear its previously seen flags.
    encoder.clearBuffer(buffers.wholeFlags.current!);
    if (count) {
      write(buffers.wholeItems, "fog-whole-items", up.whole_items());
      wholeUniform.write(
        d.vec4f(...(up.whole_uniform() as unknown as [number, number, number, number])),
      );
      pipelines.whole
        .with(faces)
        .with(
          root.createBindGroup(wholeLayout, {
            boxes: buffers.occluders.current!,
            whole: wholeUniform,
            flags: buffers.wholeFlags.current!,
            items: buffers.wholeItems.current!,
          }),
        )
        .with(encoder)
        .dispatchWorkgroups(x, y);
    }
    encoder.copyBufferToTexture(
      { buffer: buffers.wholeFlags.current!, bytesPerRow: WHOLE_TEXTURE_WIDTH * WORD },
      { texture: buffers.wholes.current!, origin: [0, flagsRow] },
      [WHOLE_TEXTURE_WIDTH, rows],
    );
  };

  /** The frame size's tile lists, made for the cull's tiles. */
  const ensureTiles = (t: FogTiles, [tilesX, tilesY, eyesMax]: Uint32Array) => {
    if (t.lists && t.tilesX === tilesX && t.tilesY === tilesY) return;
    const n = tilesX * tilesY;
    t.lists = t.scope.buffer({ label: "fog-tile-lists", size: n * eyesMax * WORD, usage: STORAGE });
    t.counts = t.scope.buffer({
      label: "fog-tile-counts",
      size: n * WORD,
      usage: STORAGE | COPY_DST | COPY_SRC,
    });
    t.tilesX = tilesX;
    t.tilesY = tilesY;
    generation++;
  };

  const readback = async (from: GPUBuffer, bytes: number): Promise<ArrayBuffer> => {
    const read = device.createBuffer({ size: bytes, usage: MAP_READ | COPY_DST });
    try {
      const encoder = device.createCommandEncoder({ label: "fog-readback" });
      encoder.copyBufferToBuffer(from, 0, read, 0, bytes);
      device.queue.submit([encoder.finish()]);
      await read.mapAsync(1);
      return read.getMappedRange().slice(0);
    } finally {
      read.destroy();
    }
  };
  const outBuffer = (bytes: number) =>
    device.createBuffer({ size: Math.max(16, bytes), usage: STORAGE | COPY_SRC });

  return {
    /** What fog is drawn from: the scene's presentation over the ground
     *  `heights` (none: flat), or none (everything seen). */
    setSource(next: FogSource | null, heights: TerrainGrid | null) {
      grid = heights ?? FLAT;
      generation++;
      if (next === source) return;
      source = next;
      // This renderer holds nothing of it yet.
      source?.resend_fog();
    },
    /** The ground paint's look on the painted layers. */
    setPaintStyle(style: {
      albedo: number;
      emissive: number;
      fog_keep: number;
      grass_glow: number;
      grass_falloff_m: number;
      saturation: number;
    }) {
      paintStyle.write({
        albedo: style.albedo,
        emissive: style.emissive,
        fogKeep: style.fog_keep,
        grassGlow: style.grass_glow,
        grassFalloff: style.grass_falloff_m,
        saturation: style.saturation,
        pad1: 0,
        pad2: 0,
      });
    },
    /** The tile lists for a frame size, owned by that size's scope. */
    sized(scope: GpuRegistry, depth: GPUTexture, paint: GPUTexture): FogTiles {
      return {
        scope,
        tilesX: 0,
        tilesY: 0,
        lists: null,
        counts: null,
        depthView: depth.createView(),
        paintView: paint.createView(),
      };
    },
    /** This frame's rebuilds, structures and tile cull over the prepass
     *  depth. `camera` is the frame's camera record. */
    encode(
      encoder: GPUCommandEncoder,
      frameTiles: FogTiles,
      camera: Float32Array,
      width: number,
      height: number,
    ) {
      tiles = frameTiles;
      const up = source?.prepare_fog(camera, width, height, 0);
      if (!up) {
        if (active)
          device.queue.writeBuffer(root.unwrap(params), 0, new Uint32Array(FOG_PARAMS_WORDS));
        active = false;
        return;
      }
      try {
        const dispatch = upload(up);
        if (dispatch) rebuildMaps(encoder, dispatch);
        if (!active) return;
        ensureTiles(frameTiles, up.tiles());
        const { cull, faces } = groups();
        if (up.whole().length) encodeWholes(encoder, up, faces);
        encoder.clearBuffer(frameTiles.counts!);
        pipelines.cull
          .with(cull!)
          .with(encoder)
          .dispatchWorkgroups(frameTiles.tilesX, frameTiles.tilesY);
      } finally {
        up.free();
      }
    },
    /** The fragment bind groups: ground, faces standing on it, and units
     *  (never fogged). */
    groups,
    stats(): FogVisibilityStats {
      const rust = source?.fog_stats();
      return fogStats(
        rust ? (JSON.parse(rust) as Omit<FogVisibilityStats, "tileListBytes" | "tiles">) : null,
        tiles,
        active,
      );
    },
    /** Fog at `points` against every eye, after building any maps still due. */
    async probe(points: readonly FogProbeInput[]): Promise<Uint8Array> {
      const up = source?.prepare_fog(undefined, 0, 0, points.length);
      if (!up?.active()) {
        up?.free();
        return new Uint8Array(points.length).fill(1);
      }
      const records = probeRecords(points);
      const input = storage("fog-probe-points", records.byteLength);
      const out = outBuffer(points.length * WORD);
      try {
        device.queue.writeBuffer(input, 0, records);
        const encoder = device.createCommandEncoder({ label: "fog-probe" });
        // Uploading can replace the buffers the groups bind: bind after it.
        const dispatch = upload(up);
        if (dispatch) rebuildMaps(encoder, dispatch);
        pipelines.probe
          .with(groups().faces)
          .with(root.createBindGroup(probeLayout, { points: input, out }))
          .with(encoder)
          .dispatchWorkgroups(Math.max(1, Math.ceil(points.length / BUILD_WORKGROUP)));
        device.queue.submit([encoder.finish()]);
        return Uint8Array.from(new Uint32Array(await readback(out, points.length * WORD)));
      } finally {
        up.free();
        input.destroy();
        out.destroy();
      }
    },
    /** Rebuild every eye's maps on the next frame, as if all had moved (a
     *  cost probe). */
    rebuildAll() {
      source?.rebuild_fog();
    },
    /** The per-tile eye counts of the last frame (a debug readback). */
    async tileCounts(): Promise<Uint32Array> {
      if (!tiles?.counts) return new Uint32Array(0);
      const bytes = tiles.tilesX * tiles.tilesY * WORD;
      return new Uint32Array(await readback(tiles.counts, bytes));
    },
    /** The structures that take fog whole, and whether each was flagged
     *  seen by the last frame (a debug readback). */
    async wholes(): Promise<{ boxes: readonly FogOccluder[]; seen: Uint8Array }> {
      const boxes = occluderBoxes(source?.fog_occluders() ?? new Float64Array(0));
      if (!boxes.length) return { boxes, seen: new Uint8Array(0) };
      const flags = new Uint32Array(
        await readback(buffers.wholeFlags.current!, boxes.length * WORD),
      );
      return { boxes, seen: Uint8Array.from(flags) };
    },
    /** The WGSL sight shape at `[front, side, rear, off]` rows. */
    async probeShape(rows: Float32Array): Promise<Float32Array> {
      const n = rows.length / 4;
      const input = storage("fog-shape-rows", rows.byteLength);
      const out = outBuffer(n * WORD);
      try {
        device.queue.writeBuffer(input, 0, rows);
        const encoder = device.createCommandEncoder({ label: "fog-shape-probe" });
        pipelines.shape
          .with(root.createBindGroup(shapeLayout, { rows: input, out }))
          .with(encoder)
          .dispatchWorkgroups(Math.max(1, Math.ceil(n / BUILD_WORKGROUP)));
        device.queue.submit([encoder.finish()]);
        return new Float32Array(await readback(out, n * WORD));
      } finally {
        input.destroy();
        out.destroy();
      }
    },
  };
}

function fogStats(
  source: Omit<FogVisibilityStats, "tileListBytes" | "tiles"> | null,
  tiles: FogTiles | null,
  active: boolean,
): FogVisibilityStats {
  const s = source ?? {
    enabled: false,
    eyes: 0,
    slots: 0,
    occluders: 0,
    rebuilt: 0,
    pending: 0,
    rebuiltTotal: 0,
    mapBytes: 0,
    terrainMapBytes: 0,
  };
  const lists = tiles?.lists;
  const eyesMax = lists ? lists.size / WORD / (tiles.tilesX * tiles.tilesY) : 0;
  return {
    ...s,
    enabled: s.enabled && active,
    tileListBytes: lists ? tiles.tilesX * tiles.tilesY * (eyesMax + 1) * WORD : 0,
    tiles: lists ? [tiles.tilesX, tiles.tilesY] : [0, 0],
  };
}
export type FogVisibility = Awaited<ReturnType<typeof createFogVisibility>>;
/** The lab's hold on the sight lights: debug readbacks, never in a frame. */
export type FogProbes = Pick<
  FogVisibility,
  "probe" | "probeShape" | "tileCounts" | "rebuildAll" | "wholes"
>;
