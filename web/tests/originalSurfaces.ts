/** Test-only input relocation. Original captures/digests remain untouched. */
export function originalSurfaceInput<
  T extends Record<string, unknown> & {
    roads?: { points: number[][]; width_m: number }[];
  },
>(original: T) {
  const { roads, ...map } = original;
  if (!roads) return map;
  return {
    ...map,
    surfaces: roads.map((road) => ({
      kind: "road",
      shape: { kind: "stroke", points: road.points, width_m: road.width_m },
    })),
  };
}
