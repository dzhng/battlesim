// What the camera keeps clear of: the ground, and boxes (the buildings a side
// knows stand) bucketed in a grid, so a query reads the few boxes near a point
// and never the map's whole table. Pure: the caller supplies the boxes and the
// ground, from public map geometry and what the viewing side has learned.
import { vec3, type Vec3 } from "math";
import { box3, raycast3 } from "math/shapes";

/** A box standing on the map: centred on `center`, turned by `yaw` about +Z,
 *  from `baseZ` up to `baseZ + 2 · half[2]`. */
export interface ObstacleBox {
  center: readonly [number, number];
  yaw: number;
  half: readonly [number, number, number];
  baseZ: number;
}

export interface CameraObstacles {
  /** How many boxes there are. */
  readonly count: number;
  /** Ground height under a point. */
  groundAt(x: number, y: number): number;
  /** The tallest box's top (−Infinity with no boxes): a point `clearance`
   *  above it meets no box. */
  readonly ceiling: number;
  /** Whether `point` stands `clearance` metres clear of the ground under it
   *  and of every box. A box is grown by `clearance` on each axis, so its
   *  edges and corners keep a little more than its faces do. */
  clear(point: Vec3, clearance: number): boolean;
  /** Whether the straight move from `a` to `b` stays `clearance` metres
   *  clear of every box (the ground is judged at the ends, by `clear`). */
  sweepClear(a: Vec3, b: Vec3, clearance: number): boolean;
  /** The top of the tallest box that move comes within `clearance` of
   *  (−Infinity when it is clear): how high a way over them runs. */
  sweepTop(a: Vec3, b: Vec3, clearance: number): number;
  /**
   * `point` pushed out of whatever it is not clear of, into `out`: up off the
   * ground, and out of each box through one of its four sides or its top,
   * whichever leaves it nearest `toward` (the point itself for the shortest
   * way out; where the camera was, to stay on the side it is on). A point
   * wedged between boxes goes over them, so the result is always clear.
   */
  pushOut(out: Vec3, point: Vec3, toward: Vec3, clearance: number): Vec3;
  /** Box tests run so far: the cost probe. */
  readonly tested: number;
}

/** Floats per box record: centre x, y; cos and sin of its yaw; half extents
 *  along and across it; base and top heights. */
const RECORD = 8;
/** Grid cell, metres: a few buildings a cell in a dense town. */
const CELL_M = 32;
/** Boxes one push leaves in turn before it goes over them instead. */
const PUSH_BOXES = 4;
/** How far past a grown box's face a pushed point lands, metres: beyond the
 *  rounding of the turn into the box's frame and back. */
const PUSH_SKIN = 1e-6;

const _local_a = vec3.create();
const _local_dir = vec3.create();
const _local_toward = vec3.create();
const _grown = box3.create();
const _no_dir = vec3.create();
const _sweep_dir = vec3.create();

/** An obstacle view over `boxes` and the ground `groundAt` gives. */
export function createCameraObstacles(
  boxes: readonly ObstacleBox[],
  groundAt: (x: number, y: number) => number,
): CameraObstacles {
  const records = new Float64Array(boxes.length * RECORD);
  // Each box's own bounds on the ground plane: min x, min y, max x, max y.
  const bounds = new Float64Array(boxes.length * 4);
  let [minX, minY, maxX, maxY] = [Infinity, Infinity, -Infinity, -Infinity];
  let ceiling = -Infinity;
  boxes.forEach((box, i) => {
    const [cos, sin] = [Math.cos(box.yaw), Math.sin(box.yaw)];
    const [hx, hy, hz] = box.half;
    const top = box.baseZ + 2 * hz;
    records.set([box.center[0], box.center[1], cos, sin, hx, hy, box.baseZ, top], i * RECORD);
    const ex = Math.abs(cos) * hx + Math.abs(sin) * hy;
    const ey = Math.abs(sin) * hx + Math.abs(cos) * hy;
    bounds.set(
      [box.center[0] - ex, box.center[1] - ey, box.center[0] + ex, box.center[1] + ey],
      i * 4,
    );
    minX = Math.min(minX, box.center[0] - ex);
    minY = Math.min(minY, box.center[1] - ey);
    maxX = Math.max(maxX, box.center[0] + ex);
    maxY = Math.max(maxY, box.center[1] + ey);
    ceiling = Math.max(ceiling, top);
  });
  const cols = boxes.length ? Math.floor((maxX - minX) / CELL_M) + 1 : 0;
  const rows = boxes.length ? Math.floor((maxY - minY) / CELL_M) + 1 : 0;
  const col = (x: number) => Math.min(cols - 1, Math.max(0, Math.floor((x - minX) / CELL_M)));
  const row = (y: number) => Math.min(rows - 1, Math.max(0, Math.floor((y - minY) / CELL_M)));
  // Cell lists, compressed: cell c's boxes are items[starts[c] .. starts[c + 1]].
  const starts = new Uint32Array(cols * rows + 1);
  const eachCell = (i: number, visit: (cell: number) => void) => {
    for (let r = row(bounds[i * 4 + 1]); r <= row(bounds[i * 4 + 3]); r++)
      for (let c = col(bounds[i * 4]); c <= col(bounds[i * 4 + 2]); c++) visit(r * cols + c);
  };
  for (let i = 0; i < boxes.length; i++) eachCell(i, (cell) => starts[cell + 1]++);
  for (let c = 0; c < cols * rows; c++) starts[c + 1] += starts[c];
  const items = new Uint32Array(starts[cols * rows]);
  const filled = starts.slice(0, cols * rows);
  for (let i = 0; i < boxes.length; i++) eachCell(i, (cell) => (items[filled[cell]++] = i));

  let tested = 0;
  /** `p` in the frame of the box whose record starts at `o`, where the box
   *  is axis-aligned about its centre (heights stay the world's). */
  const toLocal = (out: Vec3, o: number, x: number, y: number, z: number): Vec3 => {
    const [cos, sin] = [records[o + 2], records[o + 3]];
    const [dx, dy] = [x - records[o], y - records[o + 1]];
    return vec3.set(out, dx * cos + dy * sin, dy * cos - dx * sin, z);
  };
  /** The top of the tallest box the last `boxMet` met. */
  let tallest = -Infinity;
  /** The record of a box the move from `a` along unit `dir` for `length`
   *  metres (0: the point `a`) comes within `clearance` of, or −1: the first
   *  found, or with `all` the last, having looked at every one. */
  const boxMet = (a: Vec3, dir: Vec3, length: number, clearance: number, all = false): number => {
    const bx = a[0] + dir[0] * length;
    const by = a[1] + dir[1] * length;
    let met = -1;
    tallest = -Infinity;
    if (Math.min(a[2], a[2] + dir[2] * length) - clearance >= ceiling) return -1;
    const c1 = col(Math.max(a[0], bx) + clearance);
    const r1 = row(Math.max(a[1], by) + clearance);
    for (let r = row(Math.min(a[1], by) - clearance); r <= r1; r++)
      for (let c = col(Math.min(a[0], bx) - clearance); c <= c1; c++)
        for (let k = starts[r * cols + c]; k < starts[r * cols + c + 1]; k++) {
          const o = items[k] * RECORD;
          tested++;
          toLocal(_local_a, o, a[0], a[1], a[2]);
          box3.set(
            _grown,
            -records[o + 4] - clearance,
            -records[o + 5] - clearance,
            records[o + 6] - clearance,
            records[o + 4] + clearance,
            records[o + 5] + clearance,
            records[o + 7] + clearance,
          );
          // The direction turns with the frame; it has no position.
          if (length > 0)
            toLocal(_local_dir, o, records[o] + dir[0], records[o + 1] + dir[1], dir[2]);
          const meets =
            length > 0
              ? raycast3.intersectsBox3(_local_a, _local_dir, length, _grown)
              : box3.containsPoint(_grown, _local_a);
          if (!meets) continue;
          met = o;
          tallest = Math.max(tallest, records[o + 7]);
          if (!all) return met;
        }
    return met;
  };
  /** `boxMet` along the straight move from `a` to `b`. */
  const moveMet = (a: Vec3, b: Vec3, clearance: number, all: boolean): number => {
    const length = vec3.distance(a, b);
    // Too short to have a direction: the point itself.
    if (length < 1e-9) return boxMet(a, _no_dir, 0, clearance, all);
    vec3.scale(_sweep_dir, vec3.subtract(_sweep_dir, b, a), 1 / length);
    return boxMet(a, _sweep_dir, length, clearance, all);
  };
  const offGround = (p: Vec3, clearance: number) => p[2] - groundAt(p[0], p[1]) >= clearance;
  /** `out` moved out of the box at record `o` (grown by `clearance`) through
   *  the side or top whose exit lies nearest `toward`. */
  const leave = (out: Vec3, o: number, toward: Vec3, clearance: number): void => {
    const hx = records[o + 4] + clearance + PUSH_SKIN;
    const hy = records[o + 5] + clearance + PUSH_SKIN;
    const top = records[o + 7] + clearance + PUSH_SKIN;
    const p = toLocal(_local_a, o, out[0], out[1], out[2]);
    const t = toLocal(_local_toward, o, toward[0], toward[1], toward[2]);
    // Leaving through a face moves one coordinate onto it; the other two
    // stay. Each exit's squared distance from `toward`, less the part all share.
    const [dx, dy, dz] = [(p[0] - t[0]) ** 2, (p[1] - t[1]) ** 2, (p[2] - t[2]) ** 2];
    const west = (-hx - t[0]) ** 2 - dx;
    const east = (hx - t[0]) ** 2 - dx;
    const south = (-hy - t[1]) ** 2 - dy;
    const north = (hy - t[1]) ** 2 - dy;
    const over = (top - t[2]) ** 2 - dz;
    const least = Math.min(west, east, south, north, over);
    if (least === over) p[2] = top;
    else if (least === west || least === east) p[0] = least === west ? -hx : hx;
    else p[1] = least === south ? -hy : hy;
    const [cos, sin] = [records[o + 2], records[o + 3]];
    vec3.set(
      out,
      records[o] + p[0] * cos - p[1] * sin,
      records[o + 1] + p[0] * sin + p[1] * cos,
      p[2],
    );
  };

  return {
    count: boxes.length,
    groundAt,
    ceiling,
    clear: (point, clearance) =>
      offGround(point, clearance) && boxMet(point, _no_dir, 0, clearance) < 0,
    sweepClear: (a, b, clearance) => moveMet(a, b, clearance, false) < 0,
    sweepTop(a, b, clearance) {
      moveMet(a, b, clearance, true);
      return tallest;
    },
    pushOut(out, point, toward, clearance) {
      vec3.copy(out, point);
      let over = -Infinity;
      for (let left = PUSH_BOXES; ; left--) {
        if (!offGround(out, clearance)) out[2] = groundAt(out[0], out[1]) + clearance;
        const met = boxMet(out, _no_dir, 0, clearance);
        if (met < 0) return out;
        over = Math.max(over, tallest);
        if (left > 0) leave(out, met, toward, clearance);
        // Wedged between boxes: over the tallest met so far, and at last
        // over every box there is.
        else out[2] = (left > -PUSH_BOXES ? over : ceiling) + clearance + PUSH_SKIN;
      }
    },
    get tested() {
      return tested;
    },
  };
}
