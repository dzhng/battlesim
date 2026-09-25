// The ONE depth convention: reverse-Z (near → 1, far → 0) in a depth32float
// buffer cleared to 0, compared `greater`/`greater-equal`. Reverse-Z spends
// 32-bit float precision where it matters and pairs with camera3d's infinite-far
// projection. depth32float is WebGPU-core mandatory, so there is no fallback.
export const GPU_DEPTH_FORMAT = "depth32float" as const;
export const GPU_DEPTH_CLEAR = 0;
export const GPU_DEPTH_COMPARE = "greater" as const;

/** Reverse-Z is not a label a renderer can claim: it is the pairing of a clear
 * at the far plane with a `greater` comparison. Reading both off what a renderer
 * actually installed reports the convention rather than asserting it. */
export function isGpuReverseZ(compare: GPUCompareFunction, clearValue: number): boolean {
  return clearValue === GPU_DEPTH_CLEAR && (compare === "greater" || compare === "greater-equal");
}
