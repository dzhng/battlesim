// The placement pool: a fixed number of instance records that come and go
// with the camera (a resident chunk's kit modules), kept kind by kind. A
// kind's records are always one contiguous range of the pool, so a kind is one
// draw however many chunks put records in it, and the pool's contents are a
// GPU buffer's: `flushPool` names what changed since the last flush.
//
// A kind's range has room to grow. When it runs out, it moves to the free end
// of the pool with twice the room; when the free end runs out, the ranges are
// packed again, each with a quarter to spare. A record is removed by moving
// its kind's last record into its place. So adding and removing cost a record
// each, and a kind's range moves only when it outgrows its room. A caller
// keeps a handle per record, which stays good through every move.
//
// The pool is full when packing cannot give every range its spare quarter:
// a little under four fifths of its capacity in records. It then refuses the
// record and changes nothing.

/** The least room a kind's range is given, records. */
const MIN_ROOM = 16;
/** The room a packed range keeps beyond its records. */
const SPARE = 1.25;

export interface PlacementPool {
  /** Floats per record. */
  stride: number;
  /** Records the pool holds at most. */
  capacity: number;
  /** The records: a GPU buffer's contents, `capacity * stride` floats. */
  records: Float32Array;
  /** Per kind: where its range starts, and how many records it draws. */
  first: Int32Array;
  count: Int32Array;
  /** Records held, over every kind. */
  used: number;
  /** Per kind: the records its range has room for. */
  room: Int32Array;
  /** Where the free end starts: every range lies below it. */
  top: number;
  /** Packing's other buffer: the two swap. */
  spare: Float32Array;
  /** Per pool slot: the handle of the record there. */
  handleAt: Int32Array;
  /** Per handle: its record's kind and place in the kind's range. */
  kindOf: Int32Array;
  indexOf: Int32Array;
  /** Handles not in use: a stack. */
  freeHandles: Int32Array;
  freeCount: number;
  /** Per kind: the part of its range written since the last flush,
   *  `[dirtyFrom, dirtyTo)` within the range. */
  dirtyFrom: Int32Array;
  dirtyTo: Int32Array;
}

export function createPlacementPool(
  capacity: number,
  stride: number,
  kinds: number,
): PlacementPool {
  return {
    stride,
    capacity,
    records: new Float32Array(capacity * stride),
    first: new Int32Array(kinds),
    count: new Int32Array(kinds),
    used: 0,
    room: new Int32Array(kinds),
    top: 0,
    spare: new Float32Array(capacity * stride),
    handleAt: new Int32Array(capacity),
    kindOf: new Int32Array(capacity),
    indexOf: new Int32Array(capacity),
    freeHandles: Int32Array.from({ length: capacity }, (_, i) => capacity - 1 - i),
    freeCount: capacity,
    dirtyFrom: new Int32Array(kinds).fill(capacity),
    dirtyTo: new Int32Array(kinds),
  };
}

function markDirty(pool: PlacementPool, kind: number, from: number, to: number) {
  if (from < pool.dirtyFrom[kind]) pool.dirtyFrom[kind] = from;
  if (to > pool.dirtyTo[kind]) pool.dirtyTo[kind] = to;
}

/** Move `kind`'s range to `first`, with `room`. */
function moveRange(pool: PlacementPool, kind: number, first: number, room: number) {
  const { stride } = pool;
  const from = pool.first[kind];
  const count = pool.count[kind];
  pool.records.copyWithin(first * stride, from * stride, (from + count) * stride);
  pool.handleAt.copyWithin(first, from, from + count);
  pool.first[kind] = first;
  pool.room[kind] = room;
  markDirty(pool, kind, 0, count);
}

/** Pack every range from the pool's start, each with its spare room, `kind`
 *  with room for one record more. False, and nothing changed, when they do
 *  not fit. */
function pack(pool: PlacementPool, kind: number): boolean {
  const kinds = pool.count.length;
  let total = 0;
  for (let k = 0; k < kinds; k++) {
    const count = pool.count[k] + (k === kind ? 1 : 0);
    if (count) total += Math.max(MIN_ROOM, Math.ceil(count * SPARE));
  }
  if (total > pool.capacity) return false;
  const { stride, records, spare, handleAt } = pool;
  const handles = handleAt.slice();
  let at = 0;
  for (let k = 0; k < kinds; k++) {
    const count = pool.count[k];
    const held = count + (k === kind ? 1 : 0);
    const room = held ? Math.max(MIN_ROOM, Math.ceil(held * SPARE)) : 0;
    const from = pool.first[k];
    spare.set(records.subarray(from * stride, (from + count) * stride), at * stride);
    handleAt.set(handles.subarray(from, from + count), at);
    pool.first[k] = at;
    pool.room[k] = room;
    if (count) markDirty(pool, k, 0, count);
    at += room;
  }
  pool.records = spare;
  pool.spare = records;
  pool.top = at;
  return true;
}

/** Make room for one more record of `kind`. */
function grow(pool: PlacementPool, kind: number): boolean {
  const room = pool.room[kind];
  const wanted = Math.max(MIN_ROOM, room * 2);
  // The topmost range grows where it stands.
  if (room > 0 && pool.first[kind] + room === pool.top) {
    if (pool.top + wanted - room > pool.capacity) return pack(pool, kind);
    pool.room[kind] = wanted;
    pool.top += wanted - room;
    return true;
  }
  if (pool.top + wanted > pool.capacity) return pack(pool, kind);
  moveRange(pool, kind, pool.top, wanted);
  pool.top += wanted;
  return true;
}

/** Add a record of `kind`: `pool.stride` floats of `source` from `offset`.
 *  Returns its handle, or -1 when the pool is full (nothing changes). */
export function poolAppend(
  pool: PlacementPool,
  kind: number,
  source: Float32Array,
  offset: number,
): number {
  if (pool.count[kind] === pool.room[kind] && !grow(pool, kind)) return -1;
  const index = pool.count[kind]++;
  const slot = pool.first[kind] + index;
  const { stride } = pool;
  for (let f = 0; f < stride; f++) pool.records[slot * stride + f] = source[offset + f];
  const handle = pool.freeHandles[--pool.freeCount];
  pool.handleAt[slot] = handle;
  pool.kindOf[handle] = kind;
  pool.indexOf[handle] = index;
  pool.used++;
  markDirty(pool, kind, index, index + 1);
  return handle;
}

/** Remove the record `handle` names: its kind's last record takes its place. */
export function poolRemove(pool: PlacementPool, handle: number): void {
  const kind = pool.kindOf[handle];
  const index = pool.indexOf[handle];
  const last = --pool.count[kind];
  if (index !== last) {
    const { stride } = pool;
    const [to, from] = [pool.first[kind] + index, pool.first[kind] + last];
    pool.records.copyWithin(to * stride, from * stride, (from + 1) * stride);
    const moved = pool.handleAt[from];
    pool.handleAt[to] = moved;
    pool.indexOf[moved] = index;
    markDirty(pool, kind, index, index + 1);
  }
  pool.freeHandles[pool.freeCount++] = handle;
  pool.used--;
  // An emptied range at the free end gives its room back.
  if (last === 0 && pool.first[kind] + pool.room[kind] === pool.top) {
    pool.top = pool.first[kind];
    pool.room[kind] = 0;
  }
}

/** Hand `write` every run of records changed since the last flush, as
 *  `(first, count)` in pool slots, and forget them. Returns the records
 *  written. */
export function flushPool(
  pool: PlacementPool,
  write: (first: number, count: number) => void,
): number {
  let written = 0;
  for (let k = 0; k < pool.count.length; k++) {
    const from = pool.dirtyFrom[k];
    // A record removed since it was written is no longer drawn.
    const to = Math.min(pool.dirtyTo[k], pool.count[k]);
    pool.dirtyFrom[k] = pool.capacity;
    pool.dirtyTo[k] = 0;
    if (to <= from) continue;
    write(pool.first[k] + from, to - from);
    written += to - from;
  }
  return written;
}
