/** Test helpers for ground patches: write one as a list of cells, count the
 *  cells a run patch covers, and carry one in a publication record. */
import type { ObservationLayout } from "../src/battle/sim/observationView";

/** A patch of the side's learned ground, its runs four numbers each: tile,
 *  start + length × 256, crater + scorch × 256, tracks + trampled × 256 +
 *  cleared × 65536. */
export interface GroundRunsPatch {
  epoch: number;
  side: string;
  baseRevision: number;
  revision: number;
  full: boolean;
  runs: Float32Array;
}
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

/** Publication records in `layout`'s shape carrying only a ground patch
 *  (empty groups, a 9 × 5 fog), for a decoder to learn it. */
export function groundRecords(layout: ObservationLayout) {
  let fog = 0;
  return (patch: GroundRunsPatch): Float32Array => {
    const base = patch.full ? 0 : fog;
    fog = base + 1;
    const header: Record<string, number> = {
      tick: patch.revision,
      fogCellM: 8,
      fogNx: 9,
      fogNy: 5,
      fogFloats: patch.full ? 4 : 3,
      fogFull: Number(patch.full),
      fogBaseLo: base,
      fogRevisionLo: fog,
      groundEpoch: patch.epoch,
      groundSide: layout.ground.sides.indexOf(patch.side),
      groundFull: Number(patch.full),
      groundBase: patch.baseRevision,
      groundRevision: patch.revision,
      groundRunCount: patch.runs.length / 4,
    };
    const head = Float32Array.from([
      ...layout.header.map((name) => header[name] ?? 0),
      ...layout.groups.flatMap(() => [0, 1, 0]),
      ...(patch.full ? [65535, 65535, 8191, 0] : [0, 0, 0]),
    ]);
    const tail = packedGroundRuns(Array.from(patch.runs));
    const out = new Float32Array(head.length + tail.length);
    out.set(head);
    new Uint32Array(out.buffer).set(new Uint32Array(tail.buffer), head.length);
    return out;
  };
}
