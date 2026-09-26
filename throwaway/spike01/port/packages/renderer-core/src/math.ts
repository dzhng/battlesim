/** Scalar helpers shared by every renderer package; one owner, no per-package copies. */

export function clamp01(value: number): number {
  return Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
}

export function smoothstep(edge0: number, edge1: number, value: number): number {
  const t = Math.max(0, Math.min(1, (value - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
}

/** Deterministic [0,1) hash used by CPU-side renderer data builders. */
export function hash2(x: number, y: number): number {
  let n = (x * 374761393 + y * 668265263) | 0;
  n = Math.imul(n ^ (n >>> 13), 1274126177);
  return ((n ^ (n >>> 16)) >>> 0) / 4294967296;
}

/** Millisecond telemetry rounded to the precision exposed by renderer stats. */
export function roundMs(value: number) {
  return Number.isFinite(value) ? Number(value.toFixed(3)) : 0;
}
