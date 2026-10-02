// The ground-classes frame view's encoding (`FrameView` "ground-classes"):
// what the terrain material says the ground under a pixel is, as three bytes.
// The material writes it (`terrainMaterial.ts` `groundClasses`); the ground
// rig reads it back (`web/scenes/_groundStations.mjs` `classAt`). Black is "not
// ground". The first byte is the distance outside the nearest paved edge and
// the second outside the water's edge, negative inside: exact as far as the
// ground's look reads that distance (`groundReach`), holding its side beyond.
// A pixel across two classes holds a mix of their bytes.

/** A distance byte moves one step every 1 / this many metres. */
export const CLASS_STEPS_PER_M = 8;
/** The distance byte at an edge: lower bytes are inside. */
export const CLASS_EDGE = 128;
/** The third byte: forest in its top 2 bits, then the plot's kind in 4, then
 *  2 bits of the plot's hashed index. */
export const CLASS_FOREST_SHIFT = 6;
export const CLASS_KIND_SHIFT = 2;
export const CLASS_KIND_MAX = 15;
export const CLASS_HASH_MASK = 3;
export const FOREST_CLASSES = ["none", "verge", "inside"] as const;
