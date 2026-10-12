// The surface field's layout as the terrain material reads it: every ground
// rule's signed distance (paved, forest, water) from one bucket index, so a
// lookup costs what lies near the point instead of what the map holds. The
// ground builds it (`crates/presentation/src/static_world/ground/field.rs`,
// which says how the records and the ladder of grids are laid out, and holds
// the same lookups on the CPU).

/** Floats per record: a stroke's stretch `a, b, half width`, its kind or
 *  forest, and its cut ends, then for a paved stretch how far along its
 *  stroke it starts; a triangle `a, b, c`, its kind, and for a paved one the
 *  bearing its area's slabs are laid square to; an exposed boundary edge
 *  `a, b`; a rect `min, max`. A river's stretch is two records: `a, b`, the
 *  half width at `a` and at `b`, the bank's grade at each; then the bank's
 *  height at each, which no cell lists. */
export const SURFACE_FLOATS = 8;
/** A stretch is cut square at its end `a` (`b`): its stroke ends there. */
export const CUT_A = 1;
export const CUT_B = 2;

/** An index entry: the record's kind in the top two bits (a rect, a stroke's
 *  stretch, a triangle, else a boundary edge), then whether it opens a new
 *  forest shape in its cell's list, then the record. */
export const SURFACE_RECT = 0;
export const SURFACE_STROKE = 1;
export const SURFACE_TRIANGLE = 2;
export const SURFACE_KIND_SHIFT = 30;
export const SURFACE_NEW_SHAPE = 1 << 29;
export const SURFACE_RECORD_MASK = SURFACE_NEW_SHAPE - 1;
/** Words per cell: where its paved, forest and water lists start. Each list
 *  ends where the next starts, the last at the next cell's first. */
export const SURFACE_CELL_WORDS = 3;
/** Words per level at the head of the index: its first header word, and how
 *  many times its cells double the finest. */
export const SURFACE_LEVEL_WORDS = 2;
