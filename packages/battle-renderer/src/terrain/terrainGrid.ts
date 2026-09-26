// The simulation's terrain as a height grid, read from its exported vertices:
// vertex (i, j) at (i·spacing, j·spacing), row-major. `groundHeight` is the
// simulation's triangle rule (the south-west → north-east diagonal of every
// cell), the same rule fog's GPU march (`fogHeight`) and `WorldView` use, so
// anything seated with it stands exactly on the drawn triangles.
import type { WorldExports } from "../worldMesh";

export interface TerrainGrid {
  /** Vertex heights, row-major. */
  heights: Float32Array;
  nx: number;
  ny: number;
  spacing: number;
}

export function terrainGrid(exports: Pick<WorldExports, "positions">): TerrainGrid {
  const p = exports.positions;
  const count = p.length / 3;
  let nx = 1;
  while (nx < count && p[nx * 3 + 1] === p[1]) nx++;
  const ny = count / nx;
  if (!Number.isInteger(ny) || nx < 2 || ny < 2)
    throw new Error("terrain: the export is not a row-major vertex grid");
  const heights = new Float32Array(count);
  for (let i = 0; i < count; i++) heights[i] = p[i * 3 + 2];
  return { heights, nx, ny, spacing: p[3] - p[0] };
}

/** Ground height at (x, y), clamped to the grid's edge. */
export function groundHeight(grid: TerrainGrid, x: number, y: number): number {
  const { heights, nx, ny, spacing } = grid;
  const fx = Math.min(Math.max(x / spacing, 0), nx - 1);
  const fy = Math.min(Math.max(y / spacing, 0), ny - 1);
  const i = Math.min(Math.floor(fx), nx - 2);
  const j = Math.min(Math.floor(fy), ny - 2);
  const u = fx - i;
  const v = fy - j;
  const h00 = heights[j * nx + i];
  const h11 = heights[(j + 1) * nx + i + 1];
  if (u >= v) {
    const h10 = heights[j * nx + i + 1];
    return h00 + u * (h10 - h00) + v * (h11 - h10);
  }
  const h01 = heights[(j + 1) * nx + i];
  return h00 + v * (h01 - h00) + u * (h11 - h01);
}
