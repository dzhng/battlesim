// A CPU mirror of fog's per-eye lookup (`fogSeenBy` in fogTerm.ts) and of its
// map word, for oracle vectors only: synthetic maps and points evaluated here
// and by the WGSL must agree wherever no comparison sits within float noise.
// It never draws; the GPU lookup is the one owner of fog in pixels.
import { mulberry32 } from "math/random";
import type { FogEyeRow, FogLookupParams, FogProbeInput } from "./fogVisibility";

type P3 = readonly [number, number, number];

const _f16_view = new DataView(new ArrayBuffer(4));

/** f32 → f16 bits, round to nearest even (WGSL `pack2x16float`'s rounding). */
export function f16Bits(value: number): number {
  _f16_view.setFloat32(0, value);
  const x = _f16_view.getUint32(0);
  const sign = (x >>> 16) & 0x8000;
  const exp = (x >>> 23) & 0xff;
  let mant = x & 0x7fffff;
  if (exp === 0xff) return sign | 0x7c00 | (mant ? 0x200 : 0);
  let e = exp - 127 + 15;
  if (e >= 0x1f) return sign | 0x7c00;
  if (e <= 0) {
    if (e < -10) return sign;
    mant |= 0x800000;
    const shift = 14 - e;
    const half = 1 << (shift - 1);
    let m = mant >>> shift;
    const rest = mant & ((1 << shift) - 1);
    if (rest > half || (rest === half && m & 1)) m++;
    return sign | m;
  }
  let m = mant >>> 13;
  const rest = mant & 0x1fff;
  if (rest > 0x1000 || (rest === 0x1000 && m & 1)) {
    m++;
    if (m === 0x400) {
      m = 0;
      e++;
      if (e >= 0x1f) return sign | 0x7c00;
    }
  }
  return sign | (e << 10) | m;
}

/** f16 bits → the number they hold. */
export function f16Value(bits: number): number {
  const sign = bits & 0x8000 ? -1 : 1;
  const exp = (bits >>> 10) & 0x1f;
  const mant = bits & 0x3ff;
  if (exp === 0) return sign * mant * 2 ** -24;
  if (exp === 0x1f) return mant ? NaN : sign * Infinity;
  return sign * (1 + mant / 1024) * 2 ** (exp - 15);
}

/** A map word as the build writes it: horizon slope, jump position in the
 *  bin (0..1), foliage metres. */
export function packFogWord(horizon: number, jump: number, foliage: number): number {
  const h = f16Bits(Math.min(6e4, Math.max(-6e4, horizon)));
  const j = Math.round(Math.min(1, Math.max(0, jump)) * 255);
  const f = Math.min(Math.round(foliage * 2), 255);
  return (h | (j << 16) | (f << 24)) >>> 0;
}

export function unpackFogWord(w: number): [number, number, number] {
  return [f16Value(w & 0xffff), ((w >>> 16) & 0xff) / 255, (w >>> 24) * 0.5];
}

/** The multiplier of `sim::sight::multiplier` (mirrored in `fogShape`). */
function shape(front: number, side: number, rear: number, off: number) {
  const c = Math.cos(off);
  return side * (1 - c * c) + (c >= 0 ? front : rear) * c * c;
}

function binEdge(k: number, first: number, lnr: number, bins: number) {
  return k < 0 ? 0 : first * Math.exp((k / (bins - 1)) * lnr);
}

/** How far a comparison sits from flipping, relative to its size. */
const gap = (a: number, b: number) => Math.abs(a - b) / Math.max(Math.abs(a), Math.abs(b), 1e-3);

/** Whether eye `e` sees `p` (face normal `n`, zero for ground), and the
 *  smallest relative gap of any comparison on the way. */
export function oracleSeenBy(
  maps: Uint32Array,
  lookup: FogLookupParams,
  e: FogEyeRow,
  p: P3,
  n: P3,
): { seen: boolean; margin: number } {
  let margin = Infinity;
  const decide = (a: number, b: number) => {
    margin = Math.min(margin, gap(a, b));
    return a > b;
  };
  const face = n[0] * n[0] + n[1] * n[1] + n[2] * n[2] > 0;
  const toEye =
    n[0] * (e.position[0] - p[0]) + n[1] * (e.position[1] - p[1]) + n[2] * (e.position[2] - p[2]);
  if (face && !decide(toEye, 0)) return { seen: false, margin };
  const dx = p[0] - e.position[0];
  const dy = p[1] - e.position[1];
  const dist = Math.hypot(dx, dy);
  if (!decide(dist, lookup.firstBinM)) return { seen: true, margin };
  const theta = Math.atan2(dy, dx);
  const range = e.range * shape(e.front, e.side, e.rear, theta - e.forward);
  if (decide(dist, range)) return { seen: false, margin };
  const AZ = lookup.azimuthBins;
  const R = lookup.radialBins;
  const lnr = Math.log(e.reach / lookup.firstBinM);
  const kf = (Math.log(dist / lookup.firstBinM) / lnr) * (R - 1);
  margin = Math.min(margin, Math.abs(kf - Math.round(kf)));
  const k = Math.min(Math.ceil(kf), R - 1);
  const lo = binEdge(k - 1, lookup.firstBinM, lnr, R);
  const hi = binEdge(k, lookup.firstBinM, lnr, R);
  const a = ((theta + Math.PI) / (2 * Math.PI)) * AZ - 0.5;
  const af = Math.floor(a);
  const fa = a - af;
  margin = Math.min(margin, fa, 1 - fa);
  const i0 = (af + AZ) % AZ;
  const i1 = (i0 + 1) % AZ;
  const w0 = (e.slot * AZ + i0) * R + k;
  const w1 = (e.slot * AZ + i1) * R + k;
  const c0 = unpackFogWord(maps[w0]);
  const c1 = unpackFogWord(maps[w1]);
  const p0 = k > 0 ? unpackFogWord(maps[w0 - 1]) : [-1e4, 0, 0];
  const p1 = k > 0 ? unpackFogWord(maps[w1 - 1]) : [-1e4, 0, 0];
  const mix = (x: number, y: number, t: number) => x + (y - x) * t;
  const t0 = mix(lo, hi, c0[1]);
  const t1 = mix(lo, hi, c1[1]);
  const eps = 0.1 + (hi - lo) / 255;
  let tanInc = 0;
  const nxy = Math.hypot(n[0], n[1]);
  if (face && nxy > 1e-3) {
    const c = Math.min(1, Math.max(0.017, Math.abs((dx * n[0] + dy * n[1]) / (dist * nxy))));
    tanInc = Math.sqrt(1 - c * c) / c;
  }
  const spread = 1 + 0.01 * dist + 1.5 * dist * ((2 * Math.PI) / AZ) * tanInc;
  let horizon: number;
  if (!decide(Math.abs(t0 - t1), spread)) {
    const tj = mix(t0, t1, fa);
    horizon = decide(dist, tj + eps) ? mix(c0[0], c1[0], fa) : mix(p0[0], p1[0], fa);
  } else {
    const h0 = decide(dist, t0 + eps) ? c0[0] : p0[0];
    const h1 = decide(dist, t1 + eps) ? c1[0] : p1[0];
    horizon = mix(h0, h1, fa);
  }
  const u = Math.min(1, Math.max(0, (dist - lo) / Math.max(hi - lo, 1e-6)));
  const foliage = mix(mix(p0[2], c0[2], u), mix(p1[2], c1[2], u), fa);
  if (!decide(lookup.forestFullBlockM, foliage)) return { seen: false, margin };
  if (decide(dist, range * Math.exp(-foliage / lookup.forestAttenuationM)))
    return { seen: false, margin };
  const slope = (p[2] - e.position[2]) / dist;
  return { seen: !decide(horizon, slope), margin };
}

/** Where a probe input is tested, as the GPU's `fogProbePoint` puts it. */
export function probePoint(lookup: FogLookupParams, q: FogProbeInput): { p: P3; n: P3 } {
  const [x, y, z] = q.position;
  if (!q.normal) return { p: [x, y, z + lookup.targetHeightM], n: [0, 0, 0] };
  const [nx, ny, nz] = q.normal;
  const s = lookup.faceProbeM;
  return { p: [x + nx * s, y + ny * s, z + nz * s], n: q.normal };
}

/** Seeded synthetic oracle vectors: eyes, their maps (rising horizons with
 *  jumps inside bins, growing foliage) and probe points around them, half on
 *  ground and half on faces. */
export function oracleVectors(seed: number, lookup: FogLookupParams, eyes = 3, points = 4000) {
  const rng = mulberry32.create(seed);
  const next = () => mulberry32.sample(rng);
  const AZ = lookup.azimuthBins;
  const R = lookup.radialBins;
  const rows: FogEyeRow[] = [];
  const maps = new Uint32Array(eyes * AZ * R);
  for (let s = 0; s < eyes; s++) {
    const range = 200 + next() * 600;
    const front = 1;
    const side = 0.3 + next() * 0.7;
    const rear = side * (0.2 + next() * 0.8);
    rows.push({
      position: [500 + next() * 100, 500 + next() * 100, 2 + next() * 20],
      reach: range,
      forward: (next() * 2 - 1) * Math.PI,
      front,
      side,
      rear,
      range,
      slot: s,
    });
    for (let i = 0; i < AZ; i++) {
      let h = -0.3 + next() * 0.2;
      let fol = 0;
      for (let k = 0; k < R; k++) {
        if (next() < 0.35) h += next() * 0.08;
        if (next() < 0.15) fol = Math.min(120, fol + next() * 20);
        maps[(s * AZ + i) * R + k] = packFogWord(h, next(), fol);
      }
    }
  }
  const probes: FogProbeInput[] = [];
  for (let j = 0; j < points; j++) {
    const e = rows[j % eyes];
    const theta = next() * 2 * Math.PI;
    const d = 0.5 + next() * e.reach * 1.1;
    const x = e.position[0] + Math.cos(theta) * d;
    const y = e.position[1] + Math.sin(theta) * d;
    const z = e.position[2] + (next() * 0.8 - 0.5) * d;
    if (next() < 0.5) probes.push({ position: [x, y, z] });
    else {
      const a = next() * 2 * Math.PI;
      const up = next() < 0.2 ? 1 : 0;
      probes.push({
        position: [x, y, z],
        normal: up ? [0, 0, 1] : [Math.cos(a), Math.sin(a), 0],
      });
    }
  }
  return { eyes: rows, maps, points: probes };
}

/** The oracle's answers for vectors: seen by any eye, and the smallest gap
 *  among the eyes that decided it. */
export function oracleAnswers(
  lookup: FogLookupParams,
  vectors: ReturnType<typeof oracleVectors>,
): { seen: boolean; margin: number }[] {
  return vectors.points.map((q) => {
    const { p, n } = probePoint(lookup, q);
    let margin = Infinity;
    for (const e of vectors.eyes) {
      const r = oracleSeenBy(vectors.maps, lookup, e, p, n);
      margin = Math.min(margin, r.margin);
      if (r.seen) return { seen: true, margin };
    }
    return { seen: false, margin };
  });
}
