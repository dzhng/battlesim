// FogVisibility: the GPU owner of sight-light fog (spike 02's technique).
// It consumes `FogInput` (the static world, each own eye at the published
// tick, the occluders the side knows) and produces what `fogTerm` samples:
//
// - one horizon map per eye, `azimuth_bins × radial_bins` words: a coarse
//   terrain march at `terrain_azimuth_bins` rays, then a merge at full
//   azimuth that intersects every known occluder exactly. Only eyes that moved
//   are rebuilt, `rebuild_eyes_per_frame` at a time (new eyes at once); the
//   sight shape is applied per fragment, so a new bearing never rebuilds;
// - per frame, a cull over the depth prepass: each `tile_px` screen tile
//   bounds the ground it shows and lists the eyes whose reach touches it;
// - the structures that take fog whole (every known occluder): after the
//   maps change, or the eyes (a publication), one workgroup per occluder
//   samples its walls and roof every `whole_step_m` against every eye and
//   flags it seen if any sample is; the flags join the occluders' boxes and
//   a lookup grid in the `wholes` texture `fogWholeSeen` reads.
//
// Every buffer is owned by the frame's registry; the tile lists live in the
// size-dependent scope beside the targets. `probe` evaluates the same lookup
// at given points against every eye (the agreement metric and the lab's
// tests), and `probeShape` / `probeWith` run the WGSL against oracle vectors.
// Probes read back: lab only, never in the frame.
import { tgpu, d } from "typegpu";
import type { GpuRegistry, GpuSlot } from "./registry";
import { triangleRuleHeight } from "./triangleRule";
import { terrainSample, type TerrainHeights } from "./terrainHeights";
import {
  eyeReach,
  FOLIAGE_STEP,
  type FogEye,
  type FogGeometryPresentation,
  type FogInput,
  type FogOccluder,
  type FogSight,
  type FogWorld,
  WHOLE_TEXTURE_WIDTH,
  wholeWords,
} from "./fogInputs";
import {
  FogEyeRecord,
  FogLayer,
  GroundPaintStyle,
  FogParams,
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
const EYE_BYTES = 48;
const BOX_BYTES = 32;
const PROBE_BYTES = 32;
const BUILD_WORKGROUP = 64;
const MAX_WORKGROUPS = 65535;

const FogBox = d
  .struct({ center: d.vec2f, half: d.vec2f, yaw: d.f32, top: d.f32, base: d.f32, pad1: d.f32 })
  .$name("FogBox");
const FogProbe = d
  .struct({ position: d.vec3f, ground: d.u32, normal: d.vec3f, pad: d.u32 })
  .$name("FogProbe");

const words = (n: number) => d.arrayOf(d.u32, n);
const buildLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["compute"] },
  eyes: { storage: (n: number) => d.arrayOf(FogEyeRecord, n), access: "readonly" },
  heights: { storage: (n: number) => d.arrayOf(d.u32, n), access: "readonly" },
  foliage: { storage: (n: number) => d.arrayOf(d.vec4f, n), access: "readonly" },
  occluders: { storage: (n: number) => d.arrayOf(FogBox, n), access: "readonly" },
  rebuild: { storage: words, access: "readonly" },
  terrain: { storage: words, access: "mutable" },
  maps: { storage: words, access: "mutable" },
});
const cullLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["compute"] },
  eyes: { storage: (n: number) => d.arrayOf(FogEyeRecord, n), access: "readonly" },
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
  /** Structures' step and count: (whole_step_m, count, 0, 0). */
  whole: { uniform: d.vec4f },
  flags: { storage: words, access: "mutable" },
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
  loop {
    if(lo>=hi){return vec2f(0.0);}
    let mid=(lo+hi)/2u;
    let row=buildLayout.$.foliage[mid];
    let at=u32(row.y)*P.foliageNx+u32(row.x);
    if(at==key){return row.zw;}
    if(at<key){lo=mid+1u;}else{hi=mid;}
  }
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
  let e = buildLayout.$.eyes[buildLayout.$.rebuild[r]];
  let theta = (f32(ai) + 0.5) / f32(AZT) * ${TAU} - ${PI};
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.reach / P.firstBinM);
  let base = (e.slot * AZT + ai) * R;
  let size = vec2f(f32(P.heightNx - 1u), f32(P.heightNy - 1u)) * P.heightSpacing;
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
    let outside = q.x < 0.0 || q.y < 0.0 || q.x > size.x || q.y > size.y;
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

/** Full-azimuth merge: each ray meets every known occluder analytically
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
  let e = buildLayout.$.eyes[buildLayout.$.rebuild[r]];
  let theta = (f32(ai) + 0.5) / f32(AZ) * ${TAU} - ${PI};
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.reach / P.firstBinM);
  var pm: array<f32, ${radialBins}>;
  var pt: array<f32, ${radialBins}>;
  for (var k = 0u; k < R; k++) { pm[k] = -1e4; pt[k] = 0.0; }
  for (var i = 0u; i < P.occluderCount; i++) {
    let b = buildLayout.$.occluders[i];
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
    // An eye inside a box (a garrison slot) sees out of it.
    if (miss || tn > tf || tf <= 0.0 || tn < 0.0 || tn > e.reach) { continue; }
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

/** One workgroup per structure that takes fog whole: its four walls and its
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
  let index = wg.x + wg.y * 65535u;
  if (index >= u32(W.y)) { return; }
  if (li == 0u) { atomicStore(&wholeFound, 0u); }
  workgroupBarrier();
  let P = fogLayout.$.params;
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
    // Along a side: t in [0, 1] over n samples, kept off the corners by the inset.
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
    for (var e = 0u; e < P.eyeCount; e++) {
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

/** An eye record for `probeWith`: already built, in map slot `slot`. */
export interface FogEyeRow {
  position: readonly [number, number, number];
  reach: number;
  forward: number;
  front: number;
  side: number;
  rear: number;
  range: number;
  slot: number;
}

/** The map resolution and rule numbers a synthetic probe runs under. */
export interface FogLookupParams {
  azimuthBins: number;
  radialBins: number;
  firstBinM: number;
  targetHeightM: number;
  faceProbeM: number;
  foliageFullBlock: number;
}

export interface FogVisibilityStats {
  /** Whether fog is on (an input is set). */
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

interface SlotState {
  slot: number;
  /** Where the map was built from, and to what reach; null until built. */
  built: readonly [number, number, number] | null;
  reach: number;
  eye: FogEye;
}

/** The tile lists for one frame size. */
export interface FogTiles {
  tilesX: number;
  tilesY: number;
  lists: GPUBuffer;
  counts: GPUBuffer;
  depthView: GPUTextureView;
  /** The frame's ground paint, which the painted layers read. */
  paintView: GPUTextureView;
}

function eyeRecords(rows: readonly FogEyeRow[]): ArrayBuffer {
  const bytes = new ArrayBuffer(Math.max(1, rows.length) * EYE_BYTES);
  const f = new Float32Array(bytes);
  const u = new Uint32Array(bytes);
  rows.forEach((r, i) => {
    const o = (i * EYE_BYTES) / WORD;
    f.set([...r.position, r.reach, r.forward, r.front, r.side, r.rear, r.range], o);
    u[o + 9] = r.slot;
  });
  return bytes;
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

function occluderRecords(boxes: readonly FogOccluder[]): ArrayBuffer {
  const bytes = new ArrayBuffer(Math.max(1, boxes.length) * BOX_BYTES);
  const f = new Float32Array(bytes);
  boxes.forEach((b, i) =>
    f.set([b.x, b.y, b.hx, b.hy, b.yaw, b.top, b.base], (i * BOX_BYTES) / WORD),
  );
  return bytes;
}

/** The foliage grid's cells (`canopy_m, depth_per_m` pairs), past its header. */
function foliageCells(foliage: Float32Array): Float32Array {
  return foliage.length > 3 ? foliage.subarray(3) : new Float32Array(4);
}

const sameEye = (a: readonly number[], b: readonly number[]) =>
  a[0] === b[0] && a[1] === b[1] && a[2] === b[2];

export async function createFogVisibility(
  root: Root,
  registry: GpuRegistry,
  geometry: FogGeometryPresentation,
  terrainHeights: TerrainHeights,
) {
  const device = registry.device;
  const g = geometry;
  const merge = mergeFn(g.radial_bins);
  const pipelines = {
    terrain: root.createComputePipeline({ compute: terrainMarch }),
    merge: root.createComputePipeline({ compute: merge }),
    cull: root.createComputePipeline({ compute: cullFn(g.tile_px) }),
    probe: root.createComputePipeline({ compute: probeFn }),
    shape: root.createComputePipeline({ compute: shapeFn }),
    whole: root.createComputePipeline({ compute: wholeFn }),
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
  /** A flag word per structure cell, `rows` texture rows of them. */
  const wholeFlagBuffer = (rows: number) =>
    device.createBuffer({
      label: "fog-whole-flags",
      size: rows * WHOLE_TEXTURE_WIDTH * WORD,
      usage: STORAGE | COPY_SRC,
    });
  const wholeUniform = registry.own(root.createBuffer(d.vec4f).$usage("uniform"));

  const slot = <T extends { destroy(): void }>(initial: T): GpuSlot<T> => {
    const s = registry.slot<T>();
    s.set(initial);
    return s;
  };
  const buffers = {
    eyes: slot(storage("fog-eyes", EYE_BYTES)),
    maps: slot(storage("fog-maps", WORD)),
    terrain: slot(storage("fog-terrain-maps", WORD)),
    foliage: slot(storage("fog-foliage", 16)),
    occluders: slot(storage("fog-occluders", BOX_BYTES)),
    rebuild: slot(storage("fog-rebuild", WORD)),
    wholes: slot(wholeTexture(1)),
    wholeFlags: slot(wholeFlagBuffer(1)),
  };
  /** The structures' lookup (`wholeWords`), and whether their flags are stale. */
  let wholes = wholeWords([], 0);
  let wholesDirty = false;

  let world: FogWorld | null = null;
  /** The tallest canopy: translucent surfaces stand at most this far above ground. */
  let translucentLiftM = 0;
  let sight: FogSight | null = null;
  let occluders: readonly FogOccluder[] | null = null;
  let capacity = 0;
  let eyeCapacity = 1;
  let rebuildCapacity = 1;
  const slots = new Map<string, SlotState>();
  const free: number[] = [];
  let order: SlotState[] = [];
  /** Moved eyes waiting for a rebuild, oldest first. */
  let queue: string[] = [];
  let rebuilt = 0;
  let rebuiltTotal = 0;
  let generation = 0;
  let tiles: FogTiles | null = null;
  let groupsFor: { generation: number; tiles: FogTiles | null; value: FogGroups } | null = null;

  const eyeRow = (s: SlotState): FogEyeRow => ({
    position: s.built ?? s.eye.position,
    reach: s.reach,
    forward: s.eye.forward,
    front: s.eye.shape.front,
    side: s.eye.shape.side,
    rear: s.eye.shape.rear,
    range: s.eye.range,
    slot: s.slot,
  });

  /** Every map rebuilt from scratch (a new world, new knowledge, more slots). */
  const invalidate = () => {
    for (const s of slots.values()) s.built = null;
    queue = [];
  };

  const grow = (needed: number) => {
    capacity = Math.max(needed, capacity * 2, 4);
    const AZ = g.azimuth_bins;
    const R = g.radial_bins;
    buffers.maps.set(storage("fog-maps", capacity * AZ * R * WORD));
    buffers.terrain.set(storage("fog-terrain-maps", capacity * g.terrain_azimuth_bins * R * WORD));
    generation++;
    invalidate();
  };

  const setWorld = (next: FogWorld) => {
    // The side's foliage changes during a battle (trees felled); its
    // terrain never does, so only a new map uploads the heights again.
    world = next;
    translucentLiftM = 0;
    for (let i = 5; i < next.foliage.length; i += 4)
      translucentLiftM = Math.max(translucentLiftM, next.foliage[i]);
    const foliage = foliageCells(next.foliage);
    buffers.foliage.set(storage("fog-foliage", foliage.byteLength));
    device.queue.writeBuffer(buffers.foliage.current!, 0, foliage);
    generation++;
    invalidate();
  };

  const setOccluders = (next: readonly FogOccluder[]) => {
    occluders = next;
    const bytes = occluderRecords(next);
    buffers.occluders.set(storage("fog-occluders", bytes.byteLength));
    device.queue.writeBuffer(buffers.occluders.current!, 0, bytes);
    wholes = wholeWords(next, 2 * g.face_probe_m);
    buffers.wholes.set(wholeTexture(wholes.rows));
    device.queue.writeTexture(
      { texture: buffers.wholes.current! },
      wholes.words,
      { bytesPerRow: WHOLE_TEXTURE_WIDTH * WORD },
      [WHOLE_TEXTURE_WIDTH, wholes.rows],
    );
    buffers.wholeFlags.set(wholeFlagBuffer(wholes.flagRows));
    wholeUniform.write(d.vec4f(g.whole_step_m, next.length, 0, 0));
    generation++;
    invalidate();
  };

  const setSight = (next: FogSight) => {
    sight = next;
    wholesDirty = true;
    if (next.occluders !== occluders) setOccluders(next.occluders);
    const live = new Set(next.eyes.map((e) => e.key));
    for (const [key, s] of slots) {
      if (!live.has(key)) {
        slots.delete(key);
        free.push(s.slot);
      }
    }
    queue = queue.filter((k) => live.has(k));
    const needed = next.eyes.length;
    if (needed > capacity) {
      // Slots are renumbered densely into the grown maps.
      const kept = [...slots.values()];
      free.length = 0;
      grow(needed);
      kept.forEach((s, i) => (s.slot = i));
      for (let i = capacity - 1; i >= kept.length; i--) free.push(i);
    }
    order = next.eyes.map((eye) => {
      let s = slots.get(eye.key);
      if (!s) {
        s = { slot: free.pop()!, built: null, reach: eyeReach(eye), eye };
        slots.set(eye.key, s);
        return s;
      }
      const moved = s.built && (!sameEye(s.built, eye.position) || s.reach !== eyeReach(eye));
      s.eye = eye;
      if (moved && !queue.includes(eye.key)) queue.push(eye.key);
      return s;
    });
    if (order.length > eyeCapacity) {
      eyeCapacity = Math.max(order.length, eyeCapacity * 2);
      buffers.eyes.set(storage("fog-eyes", eyeCapacity * EYE_BYTES));
      generation++;
    }
    if (order.length > rebuildCapacity) {
      rebuildCapacity = Math.max(order.length, rebuildCapacity * 2);
      buffers.rebuild.set(storage("fog-rebuild", rebuildCapacity * WORD));
      generation++;
    }
  };

  /** Pick this frame's rebuilds: every eye never built, then moved eyes up to
   *  the budget, oldest first. */
  const pickRebuilds = (): number[] => {
    const picked: number[] = [];
    order.forEach((s, i) => {
      if (!s.built) picked.push(i);
    });
    let budget = g.rebuild_eyes_per_frame;
    const index = new Map(order.map((s, i) => [s.eye.key, i]));
    while (budget > 0 && queue.length) {
      const i = index.get(queue.shift()!)!;
      if (!picked.includes(i)) picked.push(i);
      budget--;
    }
    for (const i of picked) {
      const s = order[i];
      s.built = s.eye.position;
      s.reach = eyeReach(s.eye);
    }
    return picked;
  };

  const writeParams = (extra: {
    rebuildCount: number;
    probeCount: number;
    camera?: Float32Array;
    width?: number;
    height?: number;
  }) => {
    const cam = extra.camera;
    // The camera uniform's inverse view-projection (floats 16..31) and eye (32..34).
    const column = (c: number) =>
      cam ? d.vec4f(cam[16 + c * 4], cam[17 + c * 4], cam[18 + c * 4], cam[19 + c * 4]) : d.vec4f();
    params.write({
      invViewProj: d.mat4x4f(column(0), column(1), column(2), column(3)),
      cameraEye: d.vec3f(cam?.[32] ?? 0, cam?.[33] ?? 0, cam?.[34] ?? 0),
      translucentLiftM,
      azimuthBins: g.azimuth_bins,
      terrainAzimuthBins: g.terrain_azimuth_bins,
      radialBins: g.radial_bins,
      tilePx: g.tile_px,
      tilesX: tiles?.tilesX ?? 0,
      tilesY: tiles?.tilesY ?? 0,
      tileEyesMax: g.tile_eyes_max,
      eyeCount: order.length,
      occluderCount: occluders?.length ?? 0,
      foliageNx: world && world.foliage.length > 3 ? world.foliage[0] : 0,
      foliageNy: world && world.foliage.length > 3 ? world.foliage[1] : 0,
      heightNx: world?.nx ?? 2,
      heightNy: world?.ny ?? 2,
      rebuildCount: extra.rebuildCount,
      enabled: world && sight ? 1 : 0,
      probeCount: extra.probeCount,
      firstBinM: g.first_bin_m,
      targetHeightM: world?.targetHeightM ?? 0,
      foliageCellM: world && world.foliage.length > 3 ? world.foliage[2] : 1,
      foliageFullBlock: world?.foliageFullBlock ?? 1,
      heightSpacing: world?.spacing ?? 1,
      faceProbeM: g.face_probe_m,
      width: extra.width ?? 0,
      height: extra.height ?? 0,
      stepMinM: g.terrain_step_m[0],
      stepMaxM: g.terrain_step_m[1],
      stepFraction: g.terrain_step_fraction,
      roofReachM: g.roof_reach_m,
      ...wholes.params,
      wholeOrigin: d.vec2f(...wholes.params.wholeOrigin),
      wholeCount: world && sight ? wholes.params.wholeCount : 0,
    });
  };

  const buildGroup = () =>
    root.createBindGroup(buildLayout, {
      params,
      eyes: buffers.eyes.current!,
      heights: terrainHeights.forGrid(world!),
      foliage: buffers.foliage.current!,
      occluders: buffers.occluders.current!,
      rebuild: buffers.rebuild.current!,
      terrain: buffers.terrain.current!,
      maps: buffers.maps.current!,
    });
  let build: { generation: number; group: ReturnType<typeof buildGroup> } | null = null;

  /** Rebuild the maps of the eyes picked this frame; writes the eye table. */
  const prepare = (
    encoder: GPUCommandEncoder,
    frame: { probeCount: number; camera?: Float32Array; width?: number; height?: number },
  ) => {
    const active = !!(world && sight);
    const picked = active ? pickRebuilds() : [];
    rebuilt = picked.length;
    rebuiltTotal += rebuilt;
    if (active) {
      device.queue.writeBuffer(buffers.eyes.current!, 0, eyeRecords(order.map(eyeRow)));
      if (picked.length)
        device.queue.writeBuffer(buffers.rebuild.current!, 0, new Uint32Array(picked));
    }
    writeParams({ rebuildCount: picked.length, ...frame });
    if (picked.length) wholesDirty = true;
    if (!picked.length) return;
    if (!build || build.generation !== generation) build = { generation, group: buildGroup() };
    const dispatch = (bins: number) => {
      const n = Math.ceil((picked.length * bins) / BUILD_WORKGROUP);
      if (n > MAX_WORKGROUPS)
        throw new Error(`fog: ${picked.length} eyes exceed one rebuild dispatch`);
      return n;
    };
    pipelines.terrain
      .with(build.group)
      .with(encoder)
      .dispatchWorkgroups(dispatch(g.terrain_azimuth_bins));
    pipelines.merge.with(build.group).with(encoder).dispatchWorkgroups(dispatch(g.azimuth_bins));
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
      lists: t.lists,
      counts: t.counts,
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
          cull: tiles ? cullGroup(tiles) : null,
        },
      };
    }
    return groupsFor.value;
  };

  const readback = async (source: GPUBuffer, bytes: number): Promise<ArrayBuffer> => {
    const read = device.createBuffer({ size: bytes, usage: MAP_READ | COPY_DST });
    try {
      const encoder = device.createCommandEncoder({ label: "fog-readback" });
      encoder.copyBufferToBuffer(source, 0, read, 0, bytes);
      device.queue.submit([encoder.finish()]);
      await read.mapAsync(1);
      return read.getMappedRange().slice(0);
    } finally {
      read.destroy();
    }
  };
  const outBuffer = (bytes: number) =>
    device.createBuffer({ size: Math.max(16, bytes), usage: STORAGE | COPY_SRC });

  /** Run the probe pipeline with `group` bound as fog. */
  const runProbe = async (
    group: ReturnType<typeof fragmentGroup>,
    points: readonly FogProbeInput[],
    before?: (encoder: GPUCommandEncoder) => void,
  ): Promise<Uint8Array> => {
    const records = probeRecords(points);
    const input = storage("fog-probe-points", records.byteLength);
    const out = outBuffer(points.length * WORD);
    try {
      device.queue.writeBuffer(input, 0, records);
      const encoder = device.createCommandEncoder({ label: "fog-probe" });
      before?.(encoder);
      pipelines.probe
        .with(group)
        .with(root.createBindGroup(probeLayout, { points: input, out }))
        .with(encoder)
        .dispatchWorkgroups(Math.max(1, Math.ceil(points.length / BUILD_WORKGROUP)));
      device.queue.submit([encoder.finish()]);
      return Uint8Array.from(new Uint32Array(await readback(out, points.length * WORD)));
    } finally {
      input.destroy();
      out.destroy();
    }
  };

  return {
    /** The fog to draw: world, eyes and occluders; `null` draws everything seen. */
    set(input: FogInput | null) {
      if (!input) {
        sight = null;
        return;
      }
      if (input.world !== world) setWorld(input.world);
      setSight(input.sight);
    },
    /** The tile lists for a frame size, owned by that size's scope. */
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
    sized(
      scope: GpuRegistry,
      width: number,
      height: number,
      depth: GPUTexture,
      paint: GPUTexture,
    ): FogTiles {
      const tilesX = Math.ceil(width / g.tile_px);
      const tilesY = Math.ceil(height / g.tile_px);
      const n = tilesX * tilesY;
      return {
        tilesX,
        tilesY,
        lists: scope.buffer({
          label: "fog-tile-lists",
          size: n * g.tile_eyes_max * WORD,
          usage: STORAGE,
        }),
        counts: scope.buffer({
          label: "fog-tile-counts",
          size: n * WORD,
          usage: STORAGE | COPY_DST | COPY_SRC,
        }),
        depthView: depth.createView(),
        paintView: paint.createView(),
      };
    },
    /** This frame's rebuilds, then the tile cull over the prepass depth.
     *  `camera` is the frame's packed camera uniform. */
    encode(
      encoder: GPUCommandEncoder,
      frameTiles: FogTiles,
      camera: Float32Array,
      width: number,
      height: number,
    ) {
      tiles = frameTiles;
      prepare(encoder, { probeCount: 0, camera, width, height });
      if (!world || !sight) return;
      const { cull, faces } = groups();
      if (wholesDirty && occluders?.length) {
        // Flag each structure seen or not, then copy the flags into the
        // `wholes` texture the fragments read.
        wholesDirty = false;
        const n = occluders.length;
        pipelines.whole
          .with(faces)
          .with(
            root.createBindGroup(wholeLayout, {
              boxes: buffers.occluders.current!,
              whole: wholeUniform,
              flags: buffers.wholeFlags.current!,
            }),
          )
          .with(encoder)
          .dispatchWorkgroups(Math.min(n, MAX_WORKGROUPS), Math.ceil(n / MAX_WORKGROUPS));
        encoder.copyBufferToTexture(
          { buffer: buffers.wholeFlags.current!, bytesPerRow: WHOLE_TEXTURE_WIDTH * WORD },
          { texture: buffers.wholes.current!, origin: [0, wholes.flagsRow] },
          [WHOLE_TEXTURE_WIDTH, wholes.flagRows],
        );
      }
      encoder.clearBuffer(frameTiles.counts);
      pipelines.cull
        .with(cull!)
        .with(encoder)
        .dispatchWorkgroups(frameTiles.tilesX, frameTiles.tilesY);
    },
    /** The fragment bind groups: ground, faces standing on it, and units
     *  (never fogged). */
    groups,
    stats(): FogVisibilityStats {
      const R = g.radial_bins;
      return {
        enabled: !!(world && sight),
        eyes: order.length,
        slots: capacity,
        occluders: occluders?.length ?? 0,
        rebuilt,
        pending: queue.length,
        rebuiltTotal,
        mapBytes: capacity * g.azimuth_bins * R * WORD,
        terrainMapBytes: capacity * g.terrain_azimuth_bins * R * WORD,
        tileListBytes: tiles ? tiles.tilesX * tiles.tilesY * (g.tile_eyes_max + 1) * WORD : 0,
        tiles: tiles ? [tiles.tilesX, tiles.tilesY] : [0, 0],
      };
    },
    /** Fog at `points` against every eye, after building any maps still due. */
    async probe(points: readonly FogProbeInput[]): Promise<Uint8Array> {
      if (!world || !sight) return new Uint8Array(points.length).fill(1);
      return runProbe(groups().faces, points, (encoder) =>
        prepare(encoder, { probeCount: points.length }),
      );
    },
    /** Rebuild every eye's maps on the next frame, as if all had moved (a
     *  cost probe). */
    rebuildAll() {
      invalidate();
    },
    /** The per-tile eye counts of the last frame (a debug readback). */
    async tileCounts(): Promise<Uint32Array> {
      if (!tiles) return new Uint32Array(0);
      const bytes = tiles.tilesX * tiles.tilesY * WORD;
      return new Uint32Array(await readback(tiles.counts, bytes));
    },
    /** The structures that take fog whole, and whether each was flagged
     *  seen by the last frame (a debug readback). */
    async wholes(): Promise<{ boxes: readonly FogOccluder[]; seen: Uint8Array }> {
      const boxes = occluders ?? [];
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
    /** The lookup over given maps and eyes (oracle vectors): nothing built. */
    async probeWith(
      lookup: FogLookupParams,
      eyes: readonly FogEyeRow[],
      maps: Uint32Array,
      points: readonly FogProbeInput[],
    ): Promise<Uint8Array> {
      const scratchParams = root.createBuffer(FogParams).$usage("uniform");
      const eyeBytes = eyeRecords(eyes);
      const eyeBuffer = storage("fog-oracle-eyes", eyeBytes.byteLength);
      const mapBuffer = storage("fog-oracle-maps", maps.byteLength);
      try {
        device.queue.writeBuffer(eyeBuffer, 0, eyeBytes);
        device.queue.writeBuffer(mapBuffer, 0, maps);
        scratchParams.write({
          ...FOG_PARAMS_ZERO,
          azimuthBins: lookup.azimuthBins,
          radialBins: lookup.radialBins,
          eyeCount: eyes.length,
          probeCount: points.length,
          enabled: 1,
          firstBinM: lookup.firstBinM,
          targetHeightM: lookup.targetHeightM,
          faceProbeM: lookup.faceProbeM,
          foliageFullBlock: lookup.foliageFullBlock,
        });
        const group = root.createBindGroup(fogLayout, {
          params: scratchParams,
          layer: layers.faces,
          eyes: eyeBuffer,
          maps: mapBuffer,
          lists: mapBuffer,
          counts: mapBuffer,
          paintStyle,
          paint: noPaint.createView(),
          wholes: buffers.wholes.current!.createView(),
        });
        return await runProbe(group, points);
      } finally {
        scratchParams.destroy();
        eyeBuffer.destroy();
        mapBuffer.destroy();
      }
    },
  };
}
export type FogVisibility = Awaited<ReturnType<typeof createFogVisibility>>;
/** The lab's hold on the sight lights: debug readbacks, never in a frame. */
export type FogProbes = Pick<
  FogVisibility,
  "probe" | "probeShape" | "probeWith" | "tileCounts" | "rebuildAll" | "wholes"
>;

const FOG_PARAMS_ZERO = {
  invViewProj: d.mat4x4f(),
  cameraEye: d.vec3f(),
  translucentLiftM: 0,
  azimuthBins: 0,
  terrainAzimuthBins: 0,
  radialBins: 0,
  tilePx: 1,
  tilesX: 0,
  tilesY: 0,
  tileEyesMax: 0,
  eyeCount: 0,
  occluderCount: 0,
  foliageNx: 0,
  foliageNy: 0,
  heightNx: 2,
  heightNy: 2,
  rebuildCount: 0,
  enabled: 0,
  probeCount: 0,
  firstBinM: 1,
  targetHeightM: 0,
  foliageCellM: 1,
  foliageFullBlock: 1,
  heightSpacing: 1,
  faceProbeM: 0,
  width: 0,
  height: 0,
  stepMinM: 1,
  stepMaxM: 1,
  stepFraction: 1,
  roofReachM: 0,
  wholeCount: 0,
  wholeNx: 1,
  wholeNy: 1,
  wholeCellM: 1,
  wholeOrigin: d.vec2f(),
  wholeItemsBase: 0,
  wholeBoxesBase: 0,
  wholeFlagsBase: 0,
};
