/** Test helpers for ground patches: write one as a list of cells, and count
 *  the cells a run patch covers. */
import type { GroundRunsPatch } from "../src/battle/sim/ground";
export interface CellPatch {
  epoch: number;
  side: string;
  baseRevision: number;
  revision: number;
  full: boolean;
  cells: Uint32Array;
  marks: Uint8Array;
  cleared: Uint8Array;
}
export function cellPatchRuns(cols: number, p: CellPatch): GroundRunsPatch {
  const tilesX = Math.ceil(cols / 16);
  const cells = Array.from(p.cells, (index, n) => {
    const i = index % cols,
      j = Math.floor(index / cols);
    return {
      tile: Math.floor(j / 16) * tilesX + Math.floor(i / 16),
      cell: (j % 16) * 16 + (i % 16),
      a: p.marks[n * 4] | (p.marks[n * 4 + 1] << 8),
      b: p.marks[n * 4 + 2] | (p.marks[n * 4 + 3] << 8) | (p.cleared[n] << 16),
    };
  }).sort((a, b) => a.tile - b.tile || a.cell - b.cell);
  const runs: number[] = [];
  for (const c of cells) {
    const at = runs.length - 4,
      span = runs[at + 1];
    if (
      at >= 0 &&
      runs[at] === c.tile &&
      (span % 256) + Math.floor(span / 256) === c.cell &&
      runs[at + 2] === c.a &&
      runs[at + 3] === c.b
    )
      runs[at + 1] += 256;
    else runs.push(c.tile, c.cell + 256, c.a, c.b);
  }
  return {
    epoch: p.epoch,
    side: p.side,
    baseRevision: p.baseRevision,
    revision: p.revision,
    full: p.full,
    runs: Float32Array.from(runs),
  };
}
export function groundRunCells(p: GroundRunsPatch): number {
  let count = 0;
  for (let k = 1; k < p.runs.length; k += 4) count += Math.floor(p.runs[k] / 256);
  return count;
}

/** Independent raw-bit fixture writer for the layout's final ground tail. */
export function packedGroundRuns(runs: readonly number[]): Float32Array {
  let bits = 0n,
    count = 0n,
    prior = 0;
  const put = (value: number, width: number) => {
    bits |= BigInt(value) << count;
    count += BigInt(width);
  };
  for (let row = 0; row < runs.length; row += 4) {
    const tile = runs[row],
      span = runs[row + 1],
      a = runs[row + 2],
      b = runs[row + 3];
    let delta = tile - prior;
    do {
      const byte = delta & 127;
      delta >>>= 7;
      put(byte | (delta ? 128 : 0), 8);
    } while (delta);
    put(span % 256, 8);
    put(Math.floor(span / 256) - 1, 8);
    const marks = [a & 255, a >>> 8, b & 255, (b >>> 8) & 255, b >>> 16];
    put(
      marks.reduce((mask, value, i) => mask | (Number(value !== 0) << i), 0),
      5,
    );
    for (const value of marks) if (value) put(value, 8);
    prior = tile;
  }
  const words = new Uint32Array(Math.ceil(Number(count) / 32));
  for (let i = 0; i < words.length; i++) words[i] = Number((bits >> BigInt(i * 32)) & 0xffffffffn);
  return new Float32Array(words.buffer);
}
