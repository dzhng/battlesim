/** The shadow settings a graphics profile may override on an authored light. */
export interface ShadowQuality {
  shadowMapSize: 512 | 1024 | 2048;
  shadowReach: number;
}

/** Apply display-quality shadow bounds without owning the light schema. */
export function applyShadowQuality<T extends { cascades: { map_size: number; max_far_m: number } }>(
  light: T,
  quality: ShadowQuality | null | undefined,
): T {
  if (!quality) return light;
  return {
    ...light,
    cascades: {
      ...light.cascades,
      map_size: quality.shadowMapSize,
      max_far_m: quality.shadowReach,
    },
  };
}
