// The ground lab's flat cell debug view: one flat quad per marked ground
// cell the observed side has learned, tinted by its strongest shown channel.
// It reads the side's learned ground (the patches its publications carried,
// as the presentation applied them), never the authoritative layer.
import { MeshBuilder, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";
import type { LearnedGround } from "@web/battle/sim/presentationGround";

/** A learned cell's marks, in the order their bytes are packed. */
export const GROUND_CHANNELS = ["crater", "scorch", "tracks", "trampled"] as const;
export type GroundChannel = (typeof GROUND_CHANNELS)[number];

/** One marked cell: its lower corner and marks in [0, 255]. */
export interface GroundCellView {
  x: number;
  y: number;
  marks: Record<GroundChannel, number>;
}

export interface GroundCells {
  cellM: number;
  cells: GroundCellView[];
}

/** Every marked cell the side has learned. */
export function groundCells(ground: LearnedGround): GroundCells {
  const cells: GroundCellView[] = [];
  const marked = ground.markedCells();
  for (let k = 0; k < marked.length; k += 2) {
    const [index, packed] = [marked[k], marked[k + 1]];
    cells.push({
      x: (index % ground.cols) * ground.cellM,
      y: Math.floor(index / ground.cols) * ground.cellM,
      marks: {
        crater: packed & 255,
        scorch: (packed >>> 8) & 255,
        tracks: (packed >>> 16) & 255,
        trampled: packed >>> 24,
      },
    });
  }
  return { cellM: ground.cellM, cells };
}

/** The side's known craters: how many cells are cratered, and the centre of
 *  the `binM`-square block holding the most (framing a shelled field). */
export function knownCraters(ground: LearnedGround, binM: number) {
  const per = Math.max(1, Math.round(binM / ground.cellM));
  const bins = new Map<number, number>();
  let cells = 0;
  const marked = ground.markedCells();
  for (let k = 0; k < marked.length; k += 2) {
    if ((marked[k + 1] & 255) < 64) continue;
    const [i, j] = [marked[k] % ground.cols, Math.floor(marked[k] / ground.cols)];
    cells++;
    const key = Math.floor(j / per) * ground.cols + Math.floor(i / per);
    bins.set(key, (bins.get(key) ?? 0) + 1);
  }
  let densest: [number, number] | null = null;
  let most = 0;
  for (const [key, n] of bins)
    if (n > most) {
      most = n;
      const [bi, bj] = [key % ground.cols, Math.floor(key / ground.cols)];
      densest = [(bi + 0.5) * per * ground.cellM, (bj + 0.5) * per * ground.cellM];
    }
  return { cells, densest, most };
}

export const CHANNEL_COLORS: Record<GroundChannel, Rgba> = {
  crater: [0.95, 0.2, 0.1, 1],
  scorch: [0.08, 0.06, 0.05, 1],
  tracks: [0.2, 0.55, 1.0, 1],
  trampled: [1.0, 0.85, 0.2, 1],
};
const LIFT_M = 0.2;

/** Flat quads for every cell with a shown channel marked. The first shown
 *  channel in `GROUND_CHANNELS` order wins a cell; its value sets the alpha. */
export function buildGroundCellOverlay(
  ground: GroundCells,
  shown: ReadonlySet<GroundChannel>,
  z: SurfaceHeight,
) {
  const mesh = new MeshBuilder();
  const s = ground.cellM;
  const at = (x: number, y: number): [number, number, number] => [x, y, z(x, y) + LIFT_M];
  for (const cell of ground.cells) {
    const channel = GROUND_CHANNELS.find((c) => shown.has(c) && cell.marks[c] > 0);
    if (!channel) continue;
    const [r, g, b] = CHANNEL_COLORS[channel];
    const color: Rgba = [r, g, b, 0.35 + 0.6 * (cell.marks[channel] / 255)];
    const { x, y } = cell;
    mesh.quad(at(x, y), at(x + s, y), at(x + s, y + s), at(x, y + s), color);
  }
  return mesh.build();
}
