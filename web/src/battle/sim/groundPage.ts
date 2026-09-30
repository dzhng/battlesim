/** Lossless 256-cell pages. Runs carry exclusive end:u16 and five channel
 * bytes; dense pages retain the original five-byte cell stride. */
export interface GroundPage {
  data: Uint8Array;
  dense: boolean;
}
const CELLS = 256,
  STRIDE = 5,
  RUN = 7;
export function createGroundPage(): GroundPage {
  return { data: Uint8Array.of(0, 1, 0, 0, 0, 0, 0), dense: false };
}
function endAt(data: Uint8Array, k: number): number {
  return data[k * RUN] | (data[k * RUN + 1] << 8);
}
function wordAt(data: Uint8Array, at: number): number {
  return (data[at] | (data[at + 1] << 8) | (data[at + 2] << 16) | (data[at + 3] << 24)) >>> 0;
}
function valueAt(page: GroundPage, c: number): number {
  if (page.dense) return c * STRIDE;
  let lo = 0,
    hi = page.data.length / RUN;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (endAt(page.data, mid) <= c) lo = mid + 1;
    else hi = mid;
  }
  return lo * RUN + 2;
}
export function readGroundPage(page: GroundPage, c: number, out: Uint8Array, offset = 0): void {
  const at = valueAt(page, c);
  for (let k = 0; k < STRIDE; k++) out[offset + k] = page.data[at + k];
}
export function groundPageCleared(page: GroundPage, c: number): boolean {
  return page.data[valueAt(page, c) + 4] > 0;
}
export function groundPageSpans(
  page: GroundPage,
  visit: (start: number, end: number, word: number, cleared: number) => void,
): void {
  if (!page.dense) {
    let start = 0;
    for (let k = 0; k < page.data.length / RUN; k++) {
      const end = endAt(page.data, k);
      visit(start, end, wordAt(page.data, k * RUN + 2), page.data[k * RUN + 6]);
      start = end;
    }
    return;
  }
  let start = 0,
    word = wordAt(page.data, 0),
    cleared = page.data[4];
  for (let i = 1; i <= CELLS; i++) {
    const next = i < CELLS ? wordAt(page.data, i * STRIDE) : -1,
      clear = i < CELLS ? page.data[i * STRIDE + 4] : -1;
    if (next !== word || clear !== cleared) {
      visit(start, i, word, cleared);
      start = i;
      word = next;
      cleared = clear;
    }
  }
}
export function clearedGroundSpans(
  page: GroundPage,
  visit: (start: number, len: number) => void,
): void {
  let lo = -1,
    hi = 0;
  groundPageSpans(page, (start, end, _word, cleared) => {
    if (cleared) {
      if (lo < 0) lo = start;
      hi = end;
    } else if (lo >= 0) {
      visit(lo, hi - lo);
      lo = -1;
    }
  });
  if (lo >= 0) visit(lo, hi - lo);
}
function countCleared(page: GroundPage, start: number, end: number): number {
  if (page.dense) {
    let count = 0;
    for (let c = start; c < end; c++) if (page.data[c * STRIDE + 4]) count++;
    return count;
  }
  let count = 0;
  groundPageSpans(page, (lo, hi, _word, cleared) => {
    if (cleared) count += Math.max(0, Math.min(end, hi) - Math.max(start, lo));
  });
  return count;
}
function putWord(data: Uint8Array, at: number, word: number, cleared: number): void {
  for (let k = 0; k < 4; k++) data[at + k] = word >>> (k * 8);
  data[at + 4] = cleared;
}
/** Returns the change in the count of cleared cells. No dense expansion is
 * needed while accumulating uniform or short-run snapshots. */
function setGroundSpan(
  page: GroundPage,
  start: number,
  len: number,
  word: number,
  cleared: number,
): number {
  const stop = start + len,
    delta = (cleared ? len : 0) - countCleared(page, start, stop);
  if (start === 0 && len === CELLS) {
    const data = new Uint8Array(RUN);
    data[1] = 1;
    putWord(data, 2, word, cleared);
    page.data = data;
    page.dense = false;
    return delta;
  }
  if (page.dense) {
    for (let c = start; c < stop; c++) putWord(page.data, c * STRIDE, word, cleared);
    return delta;
  }
  const ends: number[] = [],
    words: number[] = [],
    clears: number[] = [];
  const append = (end: number, value: number, clear: number) => {
    const n = ends.length;
    if (n && words[n - 1] === value && clears[n - 1] === clear) {
      ends[n - 1] = end;
      return;
    }
    ends.push(end);
    words.push(value);
    clears.push(clear);
  };
  groundPageSpans(page, (lo, hi, value, clear) => {
    if (hi <= start || lo >= stop) append(hi, value, clear);
    else {
      if (lo < start) append(start, value, clear);
      append(Math.min(hi, stop), word, cleared);
      if (hi > stop) append(hi, value, clear);
    }
  });
  const dense = ends.length * RUN >= CELLS * STRIDE;
  const data = new Uint8Array(dense ? CELLS * STRIDE : ends.length * RUN);
  if (dense) {
    let lo = 0;
    for (let k = 0; k < ends.length; k++) {
      for (let c = lo; c < ends[k]; c++) putWord(data, c * STRIDE, words[k], clears[k]);
      lo = ends[k];
    }
  } else
    for (let k = 0; k < ends.length; k++) {
      data[k * RUN] = ends[k];
      data[k * RUN + 1] = ends[k] >>> 8;
      putWord(data, k * RUN + 2, words[k], clears[k]);
    }
  page.data = data;
  page.dense = dense;
  return delta;
}

/** Compress one completed dense tile, keeping edits allocation-free while
 * its publication records are being applied. */
function compressGroundPage(page: GroundPage): void {
  if (!page.dense) return;
  const ends: number[] = [],
    words: number[] = [],
    clears: number[] = [];
  groundPageSpans(page, (_start, end, word, clear) => {
    ends.push(end);
    words.push(word);
    clears.push(clear);
  });
  if (ends.length * RUN >= page.data.byteLength) return;
  const data = new Uint8Array(ends.length * RUN);
  for (let k = 0; k < ends.length; k++) {
    data[k * RUN] = ends[k];
    data[k * RUN + 1] = ends[k] >>> 8;
    putWord(data, k * RUN + 2, words[k], clears[k]);
  }
  page.data = data;
  page.dense = false;
}

/** One reusable edit buffer per receiver, never one temporary dense page
 * per retained tile. Fragmented records are committed together. */
export interface GroundPageEdits {
  starts: Uint16Array;
  lens: Uint16Array;
  words: Uint32Array;
  clears: Uint8Array;
  count: number;
  dense: Uint8Array;
}
export function createGroundPageEdits(): GroundPageEdits {
  return {
    starts: new Uint16Array(CELLS),
    lens: new Uint16Array(CELLS),
    words: new Uint32Array(CELLS),
    clears: new Uint8Array(CELLS),
    count: 0,
    dense: new Uint8Array(CELLS * STRIDE),
  };
}
export function applyGroundPageEdits(page: GroundPage, edit: GroundPageEdits): number {
  if (edit.count === 1) {
    const delta = setGroundSpan(page, edit.starts[0], edit.lens[0], edit.words[0], edit.clears[0]);
    compressGroundPage(page);
    return delta;
  }
  const data = edit.dense;
  if (page.dense) data.set(page.data);
  else
    groundPageSpans(page, (lo, hi, word, clear) => {
      for (let c = lo; c < hi; c++) putWord(data, c * STRIDE, word, clear);
    });
  let delta = 0;
  for (let k = 0; k < edit.count; k++) {
    const start = edit.starts[k],
      stop = start + edit.lens[k],
      word = edit.words[k],
      clear = edit.clears[k];
    for (let c = start; c < stop; c++) {
      delta += Number(clear > 0) - Number(data[c * STRIDE + 4] > 0);
      putWord(data, c * STRIDE, word, clear);
    }
  }
  const packed: GroundPage = { data, dense: true };
  compressGroundPage(packed);
  page.data = packed.dense ? data.slice() : packed.data;
  page.dense = packed.dense;
  return delta;
}

export function groundPageMarked(page: GroundPage): boolean {
  let marked = false;
  groundPageSpans(page, (_lo, _hi, word) => {
    marked ||= word !== 0;
  });
  return marked;
}
