// The ground lab's flat cell debug view: one flat quad per marked ground
// cell the observed side has learned, tinted by its strongest shown channel.
// It reads the side's `GroundView` (the patches its publications carried),
// never the authoritative layer.
import { MeshBuilder, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";
import { GROUND_CHANNELS, type GroundChannel, type GroundView } from "@web/battle/sim/ground";

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

/** Every marked cell the view holds. */
export function groundCells(view: GroundView): GroundCells {
  const cells: GroundCellView[] = [];
  view.forEachMarked((i, j) =>
    cells.push({ x: i * view.cellM, y: j * view.cellM, marks: view.cell(i, j) }),
  );
  return { cellM: view.cellM, cells };
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
