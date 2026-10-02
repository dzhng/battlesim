// The forest floor's dressing as the scenery layer keeps it. A map may hold
// hundreds of hectares of forest and thousands of pieces to the hectare, so
// the dressing is never expanded whole: the ground is dressed a cell at a
// time (`DressingField`), and only the cells the camera can see pieces in
// are laid, into the slots of one pool of instance records. A cell draws
// whole at one tier, straight from its slot: laying a cell is the only work a
// piece ever costs the CPU.
//
// - A cell is wanted while it is in view and its tallest possible piece is
//   still drawn: a piece shrinks to nothing on the GPU as its projected height
//   falls to `DRESSING_GONE` of `fade_px`, so past that distance a cell is
//   left out.
// - Cells are laid nearest first, a few a view (`budget`): after a cut the
//   far ones arrive over the next frames (`pending` says so, and a capture
//   waits for it).
// - The pool is bounded: when every slot is held, the cell least recently
//   wanted gives its up; when every slot is wanted, the farthest cells go
//   undrawn.
// - Ground the side has seen cleared holds no dressing: a piece there is laid
//   with no size, and when the cleared ground changes the laid cells are laid
//   again as their turn comes.
import { box3, frustum, type Box3 } from "math/shapes";
import { distanceTo } from "../frame/staticChunks";
import { INSTANCE_FLOATS, TIER_COUNT, tierFor, treeInstances, type TierView } from "./lod";
import { cellKey, TREE_FIELD, TREE_FLOATS, type DressingField, type KindSize } from "./placement";

/** A piece of dressing is gone at this share of the projected height it
 *  starts to shrink at (`forest_floor.dressing.fade_px`). */
export const DRESSING_GONE = 0.5;
/** A cell's slot before it is laid, and once it is laid and holds nothing. */
const NOT_LAID = -1;
const EMPTY = -2;

export interface DressedCell {
  i: number;
  j: number;
  /** All it can draw: the cell, the ground's range of heights under it and
   *  its tallest piece above. */
  box: Box3;
  /** Its slot in the pool, `NOT_LAID` or `EMPTY`. */
  slot: number;
  /** Per kind of the field's `kinds`, where its records start in the slot;
   *  one more entry closes the last. */
  starts: number[];
  /** The selection that last wanted it, and how far off it was. */
  stamp: number;
  distance: number;
  /** Laid before the cleared ground last changed. */
  stale: boolean;
}

export interface DressingCache {
  field: DressingField;
  sizes: readonly KindSize[];
  /** `forest_floor.dressing.fade_px`. */
  fadePx: number;
  cells: Map<number, DressedCell>;
  /** Per slot: the cell laid in it. */
  held: (DressedCell | null)[];
  /** Per tier: the cells the last selection draws. */
  drawn: DressedCell[][];
  /** Wanted cells are still to be laid: select again. */
  pending: boolean;
  stamp: number;
  /** Whether the side has seen the ground at (x, y) cleared. */
  cleared: ((x: number, y: number) => boolean) | null;
  wanted: DressedCell[];
}

/** A cache of `slots` cells over `field`, none laid. */
export function createDressingCache(
  field: DressingField,
  sizes: readonly KindSize[],
  fadePx: number,
  slots: number,
): DressingCache {
  const cells = new Map<number, DressedCell>();
  const m = field.cellM;
  // A piece stands in its cell and reaches a little past it.
  const past = field.reachM;
  for (let o = 0; o < field.cells.length; o += 2) {
    const [i, j] = [field.cells[o], field.cells[o + 1]];
    cells.set(cellKey(i, j), {
      i,
      j,
      box: box3.set(
        box3.create(),
        i * m - past,
        j * m - past,
        field.ground[0],
        (i + 1) * m + past,
        (j + 1) * m + past,
        field.ground[1] + field.topM,
      ),
      slot: NOT_LAID,
      starts: field.kinds.map(() => 0).concat(0),
      stamp: 0,
      distance: 0,
      stale: false,
    });
  }
  return {
    field,
    sizes,
    fadePx,
    cells,
    held: Array.from({ length: slots }, () => null),
    drawn: Array.from({ length: TIER_COUNT }, () => []),
    pending: false,
    stamp: 0,
    cleared: null,
    wanted: [],
  };
}

/** How far off the field's tallest piece is gone, for `view`. */
export function dressingReach(cache: DressingCache, view: TierView): number {
  return (cache.field.topM * view.pixelsPerMetre) / (cache.fadePx * DRESSING_GONE);
}

/** The side's cleared ground changed (or is `null`: none is known): every
 *  laid cell is laid again when next wanted. */
export function setDressingCleared(cache: DressingCache, cleared: DressingCache["cleared"]): void {
  cache.cleared = cleared;
  for (const cell of cache.held) if (cell) cell.stale = true;
  cache.pending = true;
}

/** A slot for a cell wanted now: a free one, else the one whose cell was
 *  wanted longest ago; -1 when every slot is wanted by this selection. */
function takeSlot(cache: DressingCache): number {
  let oldest = -1;
  for (let s = 0; s < cache.held.length; s++) {
    const cell = cache.held[s];
    if (!cell) return s;
    if (cell.stamp < cache.stamp && (oldest < 0 || cell.stamp < cache.held[oldest]!.stamp))
      oldest = s;
  }
  if (oldest >= 0) cache.held[oldest]!.slot = NOT_LAID;
  return oldest;
}

/** A cell's pieces as the GPU's records, kind by kind in the field's order,
 *  with where each kind starts written to `cell.starts`: a tree's record,
 *  but its last float is the piece's height over `fade_px`, which the layer's
 *  vertex stage turns into the piece's size on screen over the size it starts
 *  to shrink at. A piece on cleared ground has no size. */
function cellRecords(cache: DressingCache, cell: DressedCell): Float32Array {
  const { field, cleared } = cache;
  const placed = field.place(cell.i, cell.j);
  const instances = treeInstances(placed, cache.sizes);
  const count = instances.kinds.length;
  cell.starts.fill(0);
  for (let p = 0; p < count; p++) cell.starts[field.kinds.indexOf(instances.kinds[p]) + 1]++;
  for (let n = 1; n < cell.starts.length; n++) cell.starts[n] += cell.starts[n - 1];
  const next = cell.starts.slice();
  const out = new Float32Array(count * INSTANCE_FLOATS);
  for (let p = 0; p < count; p++) {
    const from = p * INSTANCE_FLOATS;
    const to = next[field.kinds.indexOf(instances.kinds[p])]++ * INSTANCE_FLOATS;
    out.set(instances.records.subarray(from, from + INSTANCE_FLOATS), to);
    out[to + INSTANCE_FLOATS - 1] = instances.heights[p] / cache.fadePx;
    const [x, y] = [placed[p * TREE_FLOATS + TREE_FIELD.x], placed[p * TREE_FLOATS + TREE_FIELD.y]];
    // The scale: with none, the piece is no triangle at all.
    if (cleared?.(x, y)) out.fill(0, to + 4, to + 7);
  }
  return out;
}

/** Choose what `view` draws of the dressing (its `lodPx` the dressing's):
 *  `cache.drawn`, per tier. Up to `budget` wanted cells are laid, nearest
 *  first, each through `write(slot, records)` (the pool's records from
 *  `slot * field.capacity`); `cache.pending` says more are waiting. */
export function selectDressing(
  cache: DressingCache,
  view: TierView,
  budget: number,
  write: (slot: number, records: Float32Array) => void,
): void {
  const { field, wanted } = cache;
  cache.stamp++;
  cache.pending = false;
  for (const list of cache.drawn) list.length = 0;
  wanted.length = 0;
  const reach = dressingReach(cache, view);
  const m = field.cellM;
  const [ex, ey] = [view.eye[0], view.eye[1]];
  for (let j = Math.floor((ey - reach) / m); j <= Math.floor((ey + reach) / m); j++)
    for (let i = Math.floor((ex - reach) / m); i <= Math.floor((ex + reach) / m); i++) {
      const cell = cache.cells.get(cellKey(i, j));
      if (!cell || cell.slot === EMPTY) continue;
      if (!frustum.sidesIntersectsBox3(view.sides, cell.box)) continue;
      cell.distance = distanceTo(cell.box, view.eye);
      if (cell.distance >= reach) continue;
      cell.stamp = cache.stamp;
      wanted.push(cell);
    }
  wanted.sort((a, b) => a.distance - b.distance);
  for (const cell of wanted) {
    if (cell.slot === NOT_LAID || cell.stale) {
      if (budget <= 0) cache.pending = true;
      else {
        const slot = cell.slot === NOT_LAID ? takeSlot(cache) : cell.slot;
        // Every slot is wanted by a nearer cell: this one goes undrawn.
        if (slot < 0) continue;
        budget--;
        const records = cellRecords(cache, cell);
        cell.stale = false;
        cell.slot = records.length ? slot : EMPTY;
        cache.held[slot] = records.length ? cell : null;
        if (records.length) write(slot, records);
      }
    }
    if (cell.slot >= 0) cache.drawn[tierFor(field.topM, cell.distance, view)].push(cell);
  }
}

/** What is laid and what the last selection draws: the pieces held, and per
 *  tier the pieces drawn. */
export function dressingCounts(cache: DressingCache): { held: number; tiers: number[] } {
  const pieces = (cell: DressedCell) => cell.starts[cell.starts.length - 1];
  return {
    held: cache.held.reduce((sum, cell) => sum + (cell ? pieces(cell) : 0), 0),
    tiers: cache.drawn.map((cells) => cells.reduce((sum, cell) => sum + pieces(cell), 0)),
  };
}
