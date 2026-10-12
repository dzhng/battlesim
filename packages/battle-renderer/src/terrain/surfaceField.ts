// The surface field: every ground rule's signed distance (paved, forest,
// water) from one bucket index, so a lookup costs what lies near the point
// instead of what the map holds. The ground builds it
// (`crates/static-world/src/ground/field.rs`, which says how the records and
// the ladder of grids are laid out); the terrain material reads it in WGSL,
// and the functions here are the same lookup on the CPU, held to the
// all-primitives distance by `surfaceField.test.ts`.
import { vec2, type Vec2 } from "math";
import { segment2, triangle2 } from "math/shapes";
import type { Ground } from "@wasm/presentation/presentation_wasm.js";
import { stretchInside } from "./rivers";
import { strokeInside } from "./strokes";

/** Floats per record: a stroke's stretch `a, b, half width`, its kind or
 *  forest, and its cut ends (`strokes.ts`), then for a paved stretch how far
 *  along its stroke it starts; a triangle `a, b, c`, its kind, and for a
 *  paved one the bearing its area's slabs are laid square to; an exposed
 *  boundary edge `a, b`; a rect `min, max`. A river's stretch is two
 *  records: `a, b`, the half width at `a` and at `b`, the bank's grade at
 *  each; then the bank's height at each, which no cell lists. */
export const SURFACE_FLOATS = 8;
/** Where a paved stretch's record holds the length of its stroke up to its
 *  end `a`: what runs along a road (its markings, a walk's slabs) is laid out
 *  by it, unbroken from stretch to stretch round a bend. */
export const SURFACE_STROKE_ALONG = 7;
/** Where a paved triangle's record holds its area's slab bearing. */
export const SURFACE_AREA_BEARING = 7;

/** An index entry: the record's kind in the top two bits, then whether it
 *  opens a new forest shape in its cell's list, then the record. */
export const SURFACE_RECT = 0;
export const SURFACE_STROKE = 1;
export const SURFACE_TRIANGLE = 2;
export const SURFACE_EDGE = 3;
export const SURFACE_KIND_SHIFT = 30;
export const SURFACE_NEW_SHAPE = 1 << 29;
export const SURFACE_RECORD_MASK = SURFACE_NEW_SHAPE - 1;
/** Words per cell: where its paved, forest and water lists start. Each list
 *  ends where the next starts, the last at the next cell's first. */
export const SURFACE_CELL_WORDS = 3;
/** Words per level at the head of the index: its first header word, and how
 *  many times its cells double the finest. */
export const SURFACE_LEVEL_WORDS = 2;

/** The finest level's widest pixel; each level up serves twice its
 *  footprint. */
export const SURFACE_FOOTPRINT_M = 2;
/** Levels the ground may build: the widest is far past any map. */
const SURFACE_LEVELS = 32;

/** How far outside (and inside) its edge each rule is read, in metres. */
export interface SurfaceReach {
  paved: number;
  forest: number;
  water: number;
}

export interface SurfaceField {
  /** `SURFACE_FLOATS` per record: paved strokes, triangles and boundary
   *  edges; each non-rectangle forest's strokes, triangles and edges; forest
   *  rects; river stretches. */
  records: Float32Array;
  /** `SURFACE_LEVEL_WORDS` per level; then, per level, its cells' headers
   *  (`SURFACE_CELL_WORDS` each, one closing word) and their entries. Every
   *  offset is a word of this array. */
  index: Uint32Array;
  /** The grid's low corner, exact in f32. */
  origin: readonly [number, number];
  /** The finest level's cell side, cells and widest pixel. */
  cellM: number;
  cols: number;
  rows: number;
  footprintM: number;
  levels: number;
  /** The widest pixel read out to its own reach: infinite when the ladder
   *  ends in the whole map, else the last level's footprint. */
  exactToM: number;
}

const _a: Vec2 = [0, 0],
  _b: Vec2 = [0, 0],
  _c: Vec2 = [0, 0],
  _point: Vec2 = [0, 0],
  _closest: Vec2 = [0, 0];

/** `ground`'s field of its ground rules, with its paved strokes as drawn
 *  (`TerrainSurface.strokes`) or as exported. `reach` says how far each rule
 *  is read at a pixel `footprintM` wide; it must not shrink as the pixel
 *  grows. */
export function surfaceField(
  ground: Ground,
  reach: (footprintM: number) => SurfaceReach,
  drawn: boolean,
): SurfaceField {
  const reaches = new Float64Array(SURFACE_LEVELS * 3);
  for (let level = 0; level < SURFACE_LEVELS; level++) {
    const r = reach(SURFACE_FOOTPRINT_M * 2 ** level);
    reaches.set([r.paved, r.forest, r.water], level * 3);
  }
  const built = ground.field(drawn, reaches);
  const [originX, originY, cellM, cols, rows, levels, exactToM] = built.grid();
  const field = {
    records: built.records(),
    index: built.index(),
    origin: [originX, originY] as const,
    cellM,
    cols,
    rows,
    footprintM: SURFACE_FOOTPRINT_M,
    levels,
    exactToM,
  };
  built.free();
  return field;
}

/** The cell under `(x, y)` for a pixel `footprintM` wide, as `out`: where its
 *  paved, forest and water lists start in `field.index`, and where they end.
 *  The GPU's `groundCell`, in its f32 arithmetic. */
export function surfaceCell(
  out: [number, number, number, number],
  field: SurfaceField,
  x: number,
  y: number,
  footprintM: number,
): [number, number, number, number] {
  let level = 0;
  let cap = field.footprintM;
  while (footprintM > cap && level + 1 < field.levels) {
    cap *= 2;
    level++;
  }
  const shift = field.index[level * SURFACE_LEVEL_WORDS + 1];
  const inverse = Math.fround(1 / field.cellM);
  const cx =
    Math.min(
      field.cols - 1,
      Math.max(0, Math.floor(Math.fround(Math.fround(Math.fround(x) - field.origin[0]) * inverse))),
    ) >> shift;
  const cy =
    Math.min(
      field.rows - 1,
      Math.max(0, Math.floor(Math.fround(Math.fround(Math.fround(y) - field.origin[1]) * inverse))),
    ) >> shift;
  const header =
    field.index[level * SURFACE_LEVEL_WORDS] +
    (cy * (((field.cols - 1) >> shift) + 1) + cx) * SURFACE_CELL_WORDS;
  for (let k = 0; k < 4; k++) out[k] = field.index[header + k];
  return out;
}

const _cell: [number, number, number, number] = [0, 0, 0, 0];

function segmentDistance(records: Float32Array, o: number): number {
  vec2.fromBuffer(_a, records, o);
  vec2.fromBuffer(_b, records, o + 2);
  segment2.closestPoint(_closest, _point, _a, _b);
  return vec2.distance(_point, _closest);
}

function insideTriangle(records: Float32Array, o: number): boolean {
  vec2.fromBuffer(_a, records, o);
  vec2.fromBuffer(_b, records, o + 2);
  vec2.fromBuffer(_c, records, o + 4);
  return triangle2.containsPoint(_a, _b, _c, _point);
}

function insideRect(records: Float32Array, o: number, x: number, y: number): number {
  return Math.min(
    Math.min(x - records[o], records[o + 2] - x),
    Math.min(y - records[o + 1], records[o + 3] - y),
  );
}

/** How far `(x, y)` lies inside the paving (negative outside): the deepest
 *  stroke, or the union of the polygons by their exposed boundary. */
export function pavedDistance(
  field: SurfaceField,
  x: number,
  y: number,
  footprintM: number,
): number {
  const { records, index } = field;
  surfaceCell(_cell, field, x, y, footprintM);
  vec2.set(_point, x, y);
  let paved = -1e9,
    inside = false,
    nearest = 1e9;
  for (let e = _cell[0]; e < _cell[1]; e++) {
    const o = (index[e] & SURFACE_RECORD_MASK) * SURFACE_FLOATS;
    const kind = index[e] >>> SURFACE_KIND_SHIFT;
    if (kind === SURFACE_STROKE) paved = Math.max(paved, strokeInside(records, o, x, y));
    else if (kind === SURFACE_TRIANGLE) inside ||= insideTriangle(records, o);
    else nearest = Math.min(nearest, segmentDistance(records, o));
  }
  return Math.max(paved, inside ? nearest : -nearest);
}

/** How far `(x, y)` lies inside the deepest forest shape (negative outside). */
export function forestDistance(
  field: SurfaceField,
  x: number,
  y: number,
  footprintM: number,
): number {
  const { records, index } = field;
  surfaceCell(_cell, field, x, y, footprintM);
  vec2.set(_point, x, y);
  let forest = -1e9,
    distance = -1e9,
    inside = false,
    nearest = 1e9;
  for (let e = _cell[1]; e < _cell[2]; e++) {
    const o = (index[e] & SURFACE_RECORD_MASK) * SURFACE_FLOATS;
    const kind = index[e] >>> SURFACE_KIND_SHIFT;
    if (index[e] & SURFACE_NEW_SHAPE) {
      forest = Math.max(forest, distance, inside ? nearest : -nearest);
      distance = -1e9;
      inside = false;
      nearest = 1e9;
    }
    if (kind === SURFACE_RECT) forest = Math.max(forest, insideRect(records, o, x, y));
    else if (kind === SURFACE_STROKE) distance = Math.max(distance, strokeInside(records, o, x, y));
    else if (kind === SURFACE_TRIANGLE) inside ||= insideTriangle(records, o);
    else nearest = Math.min(nearest, segmentDistance(records, o));
  }
  return Math.max(forest, distance, inside ? nearest : -nearest);
}

/** How far `(x, y)` lies inside the water's edge (negative outside): the
 *  deepest any river stretch puts it. */
export function waterDistance(
  field: SurfaceField,
  x: number,
  y: number,
  footprintM: number,
): number {
  const { records, index } = field;
  surfaceCell(_cell, field, x, y, footprintM);
  let bed = -1e9;
  for (let e = _cell[2]; e < _cell[3]; e++)
    bed = Math.max(
      bed,
      stretchInside(records, (index[e] & SURFACE_RECORD_MASK) * SURFACE_FLOATS, x, y),
    );
  return bed;
}
