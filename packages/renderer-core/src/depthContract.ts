// The ONE depth convention: reverse-Z (near → 1, far → 0) in a depth32float
// buffer cleared to 0, compared `greater`/`greater-equal`. Reverse-Z spends
// 32-bit float precision where it matters and pairs with camera3d's infinite-far
// projection. depth32float is WebGPU-core mandatory, so there is no fallback.
export const GPU_DEPTH_FORMAT = "depth32float" as const;
export const GPU_DEPTH_CLEAR = 0;
export const GPU_DEPTH_COMPARE = "greater" as const;
