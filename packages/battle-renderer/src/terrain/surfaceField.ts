// The surface field: every ground rule's signed distance (paved, forest,
// water) from one bucket index, so a lookup costs what lies near the point
// instead of what the map holds.
//
// The rules stay the simulation's exported primitives, read with the distance
// functions the terrain material always used; nothing is baked or resampled,
// and no class or coverage mask stands in for a distance. The field only says
// which primitives a point has to look at:
//
// - `records` holds every primitive once, eight floats each;
// - `index` holds a ladder of grids over the ground. A cell lists the records
//   whose shape, grown by the level's reach, touches the cell: paved, then
//   forest, then water.
//
// A consumer reads a distance only out to a reach that grows with the pixel
// (a road's feather is a pixel wide at least), so each level serves pixels up
// to twice the footprint of the one below, with cells that grow with it.
// Within a level's reach a lookup equals the all-primitives distance exactly;
// beyond it the value keeps its side (deeper inside, or farther outside, than
// the reach).
//
// The ladder ends in one of two ways. On a sparse map it ends at a level of
// one cell that lists everything, so a pixel of any width reads exactly. On a
// dense map it ends at the last level whose cells stay within the list
// budget: a wider pixel (ground seen edge-on, hundreds of metres off) reads
// that level, and so no farther than its reach.
//
// `terrainMaterial.ts` uploads both tables and mirrors `surfaceCell` and the
// three distances in WGSL; the functions here are that lookup on the CPU, held
// to the all-primitives distance by `surfaceField.test.ts`.
import { vec2, type Vec2 } from "math";
import { polygon2, segment2, triangle2 } from "math/shapes";
import {
  FOREST_BOUNDARY_FLOATS,
  FOREST_STROKE_FLOATS,
  FOREST_TRIANGLE_FLOATS,
} from "./forestShapes";
import { RECT_FLOATS, type TerrainSite } from "./terrainSurface";

/** Floats per record: a stroke `a, b, half width`; a triangle `a, b, c`; an
 *  exposed boundary edge `a, b`; a rect `min, max`. */
export const SURFACE_FLOATS = 8;

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

/** The finest cells' side in metres, and the widest pixel the finest level
 *  serves: the play camera looking down sees at most about 2.5 m a pixel (the
 *  next level), so only ground seen edge-on climbs far. Measured in
 *  `specs/city-maps/slices/C63-surface-distance-field.md`. */
export const SURFACE_CELL_M = 8;
export const SURFACE_FOOTPRINT_M = 2;
/** A level's cells are at least this many of its footprints a side: finer
 *  cells would each list nearly the same records. */
export const SURFACE_CELL_FOOTPRINTS = 4;
/** The finest level never holds more cells than this; a larger map doubles
 *  its cell instead, so the index is bounded by this and not by the map. */
export const SURFACE_MAX_CELLS = 1 << 18;
/** The ladder stops before a level whose cells list more records on average
 *  than `surfaceListBudget` allows: no pixel, however wide, pays for the whole
 *  of a dense map. This is the finest level's allowance. */
export const SURFACE_LIST_BUDGET = 24;

/** The mean records a cell of `level` may list. Wider pixels are fewer: in a
 *  ground view the rows whose pixels are `F` metres wide thin as 1 / sqrt(F),
 *  so each level may list sqrt(2) times the one below for the same cost a
 *  frame (a few hundred records a screen column, against the thousand rows
 *  below the horizon that read one or two each). */
export function surfaceListBudget(level: number): number {
  return SURFACE_LIST_BUDGET * Math.SQRT2 ** level;
}
/** Cells are grown by this share of the largest coordinate when records are
 *  listed, so a point the GPU's f32 arithmetic puts a hair into the next
 *  cell still finds its primitives there. 64 f32 steps of that coordinate. */
const CELL_PAD = 2 ** -18;

/** How far outside (and inside) its edge each rule is read, in metres. */
export interface SurfaceReach {
  paved: number;
  forest: number;
  water: number;
}

export interface SurfaceField {
  /** `SURFACE_FLOATS` per record: paved strokes, triangles and boundary
   *  edges; each non-rectangle forest's strokes, triangles and edges; forest
   *  rects; water rects. */
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

const PAVED = 0;
const FOREST = 1;
const WATER = 2;

const _box: number[] = [0, 0, 0, 0, 0, 0, 0, 0];
const _triangle: number[] = [0, 0, 0, 0, 0, 0];
const _rect: number[] = [0, 0, 0, 0];
const _a: Vec2 = [0, 0],
  _b: Vec2 = [0, 0],
  _c: Vec2 = [0, 0],
  _point: Vec2 = [0, 0],
  _closest: Vec2 = [0, 0];

/** Whether the closed segment `a`-`b` touches the closed box: the segment
 *  clipped to the box's two slabs is not empty. `polygon2.intersectsSegment`
 *  counts only proper crossings, and misses a segment that enters and leaves
 *  through the box's corners: every diagonal street on a square grid. */
function segmentTouchesBox(
  a: Vec2,
  b: Vec2,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
): boolean {
  let enter = 0,
    leave = 1;
  for (let axis = 0; axis < 2; axis++) {
    const low = axis === 0 ? x0 : y0,
      high = axis === 0 ? x1 : y1;
    const run = b[axis] - a[axis];
    if (run === 0) {
      if (a[axis] < low || a[axis] > high) return false;
      continue;
    }
    const u = (low - a[axis]) / run,
      v = (high - a[axis]) / run;
    enter = Math.max(enter, Math.min(u, v));
    leave = Math.min(leave, Math.max(u, v));
  }
  return enter <= leave;
}

/** The field of `site`'s ground rules. `reach` says how far each rule is read
 *  at a pixel `footprintM` wide; it must not shrink as the pixel grows. */
export function buildSurfaceField(
  site: TerrainSite,
  reach: (footprintM: number) => SurfaceReach,
): SurfaceField {
  const boundaryCount = site.surfaceBoundaries.length / site.surfaceBoundaryStride;
  let count =
    site.surfaceStrokes.length / site.surfaceStrokeStride +
    site.surfaceTriangles.length / site.surfaceTriangleStride +
    boundaryCount;
  for (const shape of site.forestShapes)
    if (shape.kind !== "rectangle")
      count +=
        shape.strokes.length / FOREST_STROKE_FLOATS +
        shape.triangles.length / FOREST_TRIANGLE_FLOATS +
        shape.boundaries.length / FOREST_BOUNDARY_FLOATS;
  count += (site.forests.length + site.water.length) / RECT_FLOATS;
  if (count > SURFACE_RECORD_MASK) throw new Error("surface field: too many ground primitives");

  // A storage binding cannot be empty: the table holds a record at least.
  const records = new Float32Array(Math.max(1, count) * SURFACE_FLOATS);
  const kinds = new Uint8Array(count);
  const rules = new Uint8Array(count);
  /** The forest shape a record belongs to; -1 for the rest. */
  const shapes = new Int32Array(count).fill(-1);
  /** Whether cells list the record at all. */
  const listed = new Uint8Array(count).fill(1);
  let minX = site.map[0],
    minY = site.map[1],
    maxX = site.map[2],
    maxY = site.map[3];
  let next = 0;
  /** Append `floats` of `source` as a record whose first `points` pairs are
   *  positions; returns the record. */
  const put = (
    source: ArrayLike<number>,
    offset: number,
    floats: number,
    kind: number,
    rule: number,
    points: number,
  ) => {
    const o = next * SURFACE_FLOATS;
    for (let k = 0; k < floats; k++) records[o + k] = source[offset + k];
    for (let k = 0; k < points; k++) {
      minX = Math.min(minX, records[o + k * 2]);
      maxX = Math.max(maxX, records[o + k * 2]);
      minY = Math.min(minY, records[o + k * 2 + 1]);
      maxY = Math.max(maxY, records[o + k * 2 + 1]);
    }
    kinds[next] = kind;
    rules[next] = rule;
    return next++;
  };
  for (let o = 0; o < site.surfaceStrokes.length; o += site.surfaceStrokeStride)
    put(site.surfaceStrokes, o, FOREST_STROKE_FLOATS, SURFACE_STROKE, PAVED, 2);
  // Triangles only say which side of the exposed boundary a point is on:
  // with no boundary there is nothing for them to sign.
  for (let o = 0; o < site.surfaceTriangles.length; o += site.surfaceTriangleStride)
    listed[put(site.surfaceTriangles, o, FOREST_TRIANGLE_FLOATS, SURFACE_TRIANGLE, PAVED, 3)] =
      boundaryCount > 0 ? 1 : 0;
  for (let o = 0; o < site.surfaceBoundaries.length; o += site.surfaceBoundaryStride)
    put(site.surfaceBoundaries, o, FOREST_BOUNDARY_FLOATS, SURFACE_EDGE, PAVED, 2);
  let shapeId = 0;
  for (const shape of site.forestShapes) {
    if (shape.kind === "rectangle") continue;
    for (let o = 0; o < shape.strokes.length; o += FOREST_STROKE_FLOATS)
      shapes[put(shape.strokes, o, FOREST_STROKE_FLOATS, SURFACE_STROKE, FOREST, 2)] = shapeId;
    for (let o = 0; o < shape.triangles.length; o += FOREST_TRIANGLE_FLOATS) {
      const at = put(shape.triangles, o, FOREST_TRIANGLE_FLOATS, SURFACE_TRIANGLE, FOREST, 3);
      shapes[at] = shapeId;
      listed[at] = shape.boundaries.length > 0 ? 1 : 0;
    }
    for (let o = 0; o < shape.boundaries.length; o += FOREST_BOUNDARY_FLOATS)
      shapes[put(shape.boundaries, o, FOREST_BOUNDARY_FLOATS, SURFACE_EDGE, FOREST, 2)] = shapeId;
    shapeId++;
  }
  [site.forests, site.water].forEach((list, k) => {
    for (let o = 0; o < list.length; o += RECT_FLOATS) {
      _rect[0] = list[o];
      _rect[1] = list[o + 1];
      _rect[2] = list[o] + list[o + 2];
      _rect[3] = list[o + 1] + list[o + 3];
      put(_rect, 0, 4, SURFACE_RECT, k === 0 ? FOREST : WATER, 2);
    }
  });

  // The grid covers the map and every primitive, so a point past it is
  // nearer the border cell it clamps to than to anything listed elsewhere.
  const originX = Math.fround(minX),
    originY = Math.fround(minY);
  let cellM = SURFACE_CELL_M;
  const across = (span: number, cell: number) => Math.max(1, Math.ceil(span / cell));
  while (across(maxX - originX, cellM) * across(maxY - originY, cellM) > SURFACE_MAX_CELLS)
    cellM *= 2;
  const cols = across(maxX - originX, cellM),
    rows = across(maxY - originY, cellM);
  const pad =
    Math.max(Math.abs(originX), Math.abs(originY), Math.abs(maxX), Math.abs(maxY), 1) * CELL_PAD;

  /** How many times `level`'s cells double the finest. */
  const doublings = (level: number) => {
    let shift = 0;
    while (cellM * 2 ** shift < SURFACE_CELL_FOOTPRINTS * SURFACE_FOOTPRINT_M * 2 ** level) shift++;
    return shift;
  };
  /** Every (cell slot, record) of `level`: the records a point in the cell
   *  may have to read. */
  const visit = (level: number, each: (slot: number, record: number) => void) => {
    const shift = doublings(level);
    const cell = cellM * 2 ** shift;
    const levelCols = ((cols - 1) >> shift) + 1,
      levelRows = ((rows - 1) >> shift) + 1;
    const whole = levelCols === 1 && levelRows === 1;
    const r = reach(SURFACE_FOOTPRINT_M * 2 ** level);
    const reaches = whole ? [Infinity, Infinity, Infinity] : [r.paved, r.forest, r.water];
    const column = (x: number) =>
      Math.min(levelCols - 1, Math.max(0, Math.floor((x - originX) / cell)));
    const row = (y: number) =>
      Math.min(levelRows - 1, Math.max(0, Math.floor((y - originY) / cell)));
    for (let record = 0; record < count; record++) {
      if (!listed[record]) continue;
      const o = record * SURFACE_FLOATS;
      const kind = kinds[record],
        rule = rules[record];
      // A triangle is membership: only the cells it touches. A stroke is read
      // out to its half width and the reach; an edge and a rect to the reach.
      const grow =
        (kind === SURFACE_TRIANGLE
          ? 0
          : reaches[rule] + (kind === SURFACE_STROKE ? records[o + 4] : 0)) + pad;
      let lowX = records[o],
        lowY = records[o + 1],
        highX = records[o + 2],
        highY = records[o + 3];
      if (kind !== SURFACE_RECT) {
        lowX = Math.min(records[o], records[o + 2]);
        highX = Math.max(records[o], records[o + 2]);
        lowY = Math.min(records[o + 1], records[o + 3]);
        highY = Math.max(records[o + 1], records[o + 3]);
      }
      if (kind === SURFACE_TRIANGLE) {
        lowX = Math.min(lowX, records[o + 4]);
        highX = Math.max(highX, records[o + 4]);
        lowY = Math.min(lowY, records[o + 5]);
        highY = Math.max(highY, records[o + 5]);
        for (let k = 0; k < 6; k++) _triangle[k] = records[o + k];
      } else if (kind !== SURFACE_RECT) {
        vec2.fromBuffer(_a, records, o);
        vec2.fromBuffer(_b, records, o + 2);
      }
      const c0 = column(lowX - grow),
        c1 = column(highX + grow),
        r0 = row(lowY - grow),
        r1 = row(highY + grow);
      for (let cy = r0; cy <= r1; cy++)
        for (let cx = c0; cx <= c1; cx++) {
          if (kind !== SURFACE_RECT && !whole) {
            // The cell grown by the reach, as a square: a little more than the
            // exact rounded box, never less.
            const x0 = originX + cx * cell - grow,
              y0 = originY + cy * cell - grow,
              x1 = originX + (cx + 1) * cell + grow,
              y1 = originY + (cy + 1) * cell + grow;
            if (kind === SURFACE_TRIANGLE) {
              _box[0] = x0;
              _box[1] = y0;
              _box[2] = x1;
              _box[3] = y0;
              _box[4] = x1;
              _box[5] = y1;
              _box[6] = x0;
              _box[7] = y1;
              if (!polygon2.overlapConvex(_triangle, 3, _box, 4)) continue;
            } else if (!segmentTouchesBox(_a, _b, x0, y0, x1, y1)) continue;
          }
          each((cy * levelCols + cx) * SURFACE_CELL_WORDS + rule, record);
        }
    }
  };

  const counts: Uint32Array[] = [];
  let words = 0;
  let exactToM = Infinity;
  for (let level = 0; ; level++) {
    const shift = doublings(level);
    const cells = (((cols - 1) >> shift) + 1) * (((rows - 1) >> shift) + 1);
    const levelCounts = new Uint32Array(cells * SURFACE_CELL_WORDS);
    let entries = 0;
    visit(level, (slot) => {
      levelCounts[slot]++;
      entries++;
    });
    if (level > 0 && entries > cells * surfaceListBudget(level)) {
      exactToM = SURFACE_FOOTPRINT_M * 2 ** (level - 1);
      break;
    }
    counts.push(levelCounts);
    words += levelCounts.length + 1 + entries;
    if (cells === 1) break;
  }
  const levels = counts.length;
  const index = new Uint32Array(levels * SURFACE_LEVEL_WORDS + words);
  let at = levels * SURFACE_LEVEL_WORDS;
  for (let level = 0; level < levels; level++) {
    const cursors = counts[level];
    const header = at;
    index[level * SURFACE_LEVEL_WORDS] = header;
    index[level * SURFACE_LEVEL_WORDS + 1] = doublings(level);
    let cursor = header + cursors.length + 1;
    for (let slot = 0; slot < cursors.length; slot++) {
      const size = cursors[slot];
      index[header + slot] = cursor;
      cursors[slot] = cursor;
      cursor += size;
    }
    index[header + cursors.length] = cursor;
    visit(level, (slot, record) => {
      index[cursors[slot]++] = (kinds[record] << SURFACE_KIND_SHIFT) | record;
    });
    // Records are in shape order, so a cell's forest list runs shape by
    // shape: mark where each later shape starts.
    for (let slot = FOREST; slot < cursors.length; slot += SURFACE_CELL_WORDS) {
      let shape = -1;
      for (let e = index[header + slot]; e < index[header + slot + 1]; e++) {
        const own = shapes[index[e] & SURFACE_RECORD_MASK];
        if (own < 0) continue;
        if (shape >= 0 && own !== shape) index[e] |= SURFACE_NEW_SHAPE;
        shape = own;
      }
    }
    at = cursor;
  }
  return {
    records,
    index,
    origin: [originX, originY],
    cellM,
    cols,
    rows,
    footprintM: SURFACE_FOOTPRINT_M,
    levels,
    exactToM,
  };
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
    if (kind === SURFACE_STROKE)
      paved = Math.max(paved, records[o + 4] - segmentDistance(records, o));
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
    else if (kind === SURFACE_STROKE)
      distance = Math.max(distance, records[o + 4] - segmentDistance(records, o));
    else if (kind === SURFACE_TRIANGLE) inside ||= insideTriangle(records, o);
    else nearest = Math.min(nearest, segmentDistance(records, o));
  }
  return Math.max(forest, distance, inside ? nearest : -nearest);
}

/** How far `(x, y)` lies inside the deepest water rect (negative outside). */
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
      insideRect(records, (index[e] & SURFACE_RECORD_MASK) * SURFACE_FLOATS, x, y),
    );
  return bed;
}
