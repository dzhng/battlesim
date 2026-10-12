// The paved kinds the native layout names, as the ground readers know them.
/** The kinds a paved record's tag names, in the native layout's order: a
 *  stretch, triangle or boundary edge carries its area's kind as an index
 *  into these. The biome's `roads` rows are keyed by them. */
export const SURFACE_AREA_KINDS = ["road", "country_road", "dirt_track", "paving"] as const;
export type SurfaceAreaKind = (typeof SURFACE_AREA_KINDS)[number];
/** Which of them are carriageways: all but paving. */
const ROAD_AREA_KINDS: readonly string[] = ["road", "country_road", "dirt_track"];

/** The native layout's own words for the paved kinds. */
export interface SurfaceKindLayout {
  surfaceAreaKinds: string[];
  roadAreaKinds: string[];
}

/** Refuse a native layout whose paved kinds this module would misname (one
 *  from an older build names none). */
export function checkSurfaceKinds(layout: Partial<SurfaceKindLayout>): void {
  const same = (a: readonly string[] | undefined, b: readonly string[]) =>
    a?.length === b.length && a.every((kind, k) => kind === b[k]);
  if (
    !same(layout.surfaceAreaKinds, SURFACE_AREA_KINDS) ||
    !same(layout.roadAreaKinds, ROAD_AREA_KINDS)
  )
    throw new Error("paved surface kinds differ from the ones the ground readers expect");
}

/** Whether a paved record's kind `tag` is a carriageway. */
export const isRoad = (tag: number) => ROAD_AREA_KINDS.includes(SURFACE_AREA_KINDS[tag]);
