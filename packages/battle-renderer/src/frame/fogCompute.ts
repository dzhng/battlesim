// Fog of war's GPU passes, as `fogVisibility.ts` runs them: each eye's
// horizon maps built from the ground and the foliage (`terrainMarch`, then
// `mergeFn`'s full-azimuth merge of its nearby occluders), the per-tile cull
// of the eyes over the depth prepass (`cullFn`), the structures that take fog
// whole (`wholeFn`), and the lab's probes of the lookup and the sight shape.
import { tgpu, d } from "typegpu";
import { triangleRuleHeight } from "./triangleRule";
import { terrainSample } from "./terrainHeights";
import { FogBox, FogEye, FogParams, FOLIAGE_STEP } from "../records.gen";
import { fogBinEdge, fogLayout, fogSeenSurface, fogShape } from "./fogTerm";

/** The build's workgroup: eyes' rays a dispatch marches. */
export const BUILD_WORKGROUP = 64;

const FogProbe = d
  .struct({ position: d.vec3f, ground: d.u32, normal: d.vec3f, pad: d.u32 })
  .$name("FogProbe");

export const words = (n: number) => d.arrayOf(d.u32, n);
export const buildLayout = tgpu.bindGroupLayout({
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
export const cullLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["compute"] },
  eyes: { storage: (n: number) => d.arrayOf(FogEye, n), access: "readonly" },
  depth: { texture: d.textureDepthMultisampled2d(), visibility: ["compute"] },
  lists: { storage: words, access: "mutable" },
  counts: { storage: (n: number) => d.arrayOf(d.atomic(d.u32), n), access: "mutable" },
});
export const probeLayout = tgpu.bindGroupLayout({
  points: { storage: (n: number) => d.arrayOf(FogProbe, n), access: "readonly" },
  out: { storage: words, access: "mutable" },
});
export const wholeLayout = tgpu.bindGroupLayout({
  boxes: { storage: (n: number) => d.arrayOf(FogBox, n), access: "readonly" },
  /** Reachable structures' step and count: (whole_step_m, count, 0, 0). */
  whole: { uniform: d.vec4f },
  flags: { storage: words, access: "mutable" },
  items: { storage: words, access: "readonly" },
});
export const shapeLayout = tgpu.bindGroupLayout({
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
export const terrainMarch = tgpu
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
export function mergeFn(radialBins: number) {
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
export function cullFn(tilePx: number) {
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
export const probeFn = tgpu
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
export const wholeFn = tgpu
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
export const shapeFn = tgpu
  .computeFn({
    in: { gid: d.builtin.globalInvocationId },
    workgroupSize: [BUILD_WORKGROUP],
  })(/* wgsl */ `{
  if (gid.x >= arrayLength(&shapeLayout.$.rows)) { return; }
  let r = shapeLayout.$.rows[gid.x];
  shapeLayout.$.out[gid.x] = fogShape(r.x, r.y, r.z, r.w);
}`)
  .$uses({ shapeLayout, fogShape });
