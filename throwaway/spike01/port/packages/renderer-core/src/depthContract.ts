export const GPU_DEPTH_MODES = ['read', 'read-write', 'write'] as const;

export type GpuDepthMode = typeof GPU_DEPTH_MODES[number];

export const GPU_WORLD_DEPTH_ATTACHMENT = 'worldDepth' as const;

// The ONE depth convention engine-wide: reverse-Z (near → 1, far → 0) in a
// depth32float buffer cleared to 0, compared `greater`/`greater-equal`.
// Reverse-Z spends 32-bit float precision where it matters (the far plane is
// exactly 0) and pairs with camera3d's infinite-far projection. depth32float
// is WebGPU-core mandatory, so there is no format fallback.
export const GPU_DEPTH_FORMAT = 'depth32float' as const;
export const GPU_DEPTH_CLEAR = 0;

export function isGpuDepthMode(value: unknown): value is GpuDepthMode {
  return typeof value === 'string' && (GPU_DEPTH_MODES as readonly string[]).includes(value);
}

/** Reverse-Z is not a label a renderer can claim: it is the pairing of a
 * clear at the far plane with a `greater` comparison. A forward-Z frame clears
 * to 1 and compares `less`, so reading both off the resources and pipelines a
 * renderer actually installed reports the convention rather than asserting it. */
export function isGpuReverseZ(compare: GPUCompareFunction, clearValue: number): boolean {
  return clearValue === GPU_DEPTH_CLEAR && (compare === 'greater' || compare === 'greater-equal');
}
