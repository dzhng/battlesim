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
