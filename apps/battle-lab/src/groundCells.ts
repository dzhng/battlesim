// The ground lab's flat cell debug view: one flat quad per marked ground
// cell, tinted by its strongest shown channel. It reads the authoritative
// layer through the worker's lab diagnostic, not a side's observation; slice
// 08's ground patches replace that source.
import { MeshBuilder, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";

export const CHANNELS = ["crater", "scorch", "tracks", "trampled"] as const;
export type Channel = (typeof CHANNELS)[number];

/** One marked cell: its lower corner and marks in [0, 255]. */
export interface GroundCellView {
  x: number;
  y: number;
  marks: Record<Channel, number>;
}

export interface GroundCells {
  cellM: number;
  cells: GroundCellView[];
}

/** Decode the worker's `[cell_m, (x, y, crater, scorch, tracks, trampled)…]`. */
export function decodeGroundCells(flat: Float32Array): GroundCells {
  const cells: GroundCellView[] = [];
  for (let i = 1; i + 6 <= flat.length; i += 6) {
    cells.push({
      x: flat[i],
      y: flat[i + 1],
      marks: {
        crater: flat[i + 2],
        scorch: flat[i + 3],
        tracks: flat[i + 4],
        trampled: flat[i + 5],
      },
    });
  }
  return { cellM: flat[0] ?? 1, cells };
}

export const CHANNEL_COLORS: Record<Channel, Rgba> = {
  crater: [0.95, 0.2, 0.1, 1],
  scorch: [0.08, 0.06, 0.05, 1],
  tracks: [0.2, 0.55, 1.0, 1],
  trampled: [1.0, 0.85, 0.2, 1],
};
const LIFT_M = 0.2;

/** Flat quads for every cell with a shown channel marked. The first shown
 *  channel in `CHANNELS` order wins a cell; its value sets the alpha. */
export function buildGroundCellOverlay(
  ground: GroundCells,
  shown: ReadonlySet<Channel>,
  z: SurfaceHeight,
) {
  const mesh = new MeshBuilder();
  const s = ground.cellM;
  const at = (x: number, y: number): [number, number, number] => [x, y, z(x, y) + LIFT_M];
  for (const cell of ground.cells) {
    const channel = CHANNELS.find((c) => shown.has(c) && cell.marks[c] > 0);
    if (!channel) continue;
    const [r, g, b] = CHANNEL_COLORS[channel];
    const color: Rgba = [r, g, b, 0.35 + 0.6 * (cell.marks[channel] / 255)];
    const { x, y } = cell;
    mesh.quad(at(x, y), at(x + s, y), at(x + s, y + s), at(x, y + s), color);
  }
  return mesh.build();
}
